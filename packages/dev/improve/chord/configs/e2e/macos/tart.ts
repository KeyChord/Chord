import { closeSync, existsSync, fstatSync, mkdirSync, openSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join } from "node:path";
import { waitFor } from "#wait";

const INSTALL_LOCATIONS = [
  join(homedir(), "Applications/tart.app/Contents/MacOS/tart"),
  "/Applications/tart.app/Contents/MacOS/tart",
];

let binary: string | undefined;

/** `$TART`, `tart` on `PATH`, or `tart.app` in an Applications folder. */
export function tartBinary(): string {
  binary ??=
    process.env.TART ?? Bun.which("tart") ?? INSTALL_LOCATIONS.find((path) => existsSync(path));
  if (!binary) {
    throw new Error(
      "tart is not installed. Install it with `brew install cirruslabs/cli/tart`, or unpack " +
        "https://github.com/cirruslabs/tart/releases/latest/download/tart.tar.gz into ~/Applications.",
    );
  }
  return binary;
}

export async function tart(
  args: string[],
  { timeoutMs = 600_000, inheritOutput = false } = {},
): Promise<string> {
  const child = Bun.spawn([tartBinary(), ...args], {
    stdout: inheritOutput ? "inherit" : "pipe",
    stderr: inheritOutput ? "inherit" : "pipe",
    timeout: timeoutMs,
  });
  const [stdout, stderr, status] = await Promise.all([
    child.stdout instanceof ReadableStream ? new Response(child.stdout).text() : "",
    child.stderr instanceof ReadableStream ? new Response(child.stderr).text() : "",
    child.exited,
  ]);
  if (status !== 0) {
    throw new Error(`tart ${args.join(" ")} failed (${status}): ${(stderr || stdout).trim()}`);
  }
  return stdout;
}

export interface VmInfo {
  name: string;
  source: string;
  running: boolean;
}

export async function listVms(): Promise<VmInfo[]> {
  const vms: { Name: string; Source: string; Running: boolean }[] = JSON.parse(
    await tart(["list", "--source", "local", "--format", "json"]),
  );
  return vms.map((vm) => ({ name: vm.Name, source: vm.Source, running: vm.Running }));
}

export async function findVm(name: string): Promise<VmInfo | undefined> {
  return (await listVms()).find((vm) => vm.name === name);
}

export interface RunOptions {
  /** Boot into macOS Recovery. */
  recovery?: boolean;
  /** Receives tart's stdout and stderr. */
  logFile: string;
  /** Keep the VM running after this process exits. */
  detached?: boolean;
}

export interface RunningVm {
  name: string;
  /** `vnc://:password@127.0.0.1:port`, for Virtualization.framework's VNC server. */
  vncUrl: string;
  /** Resolves with tart's exit status; never resolves for a detached VM. */
  exited: Promise<number>;
}

/**
 * Boots `name` headless with Virtualization.framework's VNC server, which, unlike macOS
 * Screen Sharing, drives the VM's virtual keyboard and also works in Recovery.
 */
export async function runVm(name: string, options: RunOptions): Promise<RunningVm> {
  const args = ["run", "--no-graphics", "--vnc-experimental"];
  if (options.recovery) args.push("--recovery");
  mkdirSync(dirname(options.logFile), { recursive: true });
  const log = openSync(options.logFile, "a");
  // The log may hold earlier runs; only this run's output names its VNC server.
  const start = fstatSync(log).size;
  const child = Bun.spawn([tartBinary(), ...args, name], {
    stdin: "ignore",
    stdout: log,
    stderr: log,
    detached: options.detached,
  });
  closeSync(log);
  if (options.detached) child.unref();

  let exitStatus: number | undefined;
  const exited = child.exited.then((status) => (exitStatus = status));
  const vncUrl = await waitFor(
    `the VNC server of ${name}`,
    async () => {
      const output = await Bun.file(options.logFile).slice(start).text();
      const url = /VNC server is running at (vnc:\/\/\S+)/.exec(output)?.[1];
      if (url) return url;
      if (exitStatus !== undefined) {
        throw new Error(`tart run ${name} exited with ${exitStatus}:\n${output.trim()}`);
      }
      return undefined;
    },
    { timeoutMs: 120_000, intervalMs: 100 },
  );
  return { name, vncUrl, exited: options.detached ? new Promise(() => {}) : exited };
}

export async function vmIp(name: string, { waitSeconds = 180 } = {}): Promise<string> {
  return (await tart(["ip", name, "--wait", String(waitSeconds)])).trim();
}

/** Stops `name` and waits until tart reports it stopped. */
export async function stopVm(name: string, { timeoutSeconds = 60 } = {}) {
  if (!(await findVm(name))?.running) return;
  await tart(["stop", name, "--timeout", String(timeoutSeconds)]);
  await waitFor(`${name} to stop`, async () => ((await findVm(name))?.running ? undefined : true), {
    timeoutMs: 30_000,
  });
}

export async function deleteVm(name: string) {
  if (!(await findVm(name))) return;
  await stopVm(name);
  await tart(["delete", name]);
}
