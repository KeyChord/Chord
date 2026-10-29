#!/usr/bin/env bun
/* eslint-disable no-console -- This CLI writes usage and progress to stdout. */
/**
 * End-to-end tests of the production Chord app in a macOS VM run by tart. Each run clones
 * a golden VM (SIP disabled so Accessibility can be granted), installs a release DMG the
 * way a user does, and runs the `*.e2e.ts` files, which type real keystrokes through
 * Virtualization.framework's VNC server. Nothing touches the host's keyboard settings.
 */
import { readdirSync, statSync } from "node:fs";
import { join, resolve } from "node:path";
import { parseArgs } from "node:util";
import { grantAccessibility, installDmg, launch, quit } from "#chord-app";
import { createGolden, DEFAULT_IMAGE, GOLDEN_VM } from "#golden";
import { Guest } from "#guest";
import { deleteVm, findVm, tartBinary } from "#tart";
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
import { VncClient } from "#vnc";

const USAGE = `Usage: vpx _e2e-macos <command> [options]

Commands:
  setup               Build the golden VM that test runs clone (once; pulls ~30 GB)
  test [files...]     Run the e2e tests against a production DMG in a fresh clone
  up                  Boot a clone with Chord installed and running, and leave it up
  down                Stop and delete a VM started by \`up\`
  screenshot [path]   Save the screen of a VM started by \`up\`
  ssh [command]       Open a shell, or run a command, in a VM started by \`up\`

Options:
  --dmg <path>        DMG to install (default: the newest from \`vpx _tauri build\`)
  --build             Build the DMG first, as the release workflow does
  --release <tag>     Download Chord-aarch64.dmg from a GitHub release (prod, beta, ...)
  --vm <name>         Use a VM started by \`up\` (test, down, screenshot, ssh)
  --keep              Leave the test VM running afterwards (test)
  --force             Rebuild an existing golden VM (setup)
  --image <ref>       Base image (setup; default ${DEFAULT_IMAGE})
  --concurrency <n>   Parallel image downloads (setup; default 4)
  --golden <name>     Golden VM (default ${GOLDEN_VM})
`;

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
    // tart runs only Apple Silicon guests.
    const asset = "Chord-aarch64.dmg";
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
  }
  const target = join(APP_DIR, "src-tauri/target");
  const dirs = [join(target, "release/bundle/dmg")];
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
      await quit(guest, running.app);
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
    await guest.copyFrom(state.app.log, join(artifacts, "chords.log")).catch(() => {});
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

async function down() {
  const name = values.vm ?? (await loadRunning().catch(() => undefined))?.vm;
  if (!name) throw new Error("no VM to stop; pass --vm <name>");
  await deleteVm(name);
  forgetRunning(name);
  console.log(`deleted ${name}`);
}

async function screenshot() {
  const state = await loadRunning(values.vm);
  const vnc = await VncClient.connect(state.vncUrl);
  const path = resolve(args[0] ?? join(E2E_DIR, `screenshot-${stamp()}.png`));
  await Bun.write(path, await vnc.screenshot());
  vnc.close();
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
