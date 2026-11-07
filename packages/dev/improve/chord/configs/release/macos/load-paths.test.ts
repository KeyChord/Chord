import { expect, test } from "bun:test";
import { parseLoadCommands, strayPaths } from "#load-paths";

const APP = "/Applications/Chord.app";
const exists = (path: string) =>
  path === `${APP}/Contents/Frameworks` || path === `${APP}/Contents/Frameworks/helper.dylib`;

/** Trimmed from `otool -arch all -l` of the 0.2.0 release's executable. */
const RELEASED_0_2_0 = `/Applications/Chord.app/Contents/MacOS/chord:
Load command 13
          cmd LC_LOAD_DYLIB
      cmdsize 88
         name /System/Library/Frameworks/Carbon.framework/Versions/A/Carbon (offset 24)
   time stamp 2 Wed Dec 31 19:00:02 1969
Load command 23
          cmd LC_LOAD_DYLIB
      cmdsize 56
         name @rpath/libbun_embed.dylib (offset 24)
Load command 37
          cmd LC_LOAD_DYLIB
      cmdsize 56
         name /usr/lib/libSystem.B.dylib (offset 24)
Load command 40
          cmd LC_RPATH
      cmdsize 56
         path /Users/runner/work/_temp/bun-embed (offset 12)
Load command 41
          cmd LC_RPATH
      cmdsize 48
         path @executable_path/../Frameworks (offset 12)
Load command 42
      cmd LC_FUNCTION_STARTS
  cmdsize 16
  dataoff 40000000
`;

test("reads rpaths and libraries from an otool listing", () => {
  expect(parseLoadCommands(RELEASED_0_2_0)).toEqual({
    rpaths: ["/Users/runner/work/_temp/bun-embed", "@executable_path/../Frameworks"],
    libraries: [
      "/System/Library/Frameworks/Carbon.framework/Versions/A/Carbon",
      "@rpath/libbun_embed.dylib",
      "/usr/lib/libSystem.B.dylib",
    ],
  });
});

test("the build machine's directory on 0.2.0's rpath is caught", () => {
  expect(
    strayPaths(APP, "Contents/MacOS/chord", parseLoadCommands(RELEASED_0_2_0), exists),
  ).toEqual([
    {
      file: "Contents/MacOS/chord",
      kind: "rpath",
      path: "/Users/runner/work/_temp/bun-embed",
      problem: "outside the app",
    },
  ]);
});

test("the app's own Frameworks and the system's libraries are fine", () => {
  const commands = {
    rpaths: ["@executable_path/../Frameworks", "/usr/lib/swift"],
    libraries: [
      "@rpath/libbun_embed.dylib",
      "@loader_path/helper.dylib",
      "/System/Library/Frameworks/AppKit.framework/Versions/C/AppKit",
    ],
  };
  expect(strayPaths(APP, "Contents/Frameworks/libbun_embed.dylib", commands, exists)).toEqual([]);
});

test("relative paths that climb out of the app, or lead nowhere, are caught", () => {
  const commands = {
    rpaths: ["@executable_path/../../../Frameworks", "@loader_path/../lib"],
    libraries: ["@executable_path/../Frameworks/gone.dylib", "/opt/homebrew/lib/libfoo.dylib"],
  };
  expect(
    strayPaths(APP, "Contents/MacOS/chord", commands, exists).map(({ path, problem }) => [
      path,
      problem,
    ]),
  ).toEqual([
    ["@executable_path/../../../Frameworks", "outside the app"],
    ["@loader_path/../lib", "missing"],
    ["@executable_path/../Frameworks/gone.dylib", "missing"],
    ["/opt/homebrew/lib/libfoo.dylib", "outside the app"],
  ]);
});
