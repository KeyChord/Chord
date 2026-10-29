import { chmodSync, mkdtempSync, rmSync } from "node:fs";
import { join } from "node:path";
import { waitFor, withTimeout } from "#wait";

/** Quotes `value` as one POSIX shell word. */
export function shellQuote(value: string): string {
  return `'${value.replaceAll("'", `'\\''`)}'`;
}

export interface RunOptions {
  /** Reject on a nonzero exit status (default). */
  check?: boolean;
  timeoutMs?: number;
  stdin?: string | Uint8Array;
}

export interface RunResult {
  stdout: string;
  stderr: string;
  exitCode: number;
}

export interface OutputOptions {
  /** Every pattern must appear in the output. */
  until: RegExp[];
  /** Any of these patterns fails the wait immediately. */
  failOn?: RegExp[];
  timeoutMs?: number;
}

/**
 * SSH access to the guest's GUI user. Cirrus Labs images log `admin` in automatically and
 * accept `admin`/`admin`; the password is supplied through `SSH_ASKPASS`, so no key
 * setup or `sshpass` is needed. Connections share one multiplexed master.
 */
export class Guest {
  private constructor(
    readonly host: string,
    readonly user: string,
    private readonly directory: string,
  ) {}

  static async connect(host: string, { user = "admin", password = "admin" } = {}): Promise<Guest> {
    // ControlPath must fit in a 104-byte socket address, which rules out $TMPDIR.
    const directory = mkdtempSync("/tmp/chord-e2e-ssh-");
    const askpass = join(directory, "askpass");
    await Bun.write(askpass, `#!/bin/sh\nprintf '%s\\n' ${shellQuote(password)}\n`);
    chmodSync(askpass, 0o700);
    return new Guest(host, user, directory);
  }

  /** Home directory of the guest user. */
  get home(): string {
    return `/Users/${this.user}`;
  }

  /** Runs `script` with `bash -euo pipefail` in the guest. */
  async run(
    script: string,
    { check = true, timeoutMs = 120_000, stdin }: RunOptions = {},
  ): Promise<RunResult> {
    const child = Bun.spawn(this.ssh(script), {
      env: this.env(),
      stdin:
        stdin === undefined
          ? "ignore"
          : typeof stdin === "string"
            ? new TextEncoder().encode(stdin)
            : stdin,
      stdout: "pipe",
      stderr: "pipe",
      timeout: timeoutMs,
    });
    const [stdout, stderr, exitCode] = await Promise.all([
      new Response(child.stdout).text(),
      new Response(child.stderr).text(),
      child.exited,
    ]);
    if (check && exitCode !== 0) {
      const detail = (stderr.trim() || stdout.trim()).slice(-4000);
      throw new Error(`guest command failed (${exitCode}): ${script.trim()}\n${detail}`);
    }
    return { stdout, stderr, exitCode };
  }

  /** Runs `script` and resolves with its output once every `until` pattern has appeared. */
  async waitForOutput(script: string, { until, failOn = [], timeoutMs = 60_000 }: OutputOptions) {
    const child = Bun.spawn(this.ssh(script), {
      env: this.env(),
      stdin: "ignore",
      stdout: "pipe",
      stderr: "pipe",
    });
    let output = "";
    try {
      return await withTimeout(
        (async () => {
          const decoder = new TextDecoder();
          for await (const chunk of child.stdout) {
            output += decoder.decode(chunk, { stream: true });
            const failure = failOn.find((pattern) => pattern.test(output));
            if (failure)
              throw new Error(`guest output matched ${failure}:\n${output.slice(-4000)}`);
            if (until.every((pattern) => pattern.test(output))) return output;
          }
          const stderr = await new Response(child.stderr).text();
          throw new Error(
            `\`${script}\` ended before printing ${until.join(", ")}:\n${output}${stderr}`,
          );
        })(),
        timeoutMs,
        `waiting for ${until.join(", ")} in \`${script}\``,
      );
    } catch (error) {
      if (error instanceof Error && !error.message.includes(output.slice(-4000))) {
        error.message += `\noutput so far:\n${output.slice(-4000)}`;
      }
      throw error;
    } finally {
      child.kill();
    }
  }

  async copyTo(local: string, remote: string) {
    await this.scp([local, `${this.user}@${this.host}:${remote}`]);
  }

  async copyFrom(remote: string, local: string) {
    await this.scp([`${this.user}@${this.host}:${remote}`, local]);
  }

  async writeFile(remote: string, contents: string | Uint8Array, { mode }: { mode?: number } = {}) {
    const path = shellQuote(remote);
    const chmod = mode === undefined ? "" : ` && chmod ${mode.toString(8)} ${path}`;
    await this.run(`mkdir -p "$(dirname ${path})" && cat > ${path}${chmod}`, { stdin: contents });
  }

  /** The file's contents, or `undefined` when it does not exist. */
  async readFile(remote: string): Promise<string | undefined> {
    const path = shellQuote(remote);
    const result = await this.run(`if [ -e ${path} ]; then cat ${path}; else exit 3; fi`, {
      check: false,
    });
    if (result.exitCode === 3) return undefined;
    if (result.exitCode !== 0) throw new Error(`cannot read ${remote}: ${result.stderr.trim()}`);
    return result.stdout;
  }

  /** Waits until SSH accepts connections and the GUI session is up. */
  async waitUntilReady({ timeoutMs = 300_000 } = {}) {
    await waitFor(
      `SSH and a logged-in desktop on ${this.host}`,
      async () => {
        const result = await this.run('pgrep -qxu "$(id -u)" Dock', {
          check: false,
          timeoutMs: 15_000,
        });
        return result.exitCode === 0 ? true : undefined;
      },
      { timeoutMs, intervalMs: 2_000 },
    );
  }

  /** Opens an interactive shell, or runs `command` with a terminal attached. */
  async shell(command?: string): Promise<number> {
    const child = Bun.spawn(
      ["ssh", "-t", ...this.options(), `${this.user}@${this.host}`, ...(command ? [command] : [])],
      { env: this.env(), stdio: ["inherit", "inherit", "inherit"] },
    );
    return await child.exited;
  }

  /** Closes the shared SSH connection. */
  async close() {
    const control = Bun.spawn(
      ["ssh", ...this.options(), "-O", "exit", `${this.user}@${this.host}`],
      {
        env: this.env(),
        stdout: "ignore",
        stderr: "ignore",
      },
    );
    await control.exited;
    rmSync(this.directory, { recursive: true, force: true });
  }

  private ssh(script: string): string[] {
    return [
      "ssh",
      ...this.options(),
      `${this.user}@${this.host}`,
      `/bin/bash -euo pipefail -c ${shellQuote(script)}`,
    ];
  }

  private async scp(args: string[]) {
    const child = Bun.spawn(["scp", "-q", ...this.options(), ...args], {
      env: this.env(),
      stdout: "pipe",
      stderr: "pipe",
      timeout: 600_000,
    });
    const [stderr, status] = await Promise.all([new Response(child.stderr).text(), child.exited]);
    if (status !== 0) throw new Error(`scp ${args.join(" ")} failed (${status}): ${stderr.trim()}`);
  }

  private options(): string[] {
    return [
      "-o",
      "StrictHostKeyChecking=no",
      "-o",
      "UserKnownHostsFile=/dev/null",
      "-o",
      "LogLevel=ERROR",
      "-o",
      "ConnectTimeout=10",
      "-o",
      "PubkeyAuthentication=no",
      "-o",
      "PreferredAuthentications=password,keyboard-interactive",
      "-o",
      "NumberOfPasswordPrompts=1",
      "-o",
      "ControlMaster=auto",
      "-o",
      `ControlPath=${this.directory}/%C`,
      "-o",
      "ControlPersist=60",
      "-o",
      "ServerAliveInterval=5",
    ];
  }

  private env(): Record<string, string | undefined> {
    return {
      ...process.env,
      SSH_ASKPASS: join(this.directory, "askpass"),
      SSH_ASKPASS_REQUIRE: "force",
    };
  }
}
