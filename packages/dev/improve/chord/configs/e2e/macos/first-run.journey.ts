// A new user's whole first run, on a Mac as new as the base image allows: visit the website,
// download Chord through Safari, install it from the disk image, follow the setup in the app,
// make a first chord, and use it. Everything goes through the screen, mouse and keyboard, by the
// text a person would look for. SSH only checks the results. See development.md.
import { afterAll, beforeAll, expect } from "bun:test";
import { allowKnownDevelopers } from "#fresh-mac";
import { Journey } from "#journey";
import { Keysym } from "#keysyms";
import { center } from "#screen";
import { waitFor } from "#wait";

const journey = await Journey.open();
const { guest, screen, vnc } = journey;
const DOWNLOADED = `${guest.home}/Downloads/${journey.intel ? "Chord-x86_64.dmg" : "Chord-aarch64.dmg"}`;
const FIRST_CHORD = "/s";

beforeAll(() => screen.wake());
afterAll(() => journey.close());

/** The whole text on a button, since instructions mention the same words in sentences. */
const button = (label: string) => new RegExp(`^\\s*${label}\\s*$`);

/** macOS's offer to install the developer tools, which a new user should never see from Chord. */
const asksForDeveloperTools = async () =>
  (
    await guest.run("lsappinfo list | grep -q com.apple.dt.CommandLineTools.installondemand", {
      check: false,
    })
  ).exitCode === 0;

/** The app in front, which is the one a person is looking at. */
async function frontApp(): Promise<string> {
  const { stdout } = await guest.run(
    `lsappinfo info "$(lsappinfo front)" | sed -n 's/.*bundleID="\\([^"]*\\)".*/\\1/p'`,
  );
  return stdout.trim();
}

journey.step("the Mac is set up like a new one", async () => {
  await allowKnownDevelopers(guest, screen);
  const { stdout } = await guest.run("csrutil status; spctl --status; ls /Applications");
  expect(stdout).toContain("enabled.");
  expect(stdout).toContain("assessments enabled");
  expect(stdout).not.toContain("Chord");
});

journey.step("the website says what to press and how to get Chord", async () => {
  // LaunchServices occasionally refuses this just after another app has quit.
  await journey.waitUntil(
    "Safari to open the website",
    async () => (await guest.run(`open -a Safari ${journey.siteUrl}`, { check: false })).exitCode === 0,
    { timeoutMs: 15_000, intervalMs: 1_000 },
  );
  await screen.waitForText("Download for Mac", { timeoutMs: 60_000 });
  await screen.waitForText("caps lock");
});

/** Scrolls down a few lines at a time, as a person would, until `query` is on screen. */
async function scrollTo(query: string | RegExp) {
  return await waitFor(
    `${String(query)} on screen`,
    async () => {
      const box = await screen.find(query);
      if (box) return box;
      // The VNC server ignores the scroll wheel.
      for (let line = 0; line < 3; line++) {
        vnc.key(Keysym.Down, true);
        vnc.key(Keysym.Down, false);
      }
      return undefined;
    },
    { timeoutMs: 30_000, intervalMs: 500 },
  );
}

journey.step("downloading Chord says what happens next", async () => {
  // The link's underline runs into the last letter of "Intel" for OCR.
  const link = journey.intel ? /Download for Inte\S* Macs/ : "Download for Mac";
  await scrollTo(link);
  await screen.clickText(link);
  // Safari asks once per website.
  await screen.clickText(button("Allow"), { timeoutMs: 15_000 });
  await journey.waitForFile(DOWNLOADED);
  await screen.waitForText("Your download has started");
  await screen.waitForText("Applications");
  const { stdout } = await guest.run(`xattr -p com.apple.quarantine ${DOWNLOADED}`);
  expect(stdout).toContain("Safari");
});

journey.step("dragging Chord to Applications installs it", async () => {
  if (!journey.checkedByGatekeeper) {
    await guest.run(`xattr -d com.apple.quarantine ${DOWNLOADED}`);
  }
  await guest.run(`open ${DOWNLOADED}`);
  await journey.waitForFile("/Volumes/Chord/Chord.app/Contents/Info.plist");
  await screen.waitForText("Drag Chord to Applications", { timeoutMs: 30_000 });
  // Lowest on screen: the icon's label, not the window's title or the disk on the desktop.
  const source = (await screen.read()).filter((box) => button("Chord").test(box.text)).at(-1);
  const destination = await screen.find(button("Applications"));
  if (!source || !destination) throw new Error("the installer window is not showing both icons");
  // Each icon is above its label.
  const lift = 100;
  const from = center(source);
  const to = center(destination);
  await vnc.drag({ x: from.x, y: from.y - lift }, { x: to.x, y: to.y - lift });
  await journey.waitForFile("/Applications/Chord.app/Contents/Info.plist");
  // Not lipo: on a new Mac that asks to install the developer tools.
  const { stdout } = await guest.run("file -b /Applications/Chord.app/Contents/MacOS/chord");
  expect(stdout).toContain(journey.intel ? "x86_64" : "arm64");
});

journey.step("opening Chord starts setup, and nothing asks for developer tools", async () => {
  await guest.run("open /Applications/Chord.app");
  if (journey.checkedByGatekeeper) await screen.clickText(button("Open"), { timeoutMs: 30_000 });
  await screen.waitForText("Welcome to Chord", { timeoutMs: 60_000 });
  expect(await asksForDeveloperTools()).toBe(false);
  // Its window would otherwise sit on top of setup.
  await journey.waitUntil(
    "the installer to be ejected",
    async () => (await guest.run("test -e /Volumes/Chord", { check: false })).exitCode !== 0,
    { timeoutMs: 15_000 },
  );
});

journey.step("one click on the first screen is enough", async () => {
  await screen.clickText(button("Get Started\\s*→?"));
  await screen.waitForText("Allow Chord to use your keyboard", { timeoutMs: 10_000 });
});

journey.step("quitting partway through picks up where setup left off", async () => {
  await guest.run(`pkill -f 'Contents/MacOS/chord$'`);
  await journey.waitUntil("Chord to quit", async () =>
    (await guest.run(`pgrep -f 'Contents/MacOS/chord$'`, { check: false })).exitCode !== 0,
  );
  // LaunchServices can refuse this (-600) for a moment after the app quits, more so under Rosetta.
  await journey.waitUntil(
    "Chord to open again",
    async () => (await guest.run("open /Applications/Chord.app", { check: false })).exitCode === 0,
    { timeoutMs: 15_000, intervalMs: 1_000 },
  );
  await screen.clickText("Get Started", { timeoutMs: 60_000 });
  await screen.waitForText("Allow Chord to use your keyboard", { timeoutMs: 10_000 });
});

journey.step("the permission steps match what macOS shows, and setup comes back", async () => {
  await screen.waitForText("Device Control and Data Access");
  await screen.clickText(button("Open System Settings"));
  await screen.waitForText("would like to control this Mac", { timeoutMs: 15_000 });
  // This window's button has changed by now, so this is the one in macOS's message.
  await screen.clickText(button("Open System Settings"));
  const list = center(await screen.waitForText("Apps with this access", { timeoutMs: 15_000 }));
  const chordRow = await waitForChordRow(list.y);
  await vnc.click(chordRow.x + SWITCH_OFFSET, chordRow.y);
  await screen.authorize();
  await screen.waitForText("Make your first chord", { timeoutMs: 20_000 });
  expect(await frontApp()).toBe("com.leonsilicon.chord");
});

/** From the middle of "Chord" in its row to the switch at the far end of the row. */
const SWITCH_OFFSET = 757;

async function waitForChordRow(belowY: number) {
  const row = await waitFor(
    "Chord on the permission page",
    async () =>
      (await screen.read()).find(
        (box) => button("Chord").test(box.text) && center(box).y > belowY,
      ),
    { timeoutMs: 15_000, intervalMs: 1_000 },
  );
  return center(row);
}

journey.step("choosing an app makes a first chord", async () => {
  await screen.clickText(button("Safari"), { timeoutMs: 30_000 });
  await screen.waitForText("you'll type");
  await screen.clickText("Create Chord");
  await screen.waitForText("Now try it", { timeoutMs: 20_000 });
  const contents = await guest.readFile(`${guest.home}/.chord/packages/default/chords/macos.toml`);
  expect(contents).toContain(`"${FIRST_CHORD}" = { name = "Safari", shell = "open -b com.apple.Safari" }`);
});

journey.step("practice explains letting go too soon", async () => {
  vnc.key(Keysym.CapsLock, true);
  await Bun.sleep(40);
  vnc.key(Keysym.Space, true);
  await Bun.sleep(40);
  vnc.key(Keysym.CapsLock, false);
  await Bun.sleep(600);
  await screen.waitForText("list of your chords", { timeoutMs: 10_000 });
  vnc.key(Keysym.Space, false);
  await screen.waitForText("You let go of space before typing", { timeoutMs: 10_000 });
});

journey.step("practice notices the wrong chord", async () => {
  await screen.chord("/x");
  await screen.waitForText("give it another try", { timeoutMs: 10_000 });
});

journey.step("using the chord opens the app, for real", async () => {
  await screen.chord(FIRST_CHORD);
  await waitForFront("com.apple.Safari");
  await screen.waitForText("You did it!", { timeoutMs: 15_000 });
  await waitForFront("com.leonsilicon.chord");
});

async function waitForFront(bundleId: string) {
  await journey.waitUntil(`${bundleId} to come to the front`, async () => (await frontApp()) === bundleId, {
    timeoutMs: 15_000,
    intervalMs: 150,
  });
}

journey.step("a chord pack can be added without developer tools", async () => {
  await screen.clickText(button("Continue"));
  await screen.waitForText("Want chords for the apps you use?");
  await screen.waitForText("Terminal Chords");
  const { x, y } = center((await screen.find("Terminal Chords"))!);
  const add = (await screen.read()).find(
    (box) => button("Add").test(box.text) && Math.abs(center(box).y - y) < 40 && center(box).x > x,
  );
  if (!add) throw new Error("no Add button next to Terminal Chords");
  const workers = await packageWorkers();
  await vnc.click(center(add).x, center(add).y);
  await screen.waitForText("Added", { timeoutMs: 120_000 });
  expect(await asksForDeveloperTools()).toBe(false);
  // Only the new pack loads; the packages already running keep their workers.
  const after = await packageWorkers();
  expect(workers.filter((pid) => !after.includes(pid))).toEqual([]);
});

/** Process IDs of the package workers, one per loaded package with JavaScript. */
async function packageWorkers(): Promise<string[]> {
  const { stdout } = await guest.run("pgrep -f 'Contents/MacOS/chord __package-worker'", { check: false });
  return stdout.split("\n").filter(Boolean);
}

journey.step("finishing leaves Chord ready, and starting at login", async () => {
  await screen.clickText(button("Continue"));
  await screen.waitForText("You're all set");
  await screen.clickText(button("Finish"));
  await screen.waitForText("My Chords", { timeoutMs: 15_000 });
  const store = await guest.readFile(
    `${guest.home}/Library/Application Support/com.leonsilicon.chord/app-state.json`,
  );
  expect(JSON.parse(store!).onboardingComplete).toBe(true);
  const agents = await guest.run("ls ~/Library/LaunchAgents");
  expect(agents.stdout).toContain("Chord");
});

journey.step("the chord still works after setup", async () => {
  await guest.run("killall Safari", { check: false });
  await screen.chord(FIRST_CHORD);
  await waitForFront("com.apple.Safari");
});

/** No official package ships an Intel add-on yet, so on Intel their native chords are skipped. */
const NO_INTEL_ADDON = /has no addon for darwin-x64/;

// The packages Chord installs on first launch are pinned in apps/chord-tauri/data, and a pin can
// fall behind what this app reads. A chords file it can't read, or whose handler fails to load,
// is skipped with only a log line.
journey.step("every chord in the default packages loads", async () => {
  const log = await guest.readFile(`${guest.home}/Library/Logs/com.leonsilicon.chord/chords.log`);
  const skipped = (log ?? "")
    .split("\n")
    .filter((line) => /couldn't find any actions|parse error|skipping chords file|skipping handler/.test(line))
    .filter((line) => !(journey.intel && NO_INTEL_ADDON.test(line)));
  expect(skipped).toEqual([]);
});

journey.step("a Finder chord from the default packages works", async () => {
  await guest.run("open /Applications");
  await waitForFront("com.apple.finder");
  await screen.chord("gh");
  // Only the home folder has this one.
  await screen.waitForText(button("Public"), { timeoutMs: 10_000 });
});

journey.step("a menu bar chord from the default packages works", async () => {
  if (journey.intel) {
    // Fails once Intel add-ons ship, as a reminder to run the chord here too.
    const log = await guest.readFile(`${guest.home}/Library/Logs/com.leonsilicon.chord/chords.log`);
    expect(log).toMatch(NO_INTEL_ADDON);
    return;
  }
  // Its handler runs Swift through a native add-on.
  await screen.chord("=0.");
  await screen.waitForText("About This Mac", { timeoutMs: 10_000 });
  vnc.key(Keysym.Escape, true);
  vnc.key(Keysym.Escape, false);
});
