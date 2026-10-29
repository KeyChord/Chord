import { mkdirSync } from "node:fs";
import { join } from "node:path";
import { Guest } from "#guest";
import { Keysym } from "#keysyms";
import { deleteVm, findVm, runVm, tart, vmIp } from "#tart";
import { VncClient } from "#vnc";
import { withTimeout } from "#wait";

/** macOS 27 with auto-login, SSH and passwordless sudo for `admin`/`admin`; SIP is on. */
export const DEFAULT_IMAGE = "ghcr.io/cirruslabs/macos-golden-gate-vanilla:latest";
export const GOLDEN_VM = "chord-e2e-golden";

export interface GoldenOptions {
  image: string;
  name: string;
  concurrency?: number;
  /** Screenshots and tart output go here. */
  logDir: string;
  log: (message: string) => void;
}

/**
 * Builds the VM that every test run clones: the Cirrus Labs image with System Integrity
 * Protection disabled, so tests can write the TCC database to grant Accessibility.
 */
export async function createGolden({ image, name, concurrency = 4, logDir, log }: GoldenOptions) {
  mkdirSync(logDir, { recursive: true });
  const staging = `${name}-staging`;
  await deleteVm(staging);
  log(`cloning ${image} (the first pull downloads about 30 GB)`);
  await tart(["clone", image, staging, "--concurrency", String(concurrency)], {
    timeoutMs: 24 * 60 * 60_000,
    inheritOutput: true,
  });
  log("disabling SIP in macOS Recovery");
  await disableSip(staging, logDir, log);
  log("verifying SIP and provisioning");
  await provision(staging, logDir);
  await deleteVm(name);
  await tart(["rename", staging, name]);
  log(`created ${name}`);
}

/**
 * Types `csrutil disable` into Recovery's Terminal, following cirruslabs/macos-image-templates'
 * disable-sip template. Recovery has no SSH, so the steps are timed; a screenshot after
 * each one shows where it went wrong.
 */
async function disableSip(vm: string, logDir: string, log: (message: string) => void) {
  const run = await runVm(vm, { recovery: true, logFile: join(logDir, "recovery.log") });
  const vnc = await VncClient.connect(run.vncUrl);
  let index = 0;
  const step = async (label: string, waitMs: number, action: () => Promise<void>) => {
    await Bun.sleep(waitMs);
    await action();
    await Bun.sleep(2_000);
    const path = join(logDir, `recovery-${++index}-${label}.png`);
    await Bun.write(path, await vnc.screenshot());
    log(`  ${label} (${path})`);
  };
  try {
    // Startup options: move from "Macintosh HD" to "Options".
    await step("options", 60_000, async () => {
      await vnc.press(Keysym.Right);
      await vnc.press(Keysym.Right);
      await vnc.press(Keysym.Return);
    });
    await step("terminal", 30_000, () =>
      vnc.combo([Keysym.Command, Keysym.Shift, "t".charCodeAt(0)]),
    );
    await step("csrutil", 10_000, () => vnc.type("csrutil disable\n"));
    await step("confirm", 10_000, () => vnc.type("y\n"));
    // Recent releases ask for an administrator's name, then the password. Where only the
    // password is asked for, the second line runs as a harmless unknown command.
    await step("username", 10_000, () => vnc.type("admin\n"));
    await step("password", 10_000, () => vnc.type("admin\n"));
    // Shutdown closes VNC, so no framebuffer can be requested after this step.
    await Bun.sleep(10_000);
    await vnc.type("halt\n");
    await withTimeout(run.exited, 120_000, "Recovery to shut down");
  } finally {
    vnc.close();
    await tart(["stop", vm]).catch(() => {});
  }
}

async function provision(vm: string, logDir: string) {
  const run = await runVm(vm, { logFile: join(logDir, "provision.log") });
  const guest = await Guest.connect(await vmIp(vm));
  try {
    await guest.waitUntilReady();
    const { stdout } = await guest.run("csrutil status");
    if (!stdout.includes("disabled")) {
      throw new Error(
        `SIP is still enabled (${stdout.trim()}); see the Recovery screenshots in ${logDir}`,
      );
    }
    await guest.run(`
      # Keep Spotlight from indexing in the background while tests run.
      sudo mdutil -a -i off >/dev/null
      # Do not reopen windows from the previous session at login, or an app's windows
      # when it is relaunched (tests quit Terminal between runs).
      defaults write com.apple.loginwindow TALLogoutSavesState -bool false
      defaults write -g NSQuitAlwaysKeepsWindows -bool false
    `);
    await guest.run("sudo shutdown -h now", { check: false });
    await withTimeout(run.exited, 180_000, "the VM to shut down");
  } finally {
    await guest.close();
    if ((await findVm(vm))?.running) await tart(["stop", vm]);
  }
}
