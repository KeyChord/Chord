#!/usr/bin/env bun
import { runCommand } from "./commands.ts";

await runCommand(["vite", ...process.argv.slice(2)]);
