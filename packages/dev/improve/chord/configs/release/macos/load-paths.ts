import { existsSync } from "node:fs";
import { dirname, join, relative, resolve, sep } from "node:path";
import { $ } from "bun";

/** Where macOS keeps its own libraries, on the sealed system volume. */
const SYSTEM_PREFIXES = ["/System/Library/", "/usr/lib/"];

/** The first four bytes of a Mach-O file, thin or universal, either byte order. */
const MACH_O_MAGICS = new Set([
  0xfeedface, 0xfeedfacf, 0xcefaedfe, 0xcffaedfe, 0xcafebabe, 0xcafebabf,
]);

/** The commands in an `otool -l` listing that tell dyld where to look. */
export type LoadCommands = { rpaths: string[]; libraries: string[] };

/** A place a Mach-O file in the app would load code from that the app doesn't control. */
export type StrayLoadPath = {
  /** The Mach-O file, relative to the app. */
  file: string;
  kind: "rpath" | "library";
  path: string;
  problem: "outside the app" | "missing";
};

const LIBRARY_COMMANDS = new Set([
  "LC_LOAD_DYLIB",
  "LC_LOAD_WEAK_DYLIB",
  "LC_REEXPORT_DYLIB",
  "LC_LAZY_LOAD_DYLIB",
  "LC_LOAD_UPWARD_DYLIB",
]);

export function parseLoadCommands(listing: string): LoadCommands {
  const commands: LoadCommands = { rpaths: [], libraries: [] };
  let command = "";
  for (const line of listing.split("\n")) {
    const field = /^\s*(cmd|name|path)\s+(.*?)(?:\s+\(offset \d+\))?$/.exec(line);
    if (!field) continue;
    const [, key, value] = field;
    if (key === "cmd") command = value!;
    else if (key === "path" && command === "LC_RPATH") commands.rpaths.push(value!);
    else if (key === "name" && LIBRARY_COMMANDS.has(command)) commands.libraries.push(value!);
  }
  return commands;
}

/**
 * The rpaths and libraries of one Mach-O file in `app` that point outside both the app and the
 * system. Libraries found through `@rpath/` are judged by the rpaths.
 */
export function strayPaths(
  app: string,
  file: string,
  { rpaths, libraries }: LoadCommands,
  exists: (path: string) => boolean = existsSync,
): StrayLoadPath[] {
  const appRoot = resolve(app);
  const executableDir = join(appRoot, "Contents/MacOS");
  const where = (path: string): StrayLoadPath["problem"] | undefined => {
    if (SYSTEM_PREFIXES.some((prefix) => path.startsWith(prefix))) return undefined;
    let resolved: string;
    if (path.startsWith("@executable_path/")) {
      resolved = resolve(executableDir, path.slice("@executable_path/".length));
    } else if (path.startsWith("@loader_path/")) {
      resolved = resolve(dirname(resolve(appRoot, file)), path.slice("@loader_path/".length));
    } else {
      return "outside the app";
    }
    if (resolved !== appRoot && !resolved.startsWith(appRoot + sep)) return "outside the app";
    return exists(resolved) ? undefined : "missing";
  };
  const stray: StrayLoadPath[] = [];
  for (const path of rpaths) {
    const problem = where(path);
    if (problem) stray.push({ file, kind: "rpath", path, problem });
  }
  for (const path of libraries) {
    if (path.startsWith("@rpath/")) continue;
    const problem = where(path);
    if (problem) stray.push({ file, kind: "library", path, problem });
  }
  return stray;
}

async function isMachO(path: string) {
  const head = await Bun.file(path).slice(0, 4).bytes();
  if (head.length < 4) return false;
  return MACH_O_MAGICS.has(new DataView(head.buffer, head.byteOffset, 4).getUint32(0));
}

/**
 * Every rpath and library in the app's Mach-O files that leads outside the app and the system.
 * dyld loads whatever it finds at such a path, and Chord turns library validation off, so whoever
 * can create that directory can run code as Chord. XProtect refuses to open an app like that
 * ("Apple could not verify ... is free of malware"), even a notarized one, which is how a CI
 * build directory on the rpath once kept 0.2.0 from opening.
 */
export async function strayLoadPaths(app: string): Promise<StrayLoadPath[]> {
  const appRoot = resolve(app);
  const stray: StrayLoadPath[] = [];
  const files = new Bun.Glob("Contents/**/*").scan({ cwd: appRoot, onlyFiles: true });
  for await (const file of files) {
    const path = join(appRoot, file);
    if (!(await isMachO(path))) continue;
    const listing = await $`otool -arch all -l ${path}`.text();
    stray.push(...strayPaths(appRoot, relative(appRoot, path), parseLoadCommands(listing)));
  }
  return stray;
}

export function describeStrayLoadPath({ file, kind, path, problem }: StrayLoadPath) {
  return `${file}: ${kind} ${path} is ${problem}`;
}
