import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";
import { $ } from "bun";

/**
 * How notarytool signs in: a profile saved once with `xcrun notarytool store-credentials`, or the
 * Apple ID that CI has. Undefined when neither is complete.
 */
export function notaryCredentials(env: Record<string, string | undefined> = process.env) {
  if (env.CHORD_NOTARY_PROFILE) return ["--keychain-profile", env.CHORD_NOTARY_PROFILE];
  const { APPLE_ID, APPLE_PASSWORD, APPLE_TEAM_ID } = env;
  if (!APPLE_ID || !APPLE_PASSWORD || !APPLE_TEAM_ID) return undefined;
  return ["--apple-id", APPLE_ID, "--password", APPLE_PASSWORD, "--team-id", APPLE_TEAM_ID];
}

function parseSubmission(output: string): { id?: string; status?: string } {
  try {
    return JSON.parse(output);
  } catch {
    return {};
  }
}

/** What `stapler` exits with when it loses its connection to Apple. */
const CONNECTION_LOST = 68;
const STAPLER_ATTEMPTS = 24;

/**
 * Both actions reach Apple's ticket service, and from GitHub's Intel runners most connections to
 * it drop, seven in a row at worst so far. Only that is worth another try: a file with no ticket
 * fails right away.
 */
async function stapler(action: "staple" | "validate", path: string, retryMs: number) {
  for (let attempt = 1; ; attempt++) {
    const result = await $`xcrun stapler ${action} ${path}`.nothrow().quiet();
    if (result.exitCode !== CONNECTION_LOST || attempt === STAPLER_ATTEMPTS) return result;
    console.warn(`stapler ${action} lost its connection to Apple; trying again`);
    await Bun.sleep(retryMs);
  }
}

/** Puts the ticket Apple already has for this on it, or fails saying why. */
export async function staple(path: string, retryMs = 15_000) {
  const stapled = await stapler("staple", path, retryMs);
  if (stapled.exitCode !== 0) {
    throw new Error(
      `could not staple ${basename(path)} (exit ${stapled.exitCode}):\n${stapled.stdout}${stapled.stderr}`,
    );
  }
}

/**
 * Sends an app or a disk image to Apple and staples the ticket to it, so that it opens on a Mac
 * with no connection. A disk image gets its own: `spctl` assesses it apart from the app in it, and
 * rejects one that is signed but that Apple hasn't seen, even though macOS 27 still mounts that.
 */
export async function notarize(path: string, credentials: string[], retryMs = 15_000) {
  const work = mkdtempSync(join(tmpdir(), "chord-notarize-"));
  try {
    // notarytool takes a disk image as it is, but an app only in an archive.
    const upload = path.endsWith(".app") ? join(work, `${basename(path)}.zip`) : path;
    if (upload !== path) await $`ditto -c -k --keepParent ${path} ${upload}`.quiet();

    const submit =
      await $`xcrun notarytool submit ${upload} ${credentials} --wait --output-format json`
        .nothrow()
        .quiet();
    // Go by Apple's answer, not by the command having run.
    const { id, status } = parseSubmission(submit.stdout.toString());
    if (status !== "Accepted") {
      const reason = id
        ? await $`xcrun notarytool log ${id} ${credentials}`.nothrow().text()
        : `${submit.stderr}${submit.stdout}`;
      throw new Error(`Apple did not accept ${basename(path)} (${status ?? "no answer"}):\n${reason}`);
    }
    await staple(path, retryMs);
  } finally {
    rmSync(work, { recursive: true, force: true });
  }
}

/** Whether `spctl` output says Apple notarized it. "Unnotarized Developer ID" contains the word. */
export function saysNotarized(assessment: string) {
  return /^source=Notarized Developer ID$/m.test(assessment);
}

/** Whether this is accepted as notarized with the ticket on it, and what was said about it. */
export async function assessNotarization(path: string, retryMs = 15_000) {
  // A disk image is assessed as something to open, and an app as something to run.
  const kind = path.endsWith(".dmg") ? ["-t", "open", "--context", "context:primary-signature"] : [];
  const assessment = await $`spctl -a -vv ${kind} ${path}`.nothrow().quiet();
  const stapled = await stapler("validate", path, retryMs);
  const gatekeeper = await $`spctl --status`.nothrow().quiet();
  const verdict = `${assessment.stdout}${assessment.stderr}`.trim();
  return {
    notarized: saysNotarized(verdict) && stapled.exitCode === 0,
    details: [
      `Gatekeeper: ${gatekeeper.stdout}${gatekeeper.stderr}`.trim(),
      verdict,
      `stapler (exit ${stapled.exitCode}): ${stapled.stdout}${stapled.stderr}`.trim(),
    ].join("\n"),
  };
}

export async function isNotarized(path: string) {
  return (await assessNotarization(path)).notarized;
}
