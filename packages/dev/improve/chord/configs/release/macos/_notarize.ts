#!/usr/bin/env bun
/* eslint-disable no-console -- This CLI reports each file's result. */
import { assessNotarization, notarize, notaryCredentials, staple } from "#notarize";
import { parseArgs } from "node:util";

const { values, positionals: paths } = parseArgs({
  allowPositionals: true,
  options: { check: { type: "boolean" }, staple: { type: "boolean" } },
});
if (paths.length === 0) {
  console.error("Usage: vpx _notarize [--check | --staple] <app or dmg>...");
  console.error("  --check   only say whether each one is notarized, and fail if one isn't");
  console.error("  --staple  only put on the ticket of something Apple has already notarized");
  process.exit(1);
}

if (values.check) {
  let allNotarized = true;
  for (const path of paths) {
    const { notarized, details } = await assessNotarization(path);
    console.log(`${path}: ${notarized ? "notarized" : `not notarized\n${details}`}`);
    allNotarized &&= notarized;
  }
  process.exit(allNotarized ? 0 : 1);
}

if (values.staple) {
  for (const path of paths) {
    await staple(path);
    console.log(`${path}: stapled`);
  }
  process.exit(0);
}

const credentials = notaryCredentials();
if (!credentials) {
  console.error(
    "no notary credentials: set CHORD_NOTARY_PROFILE, or APPLE_ID, APPLE_PASSWORD and APPLE_TEAM_ID",
  );
  process.exit(1);
}
for (const path of paths) {
  await notarize(path, credentials);
  console.log(`${path}: notarized`);
}
