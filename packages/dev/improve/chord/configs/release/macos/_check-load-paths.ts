#!/usr/bin/env bun
/* eslint-disable no-console -- This CLI reports each app's result. */
import { describeStrayLoadPath, strayLoadPaths } from "#load-paths";

const apps = process.argv.slice(2);
if (apps.length === 0) {
  console.error("Usage: vpx _check-load-paths <app>...");
  console.error("  fails if an app could load code from outside itself and the system");
  process.exit(1);
}

let clean = true;
for (const app of apps) {
  const stray = await strayLoadPaths(app);
  if (stray.length === 0) {
    console.log(`${app}: loads only from itself and the system`);
    continue;
  }
  clean = false;
  console.error(`${app} could load code from outside itself, and XProtect won't open it:`);
  for (const path of stray) console.error(`  ${describeStrayLoadPath(path)}`);
}
process.exit(clean ? 0 : 1);
