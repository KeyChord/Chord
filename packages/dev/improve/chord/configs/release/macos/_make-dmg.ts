#!/usr/bin/env bun
/* eslint-disable no-console -- This CLI reports where the disk image went. */
import { makeDmg } from "#make-dmg";

const [app, output, icon] = process.argv.slice(2);
if (!app || !output) {
  console.error("Usage: vpx _make-dmg <app bundle> <output.dmg> [volume icon.icns]");
  process.exit(1);
}
await makeDmg(app, output, icon);
console.log(output);
