import { expect, test } from "bun:test";
import {
  ALL_APPS_PATHSLUG,
  chordsFileAppId,
  chordsFilePathslug,
  describeChordsFile,
  isValidAppId,
  isValidPackageDirname,
  newChordsFileContents,
} from "#personal-chords-file";

test("app files map to and from bundle IDs as the backend matches them", () => {
  expect(chordsFilePathslug("com.microsoft.VSCode")).toBe("chords/com/microsoft/VSCode/macos.toml");
  expect(chordsFilePathslug("  ")).toBe(ALL_APPS_PATHSLUG);
  expect(chordsFileAppId("chords/com/microsoft/VSCode/macos.toml")).toBe("com.microsoft.VSCode");
  for (const pathslug of [
    ALL_APPS_PATHSLUG,
    "chords/@keychord/chords-menu/chords/macos.toml",
    "chords/com/apple/finder/shared.toml",
  ]) {
    expect(chordsFileAppId(pathslug)).toBeUndefined();
  }
});

test("files are titled by the app they apply to", () => {
  expect(describeChordsFile(ALL_APPS_PATHSLUG)).toEqual({ title: "Every app" });
  expect(describeChordsFile("chords/com/apple/finder/macos.toml")).toEqual({
    title: "com.apple.finder",
    appId: "com.apple.finder",
  });
  expect(describeChordsFile("chords/shared.toml")).toEqual({ title: "shared.toml" });
});

test("only bundle-ID-shaped input becomes a path", () => {
  for (const appId of ["com.apple.finder", "dev.zed.Zed-Preview", "livesplit_one", "Terminal"]) {
    expect(isValidAppId(appId)).toBe(true);
  }
  for (const appId of ["", ".com", "com..apple", "com/apple", "../x", "@scope.pkg", "com.apple."]) {
    expect(isValidAppId(appId)).toBe(false);
  }
});

test("package folder names match what the backend accepts", () => {
  for (const dirname of ["default", "work", "my-chords_2", "v1.0"]) {
    expect(isValidPackageDirname(dirname)).toBe(true);
  }
  for (const dirname of ["", ".hidden", "-x", "a/b", "..", "with space", "x".repeat(101)]) {
    expect(isValidPackageDirname(dirname)).toBe(false);
  }
});

test("new files name their app and start with an empty chords table", () => {
  const contents = newChordsFileContents("com.apple.finder");
  expect(contents.startsWith('name = "com.apple.finder"\n')).toBe(true);
  expect(contents.endsWith("[chords]\n")).toBe(true);
  expect(newChordsFileContents("")).toContain('"/q" = {');
});
