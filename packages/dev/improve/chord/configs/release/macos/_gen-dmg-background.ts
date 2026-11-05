#!/usr/bin/env bun
/* eslint-disable no-console -- This CLI reports where the image went. */
/** Draws the installer window's background, for when its design changes. */
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { $ } from "bun";

const background = join(import.meta.dir, "@assets/dmg/background.tiff");
const work = mkdtempSync(join(tmpdir(), "chord-dmg-background-"));
try {
  const font = Bun.resolveSync(
    "@fontsource-variable/inter/files/inter-latin-wght-normal.woff2",
    import.meta.dir,
  );
  const draw = join(work, "draw");
  await $`xcrun swiftc -O ${join(import.meta.dir, "dmg-background.swift")} -o ${draw}`;
  await $`${draw} ${join(work, "1x.png")} 1 ${font}`;
  await $`${draw} ${join(work, "2x.png")} 2 ${font}`;
  await $`tiffutil -cathidpicheck ${join(work, "1x.png")} ${join(work, "2x.png")} -out ${background}`.quiet();
  console.log(background);
} finally {
  rmSync(work, { recursive: true, force: true });
}
