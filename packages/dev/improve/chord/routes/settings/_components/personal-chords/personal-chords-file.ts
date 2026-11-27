import { desktopPlatform } from "@chord/dev.improve.chord.lib.state/desktop-platform";

/** Chords in this file work in every app (when they start with a symbol). */
const filename = `${desktopPlatform}.toml`;
export const ALL_APPS_PATHSLUG = `chords/${filename}`;

/**
 * Mirrors the backend's `chords_file_app_id`: `chords/com/apple/finder/macos.toml` applies to
 * `com.apple.finder`. Root and bundled (`chords/@scope/…`) files apply to no app.
 */
export function chordsFileAppId(pathslug: string): string | undefined {
  const match = /^chords\/(.+)\/(?:macos|linux|windows)\.toml$/.exec(
    pathslug.replaceAll("\\", "/"),
  );
  if (!match?.[1] || match[1].startsWith("@")) return undefined;
  return match[1].replaceAll("/", ".");
}

/** The file for an app's chords, or for chords that work in every app when `appId` is empty. */
export function chordsFilePathslug(appId: string): string {
  const trimmed = appId.trim();
  return trimmed === "" ? ALL_APPS_PATHSLUG : `chords/${trimmed.replaceAll(".", "/")}/${filename}`;
}

/** Bundle IDs use letters, digits, hyphens, and periods; underscores appear in claimed IDs. */
export function isValidAppId(appId: string): boolean {
  return /^[A-Za-z0-9_-]+(?:\.[A-Za-z0-9_-]+)*$/.test(appId.trim());
}

/** Mirrors the backend's check for folder names in `~/.chord/packages`. */
export function isValidPackageDirname(dirname: string): boolean {
  return /^[A-Za-z0-9][A-Za-z0-9._-]{0,99}$/.test(dirname);
}

export function describeChordsFile(pathslug: string): { title: string; appId?: string } {
  if (pathslug.replaceAll("\\", "/") === ALL_APPS_PATHSLUG) return { title: "Every app" };
  const appId = chordsFileAppId(pathslug);
  return appId ? { title: appId, appId } : { title: pathslug.replace(/^chords\//, "") };
}

export function newChordsFileContents(appId: string): string {
  const trimmed = appId.trim();
  const launch =
    desktopPlatform === "macos"
      ? "open -a Notes"
      : desktopPlatform === "linux"
        ? "xdg-open ."
        : "start .";
  const shortcut = desktopPlatform === "macos" ? "cmd+n" : "ctrl+n";
  if (trimmed === "") {
    return `name = "Personal"

# Chords that start with a symbol work in every app, for example:
#   "/n" = { name = "Open", shell = "${launch}" }
[chords]
`;
  }
  return `name = ${JSON.stringify(trimmed)}

# Chords for ${trimmed}. Each key is the sequence you type, for example:
#   nw = { name = "New Window", shortcut = "${shortcut}" }
[chords]
`;
}
