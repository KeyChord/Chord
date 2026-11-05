#!/usr/bin/env bun
/* eslint-disable no-console -- This release CLI writes progress and usage to stdout. */
/**
 * macOS release pipeline for the full-power build.
 *
 * Tauri signs every bundled executable with the same entitlements file, but the native host
 * needs `com.apple.security.cs.disable-library-validation` (it loads package-built dylibs) while
 * the main app should keep library validation. So after `tauri build` produces the signed .app
 * we re-sign the sidecar with `NativeHost.entitlements`, re-sign the bundle, and then produce the
 * DMG and updater artifact ourselves.
 *
 * Environment:
 *   APPLE_SIGNING_IDENTITY   overrides bundle.macOS.signingIdentity from tauri.conf.json, whose
 *                            development identity Apple won't notarize: use the Developer ID one
 *   TAURI_SIGNING_PRIVATE_KEY (+ _PASSWORD)  enables signing the updater artifact
 *   CHORD_NOTARY_PROFILE, or APPLE_ID / APPLE_PASSWORD / APPLE_TEAM_ID  enables notarization + stapling
 *   CODESIGN_NO_TIMESTAMP=1  skips the trusted timestamp (offline builds)
 */
import { makeDmg } from "#make-dmg";
import { isNotarized, notarize, notaryCredentials } from "#notarize";
import { $ } from "bun";
import fs from "node:fs";
import path from "node:path";

if (process.argv.slice(2).some((arg) => arg === "--help" || arg === "-h")) {
  console.log("Usage: vpx _release-macos\n\nBuild, sign and package the production macOS app.");
  process.exit(0);
}

const repoRoot = path.resolve(import.meta.dir, "../../../../../../..");
const appDir = path.join(repoRoot, "apps/chord-tauri");
const srcTauri = path.join(appDir, "src-tauri");
const config = await Bun.file(path.join(srcTauri, "tauri.conf.json")).json();
const identity: string | undefined =
  process.env.APPLE_SIGNING_IDENTITY ?? config.bundle?.macOS?.signingIdentity;
if (!identity) {
  throw new Error(
    "no signing identity: set APPLE_SIGNING_IDENTITY or bundle.macOS.signingIdentity",
  );
}
const productName: string = config.productName ?? "Chord";
const timestampArgs = process.env.CODESIGN_NO_TIMESTAMP ? [] : ["--timestamp"];

const tauriScript = path.join(appDir, "_tauri.ts");
await $`${process.execPath} ${tauriScript} build --bundles app`.cwd(appDir);

const bundleDir = path.join(srcTauri, "target/release/bundle/macos");
const app = path.join(bundleDir, `${productName}.app`);
// The embedded Bun runtime (rbun) ships as a framework dylib; tauri copies it from
// `bundle.macOS.frameworks` and signs it, the app's rpath points at Contents/Frameworks.
const bunRuntime = path.join(app, "Contents/Frameworks/libbun_embed.dylib");
if (!(await Bun.file(bunRuntime).exists())) {
  throw new Error(`embedded Bun runtime missing from bundle: ${bunRuntime}`);
}

console.log("re-signing app bundle with Entitlements.plist");
await $`codesign --force --options runtime ${timestampArgs} --entitlements ${path.join(srcTauri, "Entitlements.plist")} --sign ${identity} ${app}`;
await $`codesign --verify --deep --strict --verbose=2 ${app}`;

const outDir = path.join(srcTauri, "target/release/bundle/chord-release");
fs.rmSync(outDir, { recursive: true, force: true });
fs.mkdirSync(outDir, { recursive: true });

// In this order, so that the app inside the disk image and the updater archive carry their tickets.
const credentials = notaryCredentials();
if (credentials) {
  console.log("notarizing the app");
  await notarize(app, credentials);
} else {
  console.log("no notary credentials; skipping notarization");
}

const dmg = path.join(outDir, `${productName}.dmg`);
console.log(`creating ${dmg}`);
await makeDmg(app, dmg, path.join(srcTauri, "icons/icon.icns"));
if (credentials) {
  console.log("notarizing the disk image");
  await notarize(dmg, credentials);
  for (const artifact of [app, dmg]) {
    if (!(await isNotarized(artifact))) throw new Error(`${artifact} isn't notarized`);
  }
}

const updaterArchive = path.join(outDir, `${productName}.app.tar.gz`);
console.log(`creating updater artifact ${updaterArchive}`);
await $`tar -czf ${updaterArchive} -C ${bundleDir} ${productName}.app`;
if (process.env.TAURI_SIGNING_PRIVATE_KEY) {
  await $`${process.execPath} ${tauriScript} signer sign ${updaterArchive}`.cwd(appDir);
} else {
  console.log("TAURI_SIGNING_PRIVATE_KEY not set; updater artifact left unsigned");
}

console.log(`done: ${outDir}`);
