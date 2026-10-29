// The Caps key remap (app/keyboard/caps_remap.rs), which changes system-wide keyboard
// state and so is only exercised here, inside a VM.
import { afterAll, beforeAll, beforeEach, expect } from "bun:test";
import { waitForExit } from "#chord-app";
import { Keysym } from "#keysyms";
import { Session } from "#session";
import { waitFor } from "#wait";

const MARKERS = "/tmp/chord-e2e";

const session = await Session.open();

/** Whether `UserKeyMapping` sends the Caps key (0x700000039) as F20 (0x70000006F). */
async function capsRemapped(): Promise<boolean> {
  const { stdout } = await session.guest.run("hidutil property --get UserKeyMapping");
  return /HIDKeyboardModifierMappingDst = 30064771183;\s*HIDKeyboardModifierMappingSrc = 30064771129;/.test(
    stdout,
  );
}

async function ensureRunning() {
  if ((await session.pid()) === undefined) await session.restartChord();
}

beforeAll(() => session.restartChord(), 180_000);
beforeEach(() => session.guest.run(`rm -rf ${MARKERS} && mkdir -p ${MARKERS}`));
afterAll(() => session.close());

session.test("Chord remaps the Caps key while it owns input", async () => {
  expect(await capsRemapped()).toBe(true);
});

session.test("a plain Caps tap toggles Caps Lock", async () => {
  const recorder = await session.openRecorder(`${MARKERS}/typed`);
  try {
    await session.vnc.type("a");
    await session.vnc.press(Keysym.CapsLock);
    await session.vnc.type("b");
    await session.vnc.press(Keysym.CapsLock);
    await session.vnc.type("c\n");
    expect(await session.waitForFile(`${MARKERS}/typed`)).toBe("aBc\n");
  } finally {
    await recorder.close();
  }
});

session.test("the Caps guardian gives the Caps key back after a crash", async () => {
  await ensureRunning();
  const pid = (await session.pid())!;
  await session.guest.run(`kill -KILL ${pid}`);
  await waitForExit(session.guest, pid);
  // launchd may throttle the guardian's start by up to 10 seconds.
  await waitFor(
    "the guardian to remove the mapping",
    async () => ((await capsRemapped()) ? undefined : true),
    {
      timeoutMs: 45_000,
      intervalMs: 1_000,
    },
  );
});

session.test("quitting with 0q gives the Caps key back", async () => {
  await ensureRunning();
  expect(await capsRemapped()).toBe(true);
  const pid = (await session.pid())!;
  await session.chord("0q");
  await waitForExit(session.guest, pid);
  expect(await capsRemapped()).toBe(false);
});
