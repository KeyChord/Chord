import { test } from "bun:test";
import { mkdirSync } from "node:fs";
import { join } from "node:path";
import { appPid, approvePackages, type ChordApp, launch, quit } from "#chord-app";
import { Guest, shellQuote } from "#guest";
import { Keysym } from "#keysyms";
import { SESSION_ENV, type SessionState } from "#vm";
import { VncClient } from "#vnc";
import { waitFor, type WaitOptions } from "#wait";

/** Files of a chord package, by path relative to the package root. */
export type PackageFiles = Record<string, string>;

/** Answers `PROBE_SEQUENCE` once Chord has loaded personal packages. */
const PROBE_PACKAGE = "e2e-probe";
const PROBE_SEQUENCE = ";;e2e-ready";
const PROBE_FILE = "/tmp/chord-e2e-ready";

export interface ChordInputOptions {
  /** How the chord runs: by releasing Space (default) or by pressing Caps while Space is held. */
  complete?: "release" | "caps";
}

/**
 * A test file's connection to the VM prepared by `_e2e-macos test`: SSH for setup and
 * assertions, and VNC for keyboard input that reaches the guest as real HID events.
 */
export class Session {
  private constructor(
    readonly guest: Guest,
    readonly vnc: VncClient,
    readonly app: ChordApp,
    readonly artifacts: string,
  ) {}

  static async open(): Promise<Session> {
    const file = process.env[SESSION_ENV];
    if (!file) {
      throw new Error(`${SESSION_ENV} is not set; run the e2e tests with \`vpx _e2e-macos test\``);
    }
    const state: SessionState = await Bun.file(file).json();
    mkdirSync(state.artifacts, { recursive: true });
    const [guest, vnc] = await Promise.all([
      Guest.connect(state.host),
      VncClient.connect(state.vncUrl),
    ]);
    return new Session(guest, vnc, state.app, state.artifacts);
  }

  /**
   * Types a chord as a user does: Caps+Space enters Chord mode, Caps is let go while Space
   * stays down, the keys are typed, and the chord runs.
   */
  async chord(sequence: string, { complete = "release" }: ChordInputOptions = {}) {
    this.vnc.key(Keysym.CapsLock, true);
    await Bun.sleep(40);
    this.vnc.key(Keysym.Space, true);
    await Bun.sleep(40);
    this.vnc.key(Keysym.CapsLock, false);
    await Bun.sleep(80);
    await this.vnc.type(sequence);
    if (complete === "caps") await this.vnc.press(Keysym.CapsLock);
    this.vnc.key(Keysym.Space, false);
    await Bun.sleep(80);
  }

  /** Runs `sequence` through the app's CLI, which forwards it to the running app. */
  async cliChord(sequence: string) {
    await this.guest.run(`${shellQuote(this.app.executable)} chord ${shellQuote(sequence)}`);
  }

  /**
   * Quits Chord, makes `packages` its only personal packages (`~/.chord/packages/<name>`)
   * besides the starter `default` one, and launches it again, returning once it owns
   * input and has loaded them.
   */
  async restartChord(packages: Record<string, PackageFiles> = {}) {
    await quit(this.guest, this.app);
    const root = `${this.guest.home}/.chord/packages`;
    await this.guest.run(
      `mkdir -p ${shellQuote(root)} && find ${shellQuote(root)} -mindepth 1 -maxdepth 1 ! -name '.*' ! -name '@keychord' ! -name default -exec rm -rf {} +`,
    );
    const all: Record<string, PackageFiles> = {
      ...packages,
      [PROBE_PACKAGE]: {
        "chords/macos.toml": `name = "${PROBE_PACKAGE}"\n\n[chords]\n"${PROBE_SEQUENCE}" = { name = "E2E: ready", shell = "echo ready > ${PROBE_FILE}" }\n`,
      },
    };
    for (const [name, files] of Object.entries(all)) {
      for (const [path, contents] of Object.entries(files)) {
        await this.guest.writeFile(`${root}/${name}/${path}`, contents);
      }
    }
    await approvePackages(
      this.guest,
      this.app,
      Object.keys(all).map((name) => `${root}/${name}`),
    );
    await launch(this.guest, this.app);
    await this.guest.run(`rm -f ${PROBE_FILE}`);
    await waitFor(
      "Chord to load the test packages",
      async () => {
        await this.cliChord(PROBE_SEQUENCE);
        const ready = await waitFor("the probe chord", () => this.fileContents(PROBE_FILE), {
          timeoutMs: 3_000,
        }).catch(() => undefined);
        return ready === undefined ? undefined : true;
      },
      { timeoutMs: 60_000, intervalMs: 500 },
    );
  }

  /**
   * Opens a Terminal window running `cat > file`, a focused app that records exactly
   * the keystrokes it receives. Lines reach the file when Return is typed.
   */
  async openRecorder(file: string): Promise<{ close(): Promise<void> }> {
    const script = `${file}.command`;
    await this.guest.writeFile(script, `#!/bin/sh\nexec cat > ${shellQuote(file)}\n`, {
      mode: 0o755,
    });
    await this.guest.run(`rm -f ${shellQuote(file)} && open -a Terminal ${shellQuote(script)}`);
    await waitFor("Terminal to run the recorder", async () =>
      (await this.guest.run(`test -e ${shellQuote(file)}`, { check: false })).exitCode === 0
        ? true
        : undefined,
    );
    // Terminal activates as the window opens; give it time to become key.
    await Bun.sleep(1_000);
    return {
      close: async () => {
        await this.guest.run("killall Terminal", { check: false });
      },
    };
  }

  async pid(): Promise<number | undefined> {
    return await appPid(this.guest, this.app);
  }

  /** The file's contents once it exists. */
  async waitForFile(path: string, options: WaitOptions = {}): Promise<string> {
    return await waitFor(path, () => this.fileContents(path), { timeoutMs: 15_000, ...options });
  }

  /** Saves the screen to the artifacts folder and returns the PNG's path. */
  async screenshot(name: string): Promise<string> {
    const path = join(this.artifacts, `${name}.png`);
    await Bun.write(path, await this.vnc.screenshot());
    return path;
  }

  /**
   * Like `test`, but saves a screenshot and Chord's log when the test fails. Keys left
   * down by a failed test are released so they cannot leak into the next one.
   */
  test(name: string, body: () => Promise<void>, timeoutMs = 120_000) {
    test(
      name,
      async () => {
        try {
          await body();
        } catch (error) {
          const slug = name
            .toLowerCase()
            .replace(/[^a-z0-9]+/g, "-")
            .replace(/^-|-$/g, "");
          for (const key of [Keysym.Space, Keysym.CapsLock, Keysym.Shift]) this.vnc.key(key, false);
          const log = join(this.artifacts, `${slug}.chords.log`);
          const saved = await Promise.allSettled([
            this.screenshot(slug),
            this.guest.copyFrom(this.app.log, log).then(() => log),
          ]);
          const paths = saved.flatMap((result) =>
            result.status === "fulfilled" ? [result.value] : [],
          );
          if (error instanceof Error)
            error.message += `\nartifacts: ${[...paths, this.artifacts].join(", ")}`;
          throw error;
        }
      },
      timeoutMs,
    );
  }

  async close() {
    this.vnc.close();
    await this.guest.close();
  }

  private async fileContents(path: string): Promise<string | undefined> {
    const contents = await this.guest.readFile(path);
    return contents === undefined || contents === "" ? undefined : contents;
  }
}
