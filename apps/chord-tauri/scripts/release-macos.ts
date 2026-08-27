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
 *   APPLE_SIGNING_IDENTITY   overrides bundle.macOS.signingIdentity from tauri.conf.json
 *   TAURI_SIGNING_PRIVATE_KEY (+ _PASSWORD)  enables signing the updater artifact
 *   APPLE_ID / APPLE_PASSWORD / APPLE_TEAM_ID  enables notarization + stapling
 *   CODESIGN_NO_TIMESTAMP=1  skips the trusted timestamp (offline builds)
 */
import { $ } from "bun";
import fs from "node:fs";
import path from "node:path";

const appDir = path.resolve(import.meta.dir, "..");
const srcTauri = path.join(appDir, "src-tauri");
const config = JSON.parse(fs.readFileSync(path.join(srcTauri, "tauri.conf.json"), "utf8"));
const identity: string | undefined =
  process.env.APPLE_SIGNING_IDENTITY ?? config.bundle?.macOS?.signingIdentity;
if (!identity) {
  throw new Error("no signing identity: set APPLE_SIGNING_IDENTITY or bundle.macOS.signingIdentity");
}
const productName: string = config.productName ?? "Chord";
const timestampArgs = process.env.CODESIGN_NO_TIMESTAMP ? [] : ["--timestamp"];

await $`bun run build-native-host`.cwd(appDir);
await $`bun tauri build --bundles app`.cwd(appDir);

const bundleDir = path.join(srcTauri, "target/release/bundle/macos");
const app = path.join(bundleDir, `${productName}.app`);
const host = path.join(app, "Contents/MacOS/chord-native-host");
if (!fs.existsSync(host)) {
  throw new Error(`sidecar missing from bundle: ${host}`);
}

console.log("re-signing native host with NativeHost.entitlements");
await $`codesign --force --options runtime ${timestampArgs} --entitlements ${path.join(srcTauri, "NativeHost.entitlements")} --sign ${identity} ${host}`;
console.log("re-signing app bundle with Entitlements.plist");
await $`codesign --force --options runtime ${timestampArgs} --entitlements ${path.join(srcTauri, "Entitlements.plist")} --sign ${identity} ${app}`;
await $`codesign --verify --deep --strict --verbose=2 ${app}`;
await $`codesign -d --entitlements - ${host}`;

const outDir = path.join(srcTauri, "target/release/bundle/chord-release");
fs.rmSync(outDir, { recursive: true, force: true });
fs.mkdirSync(outDir, { recursive: true });

const dmg = path.join(outDir, `${productName}.dmg`);
console.log(`creating ${dmg}`);
await $`hdiutil create -volname ${productName} -srcfolder ${app} -ov -format UDZO ${dmg}`;

const updaterArchive = path.join(outDir, `${productName}.app.tar.gz`);
console.log(`creating updater artifact ${updaterArchive}`);
await $`tar -czf ${updaterArchive} -C ${bundleDir} ${productName}.app`;
if (process.env.TAURI_SIGNING_PRIVATE_KEY) {
  await $`bun tauri signer sign ${updaterArchive}`.cwd(appDir);
} else {
  console.log("TAURI_SIGNING_PRIVATE_KEY not set; updater artifact left unsigned");
}

if (process.env.APPLE_ID && process.env.APPLE_PASSWORD && process.env.APPLE_TEAM_ID) {
  console.log("notarizing");
  await $`xcrun notarytool submit ${dmg} --apple-id ${process.env.APPLE_ID} --password ${process.env.APPLE_PASSWORD} --team-id ${process.env.APPLE_TEAM_ID} --wait`;
  await $`xcrun stapler staple ${app}`;
  await $`xcrun stapler staple ${dmg}`;
} else {
  console.log("APPLE_ID/APPLE_PASSWORD/APPLE_TEAM_ID not set; skipping notarization");
}

console.log(`done: ${outDir}`);
