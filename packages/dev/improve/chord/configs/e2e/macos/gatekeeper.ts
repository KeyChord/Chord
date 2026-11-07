/**
 * What Apple's checks make of a Chord download. Notarization isn't all of it: with library
 * validation off, XProtect also refuses an app that could load code from outside itself, which
 * kept the notarized 0.2.0 from opening ("Apple could not verify “Chord” is free of malware").
 * A VM's macOS may not run every check a user's Mac does, so the host checks the download too.
 */
import { mkdtempSync, readdirSync, rmdirSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  describeStrayLoadPath,
  strayLoadPaths,
} from "@chord/dev.improve.chord.configs.release.macos/load-paths";
import { assessNotarization } from "@chord/dev.improve.chord.configs.release.macos/notarize";
import { $ } from "bun";
import { type Guest, shellQuote } from "#guest";

/**
 * Checks a disk image and the app in it on this Mac. Nothing in the app may load code from outside
 * it, and both are notarized with their tickets stapled, or neither is. Throws if
 * `requireNotarized` and they aren't. Resolves to whether Gatekeeper will check them.
 */
export async function checkDownload(
  dmg: string,
  { requireNotarized = false } = {},
): Promise<boolean> {
  const mount = mkdtempSync(join(tmpdir(), "chord-e2e-dmg-"));
  await $`hdiutil attach -nobrowse -readonly -noautoopen -mountpoint ${mount} ${dmg}`.quiet();
  try {
    const apps = readdirSync(mount).filter((name) => name.endsWith(".app"));
    if (apps.length !== 1) throw new Error(`${dmg} should hold one app, not ${apps.length}`);
    const app = join(mount, apps[0]!);
    const stray = await strayLoadPaths(app);
    if (stray.length > 0) {
      throw new Error(
        `the app in ${dmg} could load code from outside itself, so XProtect won't open it:\n${stray.map(describeStrayLoadPath).join("\n")}`,
      );
    }
    const [image, inside] = await Promise.all([assessNotarization(dmg), assessNotarization(app)]);
    if (image.notarized && inside.notarized) return true;
    if (requireNotarized || image.notarized || inside.notarized) {
      throw new Error(
        `${dmg} isn't notarized throughout.\nDisk image:\n${image.details}\nApp:\n${inside.details}`,
      );
    }
    return false;
  } finally {
    const detached = await $`hdiutil detach ${mount}`.quiet().nothrow();
    if (detached.exitCode === 0) rmdirSync(mount);
  }
}

/** Gatekeeper's verdict in the VM, from the `spctl` every Mac has. */
export async function guestAssessment(guest: Guest, path: string): Promise<string> {
  const kind = path.endsWith(".dmg") ? "-t open --context context:primary-signature" : "-t exec";
  const { stdout, stderr } = await guest.run(`spctl -a -vv ${kind} ${shellQuote(path)}`, {
    check: false,
  });
  return `${stdout}${stderr}`.trim();
}

/** The guest's clock, as `log show --start` reads it. */
export async function guestTime(guest: Guest): Promise<string> {
  return (await guest.run("date '+%Y-%m-%d %H:%M:%S'")).stdout.trim();
}

/** Gatekeeper's and XProtect's verdicts, which macOS logs at its default level and keeps. */
const VERDICTS = [
  '(subsystem == "com.apple.securityd" AND category == "gk")',
  '(process == "XprotectService" AND eventMessage CONTAINS "failed on")',
  '(process == "CoreServicesUIAgent" AND eventMessage CONTAINS "XProtect finished")',
].join(" OR ");

/** What Gatekeeper and XProtect logged in the guest since `since` (from `guestTime`). */
export async function guestVerdicts(guest: Guest, since: string): Promise<string> {
  const { stdout } = await guest.run(
    `/usr/bin/log show --style compact --start ${shellQuote(since)} --predicate ${shellQuote(VERDICTS)}`,
  );
  return stdout;
}

/**
 * What stopped, or would stop, an app from opening, from a log of `guestVerdicts` taken over its
 * first launch: a refusal by Gatekeeper or XProtect, or XProtect never having looked at it.
 */
export function launchProblems(log: string): string[] {
  const problems = log
    .split("\n")
    .filter((line) => /failed on (?:rPath|load)Cmd|\ballow=0\b/.test(line))
    .map((line) => line.trim());
  // Each entry starts with its timestamp; XProtect's verdict on an app runs over several lines.
  const analyses = log
    .split(/\n(?=\d{4}-\d\d-\d\d )/)
    .filter((entry) => entry.includes("XProtect finished executable analysis"))
    .filter((entry) => /operation = "operation:execute"/.test(entry));
  if (analyses.length === 0) problems.push("XProtect never analyzed the app as it opened");
  for (const analysis of analyses) {
    const malware = /XProtectMalwareType = (\d+);/.exec(analysis)?.[1];
    if (malware !== undefined && malware !== "0") {
      problems.push(`XProtect refused it (XProtectMalwareType = ${malware})`);
    }
    if (!/XProtectIsNotarized = 1;/.test(analysis)) {
      problems.push("XProtect didn't find it notarized");
    }
  }
  return problems;
}

const XPROTECT_SERVICE =
  "/System/Library/PrivateFrameworks/XprotectFramework.framework/Versions/A/XPCServices/XprotectService.xpc/Contents/MacOS/XprotectService";

/** The guest's macOS and XProtect versions, and whether it refuses apps over their load paths. */
export async function guestChecks(guest: Guest): Promise<string> {
  const { stdout } = await guest.run(`
    echo "macOS $(sw_vers -productVersion) ($(sw_vers -buildVersion)), XProtect $(xprotect version 2>/dev/null || echo unknown)"
    if grep -qs 'failed on rPathCmd' ${XPROTECT_SERVICE}; then
      echo "This macOS refuses apps whose rpath leads outside them."
    else
      echo "This macOS doesn't check load paths; only the host's check covers them."
    fi
  `);
  return stdout.trim();
}
