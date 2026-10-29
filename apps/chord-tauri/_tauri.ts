#!/usr/bin/env bun
import { runCommand } from "./commands.ts";

await runCommand(["tauri", ...process.argv.slice(2)]);
