import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";
import { $ } from "bun";

const DMGBUILD = "dmgbuild==1.6.5";
const BACKGROUND = join(import.meta.dir, "@assets/dmg/background.tiff");

/**
 * Makes the disk image people download, which opens to a window with an arrow from Chord to the
 * Applications folder and says what to do. Tauri's bundler only arranges that window when Finder
 * can be scripted, so on CI it leaves a blank window with the folder before the app.
 *
 * dmgbuild writes the arrangement directly, with no Finder involved. It has to do so for each
 * disk image: the reference to the background image names the volume it was made on, so Finder
 * shows no background with a copy of the arrangement from another disk image.
 */
/** Signs the disk image with whatever signed the app in it, which may be nothing at all. */
async function signLikeApp(app: string, dmg: string) {
  const { stderr } = await $`codesign -dvv ${app}`.nothrow().quiet();
  const identity = /^Authority=(.+)$/m.exec(stderr.toString())?.[1];
  // Apple won't notarize a signature without a trusted timestamp.
  const timestamp = process.env.CODESIGN_NO_TIMESTAMP ? [] : ["--timestamp"];
  if (identity) await $`codesign --force ${timestamp} --sign ${identity} ${dmg}`.quiet();
}

export async function makeDmg(app: string, output: string, icon?: string) {
  const name = basename(app);
  const work = mkdtempSync(join(tmpdir(), "chord-dmg-"));
  try {
    await $`python3 -m venv ${join(work, "venv")}`.quiet();
    await $`${join(work, "venv/bin/pip")} install --quiet --disable-pip-version-check ${DMGBUILD}`.quiet();
    const settings = join(work, "settings.py");
    await Bun.write(
      settings,
      `
format = "UDZO"
compression_level = 9
files = [${JSON.stringify(app)}]
symlinks = {"Applications": "/Applications"}
${icon ? `icon = ${JSON.stringify(icon)}` : ""}
background = ${JSON.stringify(BACKGROUND)}
icon_locations = {${JSON.stringify(name)}: (170, 170), "Applications": (490, 170)}
window_rect = ((200, 120), (660, 400))
default_view = "icon-view"
icon_size = 128
text_size = 14
show_status_bar = False
show_tab_view = False
show_toolbar = False
show_pathbar = False
show_sidebar = False
sidebar_width = 0
`,
    );
    const volume = basename(app, ".app");
    await $`${join(work, "venv/bin/dmgbuild")} -s ${settings} ${volume} ${output}`.quiet();
    await signLikeApp(app, output);
  } finally {
    rmSync(work, { recursive: true, force: true });
  }
}
