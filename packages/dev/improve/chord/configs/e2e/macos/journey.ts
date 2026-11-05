import { test } from "bun:test";
import { mkdirSync } from "node:fs";
import { join } from "node:path";
import { Guest } from "#guest";
import { Screen } from "#screen";
import { VncClient } from "#vnc";
import { waitFor, type WaitOptions } from "#wait";

/** Names the JSON file with the `JourneyState` of the VM. */
export const JOURNEY_ENV = "CHORD_E2E_JOURNEY";

export interface JourneyState {
  host: string;
  vncUrl: string;
  /** Where the VM visits the website. */
  siteUrl: string;
  /** The download keeps its quarantine mark. Without notarization, Gatekeeper would refuse it. */
  checkedByGatekeeper: boolean;
  /** Downloads the Intel build, which the VM runs under Rosetta. */
  intel: boolean;
  artifacts: string;
}

interface Row {
  name: string;
  seconds: number;
  screenshot?: string;
  error?: string;
}

/**
 * A first run as a sequence of steps. Each one leaves a screenshot and a line in `journey.md`,
 * so a run can be read like a story, and tells how long a person spends on it. A failed step
 * fails those after it: they would be looking at the wrong screen.
 */
export class Journey {
  private rows: Row[] = [];
  private failed: string | undefined;

  private constructor(
    readonly guest: Guest,
    readonly vnc: VncClient,
    readonly screen: Screen,
    readonly siteUrl: string,
    readonly checkedByGatekeeper: boolean,
    readonly intel: boolean,
    private readonly artifacts: string,
  ) {}

  static async open(): Promise<Journey> {
    const file = process.env[JOURNEY_ENV];
    if (!file) throw new Error(`${JOURNEY_ENV} is not set; run with \`vpx _e2e-macos journey\``);
    const state: JourneyState = await Bun.file(file).json();
    mkdirSync(state.artifacts, { recursive: true });
    const [guest, vnc] = await Promise.all([
      Guest.connect(state.host),
      VncClient.connect(state.vncUrl),
    ]);
    const screen = new Screen(vnc, join(state.artifacts, "journey"));
    return new Journey(
      guest,
      vnc,
      screen,
      state.siteUrl,
      state.checkedByGatekeeper,
      state.intel,
      state.artifacts,
    );
  }

  step(name: string, body: () => Promise<void>, timeoutMs = 240_000) {
    test(
      name,
      async () => {
        if (this.failed) throw new Error(`skipped after "${this.failed}" failed`);
        const started = Date.now();
        const row: Row = { name, seconds: 0 };
        this.rows.push(row);
        try {
          await body();
          row.screenshot = await this.screen.capture(name).catch(() => undefined);
        } catch (error) {
          this.failed = name;
          row.error = error instanceof Error ? error.message : String(error);
          row.screenshot = await this.screen.capture(`failed ${name}`).catch(() => undefined);
          if (error instanceof Error && row.screenshot) {
            error.message += `\nscreen: ${row.screenshot}`;
          }
          throw error;
        } finally {
          row.seconds = Math.round((Date.now() - started) / 1000);
          await this.writeReport();
        }
      },
      timeoutMs,
    );
  }

  async waitUntil(what: string, check: () => Promise<boolean>, options: WaitOptions = {}) {
    await waitFor(what, async () => ((await check()) ? true : undefined), {
      timeoutMs: 60_000,
      intervalMs: 500,
      ...options,
    });
  }

  async waitForFile(path: string, options: WaitOptions = {}) {
    await this.waitUntil(
      path,
      async () => (await this.guest.run(`test -e '${path}'`, { check: false })).exitCode === 0,
      { timeoutMs: 120_000, ...options },
    );
  }

  private async writeReport() {
    const total = this.rows.reduce((sum, row) => sum + row.seconds, 0);
    const lines = [
      `# First run through ${this.siteUrl}`,
      "",
      `${this.checkedByGatekeeper ? "Checked by Gatekeeper." : "Not notarized, so without Gatekeeper's check."} ${total} s so far.`,
      "",
      "| Step | Seconds | Result |",
      "| --- | --- | --- |",
      ...this.rows.map(
        (row) =>
          `| ${row.name} | ${row.seconds} | ${row.error ? `failed: ${row.error.split("\n")[0]}` : "ok"} |`,
      ),
      "",
      ...this.rows.flatMap((row) =>
        row.screenshot ? [`## ${row.name}`, "", `![${row.name}](${row.screenshot})`, ""] : [],
      ),
    ];
    await Bun.write(join(this.artifacts, "journey.md"), lines.join("\n"));
  }

  async close() {
    await this.vnc.close();
    await this.guest.close();
  }
}
