#!/usr/bin/env bun
import { runCommand } from "./commands.ts";

const args = process.argv.slice(2);
if (!args.some((arg) => arg === "--help" || arg === "-h")) {
  await runCommand(["tsc"]);
}
await runCommand(["vite", "build", ...args]);
