#!/usr/bin/env bun
import { chmod } from "node:fs/promises";
import { join } from "node:path";

const platform = process.platform;
if (platform !== "linux" && platform !== "win32") {
  console.error("The Bun sidecar is used only on Linux and Windows.");
  process.exit(1);
}
const rustc = Bun.spawnSync(["rustc", "-vV"], { stderr: "inherit", stdout: "pipe" });
const host = rustc.stdout.toString().match(/^host: (\S+)\s*$/m)?.[1];
const target = process.argv[2] ?? host;
if (rustc.exitCode !== 0 || !host || target !== host) {
  throw new Error(`The Bun sidecar must be staged on the target host (${target ?? "unknown"})`);
}
const suffix = platform === "win32" ? ".exe" : "";
const destination = join(import.meta.dir, "src-tauri", "binaries", `chord-bun-${target}${suffix}`);
await Bun.write(destination, Bun.file(process.execPath));
if (platform === "linux") await chmod(destination, 0o755);
