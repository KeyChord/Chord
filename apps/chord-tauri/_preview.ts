#!/usr/bin/env bun
import { runCommand } from "./commands.ts";

await runCommand(["vite", "preview", ...process.argv.slice(2)]);
