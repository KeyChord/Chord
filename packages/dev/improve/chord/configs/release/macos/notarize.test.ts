import { afterEach, beforeEach, expect, test } from "bun:test";
import { chmodSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { notarize, notaryCredentials, saysNotarized } from "#notarize";

const CREDENTIALS = ["--keychain-profile", "test"];

let dir: string;
let originalPath: string | undefined;

/**
 * Stands in for Apple: answers `submit` with the given output, records every call, and fails the
 * first `stapleFailures` attempts to staple.
 */
function fakeXcrun(submission: string, log = "", stapleFailures = 0) {
  const script = `#!/bin/sh
echo "$*" >> "${dir}/calls"
case "$2" in
  submit) cat <<'EOF'
${submission}
EOF
  ;;
  log) cat <<'EOF'
${log}
EOF
  ;;
  staple)
    tried=$(grep -c ' staple ' "${dir}/calls")
    if [ "$tried" -le ${stapleFailures} ]; then
      echo "The network connection was lost."
      exit 68
    fi
  ;;
esac
`;
  const xcrun = join(dir, "bin/xcrun");
  writeFileSync(xcrun, script);
  chmodSync(xcrun, 0o755);
}

const calls = () => readFileSync(join(dir, "calls"), "utf8").trim().split("\n");

beforeEach(() => {
  dir = mkdtempSync(join(tmpdir(), "chord-notarize-test-"));
  mkdirSync(join(dir, "bin"));
  originalPath = process.env.PATH;
  process.env.PATH = `${join(dir, "bin")}:${originalPath}`;
});

afterEach(() => {
  process.env.PATH = originalPath;
  rmSync(dir, { recursive: true, force: true });
});

test("only Apple's notarized verdict counts, not the word appearing in it", () => {
  expect(saysNotarized("x: accepted\nsource=Notarized Developer ID\norigin=Leon")).toBe(true);
  expect(saysNotarized("x: rejected\nsource=Unnotarized Developer ID\norigin=Leon")).toBe(false);
  expect(saysNotarized("x: rejected\nsource=no usable signature")).toBe(false);
});

test("credentials are a saved profile or a complete Apple ID", () => {
  expect(notaryCredentials({ CHORD_NOTARY_PROFILE: "chord" })).toEqual([
    "--keychain-profile",
    "chord",
  ]);
  expect(notaryCredentials({ APPLE_ID: "a", APPLE_PASSWORD: "p", APPLE_TEAM_ID: "t" })).toEqual([
    "--apple-id",
    "a",
    "--password",
    "p",
    "--team-id",
    "t",
  ]);
  expect(notaryCredentials({ APPLE_ID: "a", APPLE_PASSWORD: "p" })).toBeUndefined();
  expect(notaryCredentials({})).toBeUndefined();
});

test("an accepted disk image is sent as it is and gets its ticket", async () => {
  fakeXcrun(`{"id":"1","status":"Accepted"}`);
  const dmg = join(dir, "Chord.dmg");
  writeFileSync(dmg, "");
  await notarize(dmg, CREDENTIALS);
  expect(calls()[0]).toContain(`notarytool submit ${dmg} --keychain-profile test --wait`);
  expect(calls()[1]).toBe(`stapler staple ${dmg}`);
});

test("an app is sent as an archive, and the ticket goes on the app", async () => {
  fakeXcrun(`{"id":"1","status":"Accepted"}`);
  const app = join(dir, "Chord.app");
  mkdirSync(join(app, "Contents"), { recursive: true });
  await notarize(app, CREDENTIALS);
  expect(calls()[0]).toMatch(/notarytool submit \S+\/Chord\.app\.zip /);
  expect(calls()[1]).toBe(`stapler staple ${app}`);
});

test("stapling survives a dropped connection", async () => {
  fakeXcrun(`{"id":"1","status":"Accepted"}`, "", 2);
  const dmg = join(dir, "Chord.dmg");
  writeFileSync(dmg, "");
  await notarize(dmg, CREDENTIALS, 0);
  expect(calls().filter((call) => call.startsWith("stapler"))).toHaveLength(3);
});

test("stapling that keeps failing fails the release, and says why", async () => {
  fakeXcrun(`{"id":"1","status":"Accepted"}`, "", 99);
  const dmg = join(dir, "Chord.dmg");
  writeFileSync(dmg, "");
  await expect(notarize(dmg, CREDENTIALS, 0)).rejects.toThrow("network connection was lost");
  expect(calls().filter((call) => call.startsWith("stapler"))).toHaveLength(5);
});

test("a rejection fails with Apple's reason and staples nothing", async () => {
  fakeXcrun(`{"id":"42","status":"Invalid"}`, "The signature does not include a secure timestamp.");
  const dmg = join(dir, "Chord.dmg");
  writeFileSync(dmg, "");
  const attempt = notarize(dmg, CREDENTIALS);
  await expect(attempt).rejects.toThrow("Chord.dmg (Invalid)");
  await expect(attempt).rejects.toThrow("secure timestamp");
  expect(calls().some((call) => call.startsWith("stapler"))).toBe(false);
});

test("no answer from Apple fails with what notarytool printed", async () => {
  fakeXcrun("Error: HTTP status code: 401. Invalid credentials.");
  const dmg = join(dir, "Chord.dmg");
  writeFileSync(dmg, "");
  await expect(notarize(dmg, CREDENTIALS)).rejects.toThrow("Invalid credentials");
  expect(calls().some((call) => call.startsWith("stapler"))).toBe(false);
});
