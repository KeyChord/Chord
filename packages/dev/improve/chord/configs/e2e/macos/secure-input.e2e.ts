// Chords typed while Secure Input hides the keyboard from Chord's event tap. Caps+Space
// then arrives through hotkeys and the chord through Chord's own key window
// (app/keyboard/caps_hotkey.rs and key_catcher.rs).
import { afterAll, beforeAll, beforeEach, describe, expect } from "bun:test";
import { join } from "node:path";
import { Keysym } from "#keysyms";
import { Session } from "#session";
import { E2E_DIR } from "#vm";
import { waitFor } from "#wait";

const MARKERS = "/tmp/chord-e2e";
const HOLDER_SOURCE = join(import.meta.dir, "secure-input-holder.swift");
const HOLDER_BINARY = join(E2E_DIR, "bin/secure-input-holder");
const HOLDER_APP = "/tmp/chord-e2e-holder/SecureInputHolder.app";

const PACKAGE = {
  "chords/macos.toml": `name = "e2e-secure"

[chords]
"/er" = { name = "E2E: shell", shell = "echo shell > ${MARKERS}/shell" }
"/ez" = { name = "E2E: type z", shortcut = "z" }
`,
};

const session = await Session.open();

/** Whether a process has Secure Input enabled, which `ioreg` shows on the session. */
async function secureInputOn(): Promise<boolean> {
  const { stdout } = await session.guest.run(
    "ioreg -l -w 0 | grep -c kCGSSessionSecureInputPID || true",
  );
  return Number(stdout.trim()) > 0;
}

async function waitForSecureInput() {
  await waitFor("Secure Input to be enabled", async () =>
    (await secureInputOn()) ? true : undefined,
  );
}

/** Starts an app in the VM that enables Secure Input and keeps it for good. */
async function startHolder() {
  const compile = Bun.spawn(["xcrun", "swiftc", "-O", HOLDER_SOURCE, "-o", HOLDER_BINARY], {
    stdout: "inherit",
    stderr: "inherit",
  });
  if ((await compile.exited) !== 0) throw new Error("cannot compile secure-input-holder.swift");
  await session.guest.run(`rm -rf ${HOLDER_APP}`);
  await session.guest.writeFile(
    `${HOLDER_APP}/Contents/Info.plist`,
    `<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>dev.improve.chord.e2e.secure-input-holder</string>
<key>CFBundleExecutable</key><string>holder</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>LSUIElement</key><true/>
</dict></plist>
`,
  );
  await session.guest.writeFile(
    `${HOLDER_APP}/Contents/MacOS/holder`,
    await Bun.file(HOLDER_BINARY).bytes(),
    { mode: 0o755 },
  );
  await session.guest.run(`open -g ${HOLDER_APP}`);
  await waitForSecureInput();
}

async function stopHolder() {
  await session.guest.run("killall holder", { check: false });
}

/**
 * The two ways Secure Input gets in Chord's way. Terminal's Secure Keyboard Entry
 * stands for an app that has it on while it has the keyboard, as with a focused
 * password field: it lets go when Chord's key window takes the keyboard, so the event
 * tap sees the rest of the chord. A background holder never lets go, and the whole
 * chord is read through the key window.
 */
const HOLDERS = [
  {
    name: "held by the app in front",
    setup: () =>
      session.guest.run("defaults write com.apple.Terminal SecureKeyboardEntry -bool true"),
    teardown: () =>
      session.guest.run("defaults delete com.apple.Terminal SecureKeyboardEntry", {
        check: false,
      }),
  },
  { name: "held by an app in the background", setup: startHolder, teardown: stopHolder },
];

async function openRecorder(file: string) {
  const recorder = await session.openRecorder(file);
  await waitForSecureInput();
  return recorder;
}

beforeAll(() => session.restartChord({ "e2e-secure": PACKAGE }), 180_000);
beforeEach(async () => {
  await session.guest.run("killall Terminal", { check: false });
  await session.guest.run(`rm -rf ${MARKERS} && mkdir -p ${MARKERS}`);
});
afterAll(async () => {
  await session.guest.run("killall Terminal", { check: false });
  await session.close();
});

for (const holder of HOLDERS) {
  describe(`Secure Input ${holder.name}`, () => {
    beforeAll(async () => {
      await session.guest.run("killall Terminal", { check: false });
      await holder.setup();
    }, 120_000);
    afterAll(async () => {
      await holder.teardown();
    });

    session.test("a chord runs, and its keys stay out of the app", async () => {
      const recorder = await openRecorder(`${MARKERS}/typed`);
      try {
        await session.vnc.type("ab");
        await session.chord("/er");
        expect(await session.waitForFile(`${MARKERS}/shell`)).toBe("shell\n");
        // The keyboard is the app's again as soon as the chord ends.
        await session.vnc.type("cd\n");
        expect(await session.waitForFile(`${MARKERS}/typed`)).toBe("abcd\n");
        // Secure Input is still on: Chord did not get there through its event tap.
        expect(await secureInputOn()).toBe(true);
      } finally {
        await recorder.close();
      }
    });

    session.test("a second chord runs after the first", async () => {
      const recorder = await openRecorder(`${MARKERS}/typed`);
      try {
        await session.chord("/er");
        expect(await session.waitForFile(`${MARKERS}/shell`)).toBe("shell\n");
        await session.guest.run(`rm ${MARKERS}/shell`);
        await session.chord("/er");
        expect(await session.waitForFile(`${MARKERS}/shell`)).toBe("shell\n");
      } finally {
        await recorder.close();
      }
    });

    session.test("Caps runs the chord while Space is held", async () => {
      const recorder = await openRecorder(`${MARKERS}/typed`);
      try {
        await session.chord("/er", { complete: "caps" });
        expect(await session.waitForFile(`${MARKERS}/shell`)).toBe("shell\n");
        await session.vnc.type("ok\n");
        expect(await session.waitForFile(`${MARKERS}/typed`)).toBe("ok\n");
      } finally {
        await recorder.close();
      }
    });

    session.test("a key the chord simulates reaches the app", async () => {
      const recorder = await openRecorder(`${MARKERS}/typed`);
      try {
        const { vnc } = session;
        await vnc.type("a");
        vnc.key(Keysym.CapsLock, true);
        await Bun.sleep(40);
        vnc.key(Keysym.Space, true);
        await Bun.sleep(40);
        vnc.key(Keysym.CapsLock, false);
        await Bun.sleep(80);
        await vnc.type("/ez");
        // Caps runs the chord. Space stays down, so Chord's window can still have the
        // keyboard when the simulated key arrives, and has to pass it on.
        await vnc.press(Keysym.CapsLock);
        await Bun.sleep(300);
        vnc.key(Keysym.Space, false);
        await Bun.sleep(80);
        await vnc.type("b\n");
        expect(await session.waitForFile(`${MARKERS}/typed`)).toBe("azb\n");
      } finally {
        await recorder.close();
      }
    });

    session.test("with Caps Lock on, Caps then Space types the space", async () => {
      const recorder = await openRecorder(`${MARKERS}/typed`);
      try {
        const { vnc } = session;
        await vnc.type("a");
        await vnc.press(Keysym.CapsLock);
        await vnc.type("b");
        // Caps turns Caps Lock off; the Space pressed before it is let go is a space.
        vnc.key(Keysym.CapsLock, true);
        await Bun.sleep(40);
        await vnc.press(Keysym.Space);
        vnc.key(Keysym.CapsLock, false);
        await Bun.sleep(80);
        await vnc.type("/er\n");
        expect(await session.waitForFile(`${MARKERS}/typed`)).toBe("aB /er\n");
        expect(
          (await session.guest.run(`test -e ${MARKERS}/shell`, { check: false })).exitCode,
        ).toBe(1);
      } finally {
        await recorder.close();
      }
    });
  });
}
