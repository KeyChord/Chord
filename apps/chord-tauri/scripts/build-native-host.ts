/**
 * Builds the `chord-native-host` sidecar and places it where Tauri's `externalBin` expects it:
 * `src-tauri/binaries/chord-native-host-<host triple>`. Tauri copies it next to the main
 * binary for `tauri dev` and into `Chord.app/Contents/MacOS/` for bundles.
 *
 * The host is always built in release mode (it is tiny and its latency matters); pass `--debug`
 * to build it unoptimized.
 */
import { $ } from "bun";
import fs from "node:fs";
import path from "node:path";

const srcTauri = path.resolve(import.meta.dir, "../src-tauri");
const debug = process.argv.includes("--debug");
const profileArgs = debug ? [] : ["--release"];
const profileDir = debug ? "debug" : "release";

const rustcInfo = await $`rustc -vV`.text();
const triple = rustcInfo.match(/^host: (.+)$/m)?.[1];
if (!triple) {
  throw new Error(`could not determine host triple from rustc -vV:\n${rustcInfo}`);
}

await $`cargo build -p chord-native-host ${profileArgs}`.cwd(srcTauri);

const built = path.join(srcTauri, "target", profileDir, "chord-native-host");
const binariesDir = path.join(srcTauri, "binaries");
const destination = path.join(binariesDir, `chord-native-host-${triple}`);
fs.mkdirSync(binariesDir, { recursive: true });
fs.copyFileSync(built, destination);
fs.chmodSync(destination, 0o755);
console.log(`native host -> ${path.relative(process.cwd(), destination)}`);
