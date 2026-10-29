import { mkdirSync, readdirSync, rmSync } from "node:fs";
import { join } from "node:path";
import type { ChordApp } from "#chord-app";
import { Guest } from "#guest";
import { deleteVm, findVm, runVm, tart, vmIp } from "#tart";

/** Everything a test process needs to reach the VM; written by the CLI as JSON. */
export interface SessionState {
  vm: string;
  host: string;
  vncUrl: string;
  app: ChordApp;
  /** Screenshots and logs of failed tests go here. */
  artifacts: string;
}

/** Names the JSON file with the `SessionState` of the VM under test. */
export const SESSION_ENV = "CHORD_E2E_SESSION";

export const REPO_ROOT = join(import.meta.dir, "../../../../../../..");
export const E2E_DIR = join(REPO_ROOT, "target/e2e");
/** `SessionState`s of VMs left running by `_e2e-macos up`, by VM name. */
const RUNNING_DIR = join(E2E_DIR, "running");

export interface BootedVm {
  name: string;
  host: string;
  vncUrl: string;
  guest: Guest;
  /** Resolves when tart exits; never resolves for a detached VM. */
  exited: Promise<number>;
}

/** Clones `golden` to `name`, boots it headless, and waits for the desktop session. */
export async function bootClone(
  golden: string,
  name: string,
  { logFile, detached = false }: { logFile: string; detached?: boolean },
): Promise<BootedVm> {
  if (!(await findVm(golden))) {
    throw new Error(`no golden VM ${golden}; create it with \`vpx _e2e-macos setup\``);
  }
  await tart(["clone", golden, name]);
  let guest: Guest | undefined;
  try {
    const { vncUrl, exited } = await runVm(name, { logFile, detached });
    guest = await Guest.connect(await vmIp(name));
    await guest.waitUntilReady();
    return { name, host: guest.host, vncUrl, guest, exited };
  } catch (error) {
    await guest?.close();
    await deleteVm(name);
    throw error;
  }
}

export async function saveRunning(state: SessionState) {
  mkdirSync(RUNNING_DIR, { recursive: true });
  await Bun.write(join(RUNNING_DIR, `${state.vm}.json`), JSON.stringify(state, null, 2));
}

export function forgetRunning(name: string) {
  rmSync(join(RUNNING_DIR, `${name}.json`), { force: true });
}

/** The state of a VM started with `up`: `name`, or the only one running. */
export async function loadRunning(name?: string): Promise<SessionState & { file: string }> {
  let names: string[] = [];
  try {
    names = readdirSync(RUNNING_DIR).map((file) => file.replace(/\.json$/, ""));
  } catch {}
  const chosen = name ?? (names.length === 1 ? names[0] : undefined);
  if (!chosen) {
    throw new Error(
      names.length === 0
        ? "no VM is running; start one with `vpx _e2e-macos up`"
        : `several VMs are running (${names.join(", ")}); pass one with --vm`,
    );
  }
  const file = join(RUNNING_DIR, `${chosen}.json`);
  if (!(await Bun.file(file).exists())) throw new Error(`no running VM named ${chosen}`);
  if (!(await findVm(chosen))?.running) {
    forgetRunning(chosen);
    throw new Error(`${chosen} is no longer running`);
  }
  return { ...(await Bun.file(file).json()), file };
}
