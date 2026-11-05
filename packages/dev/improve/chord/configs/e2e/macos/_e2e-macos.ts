#!/usr/bin/env bun
/* eslint-disable no-console -- This CLI writes usage and progress to stdout. */
/**
 * End-to-end tests of the production Chord app in a macOS VM run by tart. Each run clones
 * a golden VM (SIP disabled so Accessibility can be granted), installs a release DMG the
 * way a user does, and runs the `*.e2e.ts` files, which type real keystrokes through
 * Virtualization.framework's VNC server. Nothing touches the host's keyboard settings.
 *
 * `fresh`, `ui` and `journey` start further back, on a pristine VM with SIP on: `journey` goes
 * from the website to using a first chord by itself, and `fresh` with `ui` lets a person or an
 * agent do the same one step at a time.
 */
import { readdirSync, statSync } from "node:fs";
import { join, resolve } from "node:path";
import { parseArgs } from "node:util";
import { makeDmg } from "@chord/dev.improve.chord.configs.release.macos/make-dmg";
import { isNotarized } from "@chord/dev.improve.chord.configs.release.macos/notarize";
import { $ } from "bun";
import { grantAccessibility, installDmg, launch, quit } from "#chord-app";
import { createGolden, DEFAULT_IMAGE, GOLDEN_VM } from "#golden";
import { Guest } from "#guest";
import { JOURNEY_ENV, type JourneyState } from "#journey";
import { LIVE_SITE, startReleaseServer } from "#release-server";
import { deleteVm, findVm, runVm, tartBinary, vmIp } from "#tart";
import { serveUi, stopUiServer, uiRequest } from "#ui-server";
import {
  bootClone,
  E2E_DIR,
  forgetRunning,
  loadRunning,
  REPO_ROOT,
  saveRunning,
  SESSION_ENV,
  type SessionState,
} from "#vm";
import { withTimeout } from "#wait";

const USAGE = `Usage: vpx _e2e-macos <command> [options]

Commands:
  setup               Build the golden VM that test runs clone (once; pulls ~30 GB)
  test [files...]     Run the e2e tests against a production DMG in a fresh clone
  up                  Boot a clone with Chord installed and running, and leave it up
  fresh               Boot a pristine VM with SIP on and nothing installed, for going from
                      zero as a new user, and leave it up
  ui <action> [...]   Look at and act on a running VM as a person would (see below)
  journey             Run a new user's whole first run, from the website to using a chord
  down                Stop and delete a VM started by \`up\` or \`fresh\`
  screenshot [path]   Save the screen of a running VM
  ssh [command]       Open a shell, or run a command, in a running VM

Options:
  --dmg <path>        DMG to install (default: the newest from \`vpx _tauri build\`)
  --build             Build the DMG first, as the release workflow does
  --release <tag>     Download the DMG from a GitHub release (prod, beta, ...)
  --site <url>        Website to start from (journey; default ${LIVE_SITE})
  --published         Download the published release from the website (journey)
  --intel             Take the Intel DMG through the website's Intel link, and run it under
                      Rosetta (journey; with --published, --dmg or --release, not --build)
  --vm <name>         Use a VM started by \`up\` (test, down, screenshot, ssh)
  --keep              Leave the test VM running afterwards (test)
  --force             Rebuild an existing golden VM (setup)
  --image <ref>       Base image (setup; default ${DEFAULT_IMAGE})
  --concurrency <n>   Parallel image downloads (setup; default 4)
  --golden <name>     Golden VM (default ${GOLDEN_VM})

UI actions (each saves a numbered screenshot and prints the text on screen with coordinates):
  ui shot [name]              Look at the screen
  ui click <x> <y>|<text>     Click a point, or the first visible text containing <text>
  ui dclick <x> <y>|<text>    Double-click
  ui drag <x1> <y1> <x2> <y2> Drag with the left button
  ui type <text>              Type on the keyboard
  ui key <combo>              Press keys together, for example cmd+space or return
  ui chord <keys> [--caps]    Hold Caps+Space, type <keys>, and release (or press Caps)
  ui wait <text> [--timeout <seconds>]   Wait for text to appear
`;

const FRESH_DISPLAY = "1440x900";
const APP_DIR = join(REPO_ROOT, "apps/chord-tauri");
const TESTS_DIR = import.meta.dir;

const { values, positionals } = parseArgs({
  allowPositionals: true,
  options: {
    dmg: { type: "string" },
    build: { type: "boolean" },
    release: { type: "string" },
    vm: { type: "string" },
    keep: { type: "boolean" },
    force: { type: "boolean" },
    image: { type: "string", default: DEFAULT_IMAGE },
    concurrency: { type: "string", default: "4" },
    golden: { type: "string", default: GOLDEN_VM },
    caps: { type: "boolean" },
    site: { type: "string", default: LIVE_SITE },
    published: { type: "boolean" },
    intel: { type: "boolean" },
    timeout: { type: "string", default: "60" },
    help: { type: "boolean", short: "h" },
  },
});
const [command, ...args] = positionals;
const concurrency = Number(values.concurrency);
if (!Number.isSafeInteger(concurrency) || concurrency < 1) {
  throw new Error("--concurrency must be a positive integer");
}

function stamp(): string {
  return new Date().toISOString().replace(/[-:]/g, "").replace(/\..*/, "").replace("T", "-");
}

async function run(
  argv: string[],
  options: { cwd?: string; env?: Record<string, string | undefined> } = {},
) {
  const child = Bun.spawn(argv, { ...options, stdio: ["inherit", "inherit", "inherit"] });
  const status = await child.exited;
  if (status !== 0) throw new Error(`${argv.join(" ")} exited with ${status}`);
}

/** The DMG to test: given, downloaded from a release, or the newest local build. */
async function resolveDmg(): Promise<string> {
  if (values.dmg) return resolve(values.dmg);
  if (values.release) {
    const dir = join(E2E_DIR, "releases", values.release);
    // tart runs only Apple Silicon guests; the Intel build runs there under Rosetta.
    const asset = values.intel ? "Chord-x86_64.dmg" : "Chord-aarch64.dmg";
    await run([
      "gh",
      "release",
      "download",
      values.release,
      "--repo",
      "KeyChord/Chord",
      "--pattern",
      asset,
      "--dir",
      dir,
      "--clobber",
    ]);
    return join(dir, asset);
  }
  if (values.build) {
    // E2E needs the release app and DMG, without the separately signed updater archive.
    // Selecting dmg,app alone still creates that archive when createUpdaterArtifacts is
    // enabled in tauri.conf.json. CI=true keeps the DMG step from scripting Finder.
    await run(
      [
        process.execPath,
        join(APP_DIR, "_tauri.ts"),
        "build",
        "--bundles",
        "dmg,app",
        "--config",
        JSON.stringify({ bundle: { createUpdaterArtifacts: false } }),
      ],
      {
        cwd: APP_DIR,
        env: { ...process.env, CI: "true" },
      },
    );
    // The same disk image a release ships, not the one Tauri made along the way.
    const dmg = join(E2E_DIR, "build/Chord-aarch64.dmg");
    await $`mkdir -p ${join(E2E_DIR, "build")}`;
    await makeDmg(
      join(APP_DIR, "src-tauri/target/release/bundle/macos/Chord.app"),
      dmg,
      join(APP_DIR, "src-tauri/icons/icon.icns"),
    );
    return dmg;
  }
  const target = join(APP_DIR, "src-tauri/target");
  const dirs = [join(E2E_DIR, "build"), join(target, "release/bundle/dmg")];
  for (const entry of readdirSync(target, { withFileTypes: true })) {
    if (entry.isDirectory()) dirs.push(join(target, entry.name, "release/bundle/dmg"));
  }
  const dmgs = dirs.flatMap((dir) => {
    try {
      return readdirSync(dir)
        .filter((file) => file.endsWith(".dmg"))
        .map((file) => join(dir, file));
    } catch {
      return [];
    }
  });
  const newest = dmgs.sort((a, b) => statSync(b).mtimeMs - statSync(a).mtimeMs)[0];
  if (!newest) throw new Error("no DMG found; pass --build, --dmg <path>, or --release <tag>");
  return newest;
}

/** Installs the DMG and grants Accessibility; the app is not started. */
async function install(guest: Guest, dmg: string) {
  console.log(`installing ${dmg}`);
  const app = await installDmg(guest, dmg);
  await grantAccessibility(guest, app);
  console.log(`installed ${app.path} (${app.bundleId})`);
  return app;
}

function testFiles(): string[] {
  if (args.length > 0) return args.map((file) => resolve(file));
  return readdirSync(TESTS_DIR)
    .filter((file) => file.endsWith(".e2e.ts"))
    .map((file) => join(TESTS_DIR, file));
}

async function test() {
  const dmg = await resolveDmg();
  const id = stamp();
  const artifacts = join(E2E_DIR, "runs", id);
  // A VM started by `up` is reused; otherwise each run gets a fresh clone.
  const ephemeral = values.vm ? undefined : `chord-e2e-${id}`;
  if (ephemeral) {
    process.once("SIGINT", () => {
      console.log(`\ninterrupted; deleting ${ephemeral}`);
      void deleteVm(ephemeral).finally(() => process.exit(130));
    });
  }
  let guest: Guest | undefined;
  let state: SessionState | undefined;
  let status = 1;
  try {
    if (ephemeral) {
      console.log(`booting ${ephemeral} from ${values.golden}`);
      const vm = await bootClone(values.golden!, ephemeral, {
        logFile: join(artifacts, "tart.log"),
        detached: values.keep,
      });
      guest = vm.guest;
      state = {
        vm: ephemeral,
        host: vm.host,
        vncUrl: vm.vncUrl,
        app: await install(guest, dmg),
        artifacts,
      };
    } else {
      const running = await loadRunning(values.vm);
      guest = await Guest.connect(running.host);
      if (running.app) await quit(guest, running.app);
      const app = await install(guest, dmg);
      state = { vm: running.vm, host: running.host, vncUrl: running.vncUrl, app, artifacts };
    }
    console.log(`watch with: open '${state.vncUrl}'`);

    const sessionFile = join(artifacts, "session.json");
    await Bun.write(sessionFile, JSON.stringify(state, null, 2));
    const child = Bun.spawn(["bun", "test", "--timeout", "180000", ...testFiles()], {
      cwd: REPO_ROOT,
      env: { ...process.env, [SESSION_ENV]: sessionFile },
      stdio: ["inherit", "inherit", "inherit"],
    });
    status = await child.exited;
    if (state.app) await guest.copyFrom(state.app.log, join(artifacts, "chords.log")).catch(() => {});
  } finally {
    await guest?.close();
    if (ephemeral && values.keep && state) {
      await saveRunning(state);
      console.log(
        `left ${ephemeral} running; rerun with --vm ${ephemeral}, stop with \`vpx _e2e-macos down\``,
      );
    } else if (ephemeral) {
      await deleteVm(ephemeral);
    }
  }
  console.log(`artifacts: ${artifacts}`);
  process.exitCode = status;
}

async function up() {
  const dmg = await resolveDmg();
  const id = stamp();
  const name = `chord-e2e-${id}`;
  const artifacts = join(E2E_DIR, "runs", id);
  console.log(`booting ${name} from ${values.golden}`);
  const vm = await bootClone(values.golden!, name, {
    logFile: join(artifacts, "tart.log"),
    detached: true,
  });
  try {
    const app = await install(vm.guest, dmg);
    await launch(vm.guest, app);
    await saveRunning({ vm: name, host: vm.host, vncUrl: vm.vncUrl, app, artifacts });
  } catch (error) {
    await deleteVm(name);
    throw error;
  } finally {
    await vm.guest.close();
  }
  console.log(`${name} is running Chord at ${vm.host}`);
  console.log(`watch or control it with: open '${vm.vncUrl}'`);
}

/**
 * A VM as close to a new user's Mac as the base image allows: SIP on, nothing of Chord's, no
 * pre-granted permissions, a laptop-sized display, and Gatekeeper on (the image turns it off).
 */
async function bootFresh(name: string, logFile: string): Promise<{ host: string; vncUrl: string }> {
  console.log(`booting ${name} from ${values.image}`);
  const prepare = await bootClone(values.image!, name, { logFile, display: FRESH_DISPLAY });
  // The image pins a 1024x768 desktop in this file; without it macOS fits the display instead.
  await prepare.guest.run(`
    sudo spctl --global-enable
    sudo rm -f /Library/Preferences/com.apple.windowserver.displays.plist
  `);
  // The guest shutting down ends tart, so a VM can't reboot under a detached run.
  await prepare.guest.run("sudo shutdown -h now", { check: false });
  await withTimeout(prepare.exited, 180_000, `${name} to shut down`);
  await prepare.guest.close();

  const { vncUrl } = await runVm(name, { logFile, detached: true });
  const host = await vmIp(name);
  const guest = await Guest.connect(host);
  await guest.waitUntilReady();
  await guest.close();
  return { host, vncUrl };
}

async function fresh() {
  const id = stamp();
  const name = `chord-fresh-${id}`;
  const artifacts = join(E2E_DIR, "runs", id);
  const vm = await bootFresh(name, join(artifacts, "tart.log"));
  const state: SessionState = { vm: name, ...vm, artifacts };
  await saveRunning(state);
  try {
    await uiRequest(state, { action: "allow-known-developers", args: [] });
  } catch (error) {
    await stopUiServer(name);
    await deleteVm(name);
    forgetRunning(name);
    throw error;
  }
  console.log(`${name} is up at ${vm.host}; nothing is installed`);
  console.log(`watch it with: open '${vm.vncUrl}'`);
  console.log(`screenshots from \`ui\` go to ${join(artifacts, "screen")}`);
}

/**
 * The whole first run. With a local DMG (the default), Safari downloads it from a local copy of
 * the website, and Gatekeeper's check is skipped unless Apple has notarized that DMG. With
 * \`--published\`, everything is what a new user gets today.
 */
async function journey() {
  if (values.intel && values.build) throw new Error("--intel can't build here; pass --dmg or --release");
  const dmg = values.published ? undefined : await resolveDmg();
  const id = stamp();
  const name = `chord-journey-${id}`;
  const artifacts = join(E2E_DIR, "runs", id);
  process.once("SIGINT", () => {
    console.log(`\ninterrupted; deleting ${name}`);
    void deleteVm(name).finally(() => process.exit(130));
  });
  let status = 1;
  try {
    const vm = await bootFresh(name, join(artifacts, "tart.log"));
    const guest = await Guest.connect(vm.host);
    // tart runs only Apple Silicon guests, so this is as close to an Intel Mac as it gets.
    if (values.intel) await guest.run("sudo softwareupdate --install-rosetta --agree-to-license");
    const server =
      dmg || values.site !== LIVE_SITE
        ? await startReleaseServer(guest, {
            site: values.site!,
            dmg,
            name: values.intel ? "Chord-x86_64.dmg" : "Chord-aarch64.dmg",
          })
        : undefined;
    await guest.close();
    const state: JourneyState = {
      host: vm.host,
      vncUrl: vm.vncUrl,
      siteUrl: server?.url ?? values.site!,
      checkedByGatekeeper: dmg === undefined || (await isNotarized(dmg)),
      intel: values.intel === true,
      artifacts,
    };
    const file = join(artifacts, "journey.json");
    await Bun.write(file, JSON.stringify(state, null, 2));
    console.log(`watch with: open '${vm.vncUrl}'`);
    const child = Bun.spawn(
      ["bun", "test", "--timeout", "240000", join(TESTS_DIR, "first-run.journey.ts")],
      {
        cwd: REPO_ROOT,
        env: { ...process.env, [JOURNEY_ENV]: file },
        stdio: ["inherit", "inherit", "inherit"],
      },
    );
    status = await child.exited;
    server?.stop();
  } finally {
    if (values.keep) {
      console.log(`left ${name} running; delete it with \`tart delete ${name}\` once stopped`);
    } else {
      await deleteVm(name);
    }
  }
  console.log(`story of this run: ${join(artifacts, "journey.md")}`);
  process.exitCode = status;
}

async function ui() {
  const [action, ...rest] = args;
  if (!action) throw new Error(`ui needs an action\n\n${USAGE}`);
  const state = await loadRunning(values.vm);
  const reply = await uiRequest(state, {
    action,
    args: rest,
    timeoutSeconds: Number(values.timeout),
    caps: values.caps,
  });
  console.log(reply);
}

async function down() {
  const name = values.vm ?? (await loadRunning().catch(() => undefined))?.vm;
  if (!name) throw new Error("no VM to stop; pass --vm <name>");
  await stopUiServer(name);
  await deleteVm(name);
  forgetRunning(name);
  console.log(`deleted ${name}`);
}

async function screenshot() {
  const state = await loadRunning(values.vm);
  const path = resolve(args[0] ?? join(E2E_DIR, `screenshot-${stamp()}.png`));
  await uiRequest(state, { action: "shot", args: [path] });
  console.log(path);
}

async function ssh() {
  const state = await loadRunning(values.vm);
  const guest = await Guest.connect(state.host);
  process.exitCode = await guest.shell(args.join(" ") || undefined);
  await guest.close();
}

if (values.help || !command) {
  console.log(USAGE);
  process.exit(values.help ? 0 : 1);
}
tartBinary();
switch (command) {
  case "setup":
    if ((await findVm(values.golden!)) && !values.force) {
      console.log(`${values.golden} already exists; pass --force to rebuild it`);
      break;
    }
    await createGolden({
      image: values.image!,
      name: values.golden!,
      concurrency,
      logDir: join(E2E_DIR, "setup", stamp()),
      log: (message) => console.log(message),
    });
    break;
  case "test":
    await test();
    break;
  case "up":
    await up();
    break;
  case "fresh":
    await fresh();
    break;
  case "journey":
    await journey();
    break;
  case "ui":
    await ui();
    break;
  case "__ui-server":
    await serveUi(await loadRunning(values.vm));
    break;
  case "down":
    await down();
    break;
  case "screenshot":
    await screenshot();
    break;
  case "ssh":
    await ssh();
    break;
  default:
    console.error(`unknown command ${command}\n\n${USAGE}`);
    process.exit(1);
}
