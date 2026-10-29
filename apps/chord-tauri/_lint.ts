#!/usr/bin/env bun
import { REPO_ROOT, runCommand } from "./commands.ts";

await runCommand(
  [
    "vp",
    "lint",
    "--fix",
    "apps/chord-tauri",
    "packages/dev/improve/chord",
    ...process.argv.slice(2),
  ],
  { cwd: REPO_ROOT },
);
