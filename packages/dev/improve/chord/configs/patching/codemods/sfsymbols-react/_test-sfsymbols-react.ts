#!/usr/bin/env bun

const child = Bun.spawn(
  [
    "vpx",
    "codemod@latest",
    "jssg",
    "test",
    "-l",
    "json",
    "./codemod.ts",
    "./@fixtures",
    "--strictness",
    "loose",
    ...process.argv.slice(2),
  ],
  { cwd: import.meta.dir, stdio: ["inherit", "inherit", "inherit"] },
);
for (const signal of ["SIGINT", "SIGTERM", "SIGHUP"] as const) {
  process.on(signal, () => child.kill(signal));
}
process.exitCode = await child.exited;
