#!/usr/bin/env bun
import { runCommand } from "./commands.ts";

await runCommand([
  "typeshare",
  "./src-tauri",
  "--lang=typescript",
  "--output-file=../../packages/dev/improve/chord/lib/typeshare/types.gen.ts",
  ...process.argv.slice(2),
]);
