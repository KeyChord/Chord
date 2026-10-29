#!/usr/bin/env bun
/* eslint-disable no-console -- The dev terminal is an intentional output channel. */

import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const session = mkdtempSync(join(tmpdir(), "chord-dev-"));
const controlFile = join(session, "log-control");
const stateFile = join(session, "runtime.json");
const stopFile = join(session, "stop");
await Bun.write(controlFile, "");

const args = process.argv.slice(2);
const noVite = args.includes("--no-vite");
const cliArgs = args.filter((arg) => arg !== "--no-vite");
const appDir = join(import.meta.dir, "../../../../../../../apps/chord-tauri");
let runtime: typeof import("#runtime") | undefined;
if (process.platform === "darwin") {
  runtime = await import("#runtime");
  const rustc = Bun.spawn(["rustc", "-vV"], { stdout: "pipe", stderr: "inherit" });
  const host = (await new Response(rustc.stdout).text()).match(/^host: (.+)$/m)?.[1];
  if ((await rustc.exited) !== 0 || !host)
    throw new Error("Could not determine the Rust host target");
  const targetIndex = cliArgs.indexOf("--target");
  const target =
    targetIndex >= 0
      ? cliArgs[targetIndex + 1]
      : (cliArgs.find((arg) => arg.startsWith("--target="))?.slice(9) ?? host);
  if (!target?.endsWith("-apple-darwin"))
    throw new Error("The macOS dev launcher requires a macOS target");
  const runner = [process.execPath, Bun.fileURLToPath(import.meta.resolve("#runtime-launch"))];
  const separator = cliArgs.indexOf("--");
  cliArgs.splice(
    separator < 0 ? cliArgs.length : separator + 1,
    0,
    ...(separator < 0 ? ["--"] : []),
    "--config",
    `target.${target}.runner=${JSON.stringify(runner)}`,
  );
}

const tauri = Bun.spawn(
  [
    process.execPath,
    join(appDir, "_tauri.ts"),
    "dev",
    "--no-watch",
    "--config",
    "src-tauri/tauri.dev.conf.json",
    ...(noVite ? ["--config", '{"build":{"beforeDevCommand":null}}'] : []),
    ...cliArgs,
  ],
  {
    cwd: appDir,
    env: {
      ...process.env,
      CHORD_LOG_CONTROL_FILE: controlFile,
      CHORD_DEV_RUNTIME_STATE_FILE: stateFile,
      CHORD_DEV_RUNTIME_STOP_FILE: stopFile,
      RUST_BACKTRACE: "full",
    },
    stderr: "inherit",
    stdin: "ignore",
    stdout: "inherit",
  },
);

let sequence = 0;
let writes = Promise.resolve();

const sendTerminalLine = (line: string) => {
  const payload = `${Date.now()}-${sequence++}\t${line}`;
  writes = writes
    .then(async () => {
      await Bun.write(controlFile, payload);
      if (line === "log" || line.startsWith("log ")) {
        console.log(`[log-control] command sent: ${line}`);
      }
    })
    .catch((error) => {
      console.error(`[log-control] failed to send terminal input: ${error}`);
    });
};

const terminal = Bun.stdin.stream().getReader();
const decoder = new TextDecoder();
let terminalBuffer = "";

const readTerminal = async () => {
  while (true) {
    const { done, value } = await terminal.read();
    if (done) break;

    terminalBuffer += decoder.decode(value, { stream: true });
    let newlineIndex = terminalBuffer.indexOf("\n");
    while (newlineIndex >= 0) {
      const line = terminalBuffer.slice(0, newlineIndex).replace(/\r$/, "");
      terminalBuffer = terminalBuffer.slice(newlineIndex + 1);
      sendTerminalLine(line);
      newlineIndex = terminalBuffer.indexOf("\n");
    }
  }
};

void readTerminal().catch((error) => {
  console.error(`[log-control] failed to read terminal input: ${error}`);
});

let stopping: Promise<void> | undefined;
const forwardSignal = (signal: NodeJS.Signals) => {
  try {
    runtime?.signalSession(stateFile);
  } catch (error) {
    console.error(`[dev-runtime] ${error}`);
  }
  stopping ??= (async () => {
    await Bun.write(stopFile, "stop");
    // A Launch Services app is not a child of cargo. Stop our verified app
    // explicitly before terminating Tauri's build/run supervisor.
    await runtime?.stopSession(stateFile).catch((error) => console.error(`[dev-runtime] ${error}`));
    try {
      tauri.kill(signal);
    } catch {
      /* Tauri may already have exited. */
    }
  })();
};

process.on("SIGINT", () => forwardSignal("SIGINT"));
process.on("SIGTERM", () => forwardSignal("SIGTERM"));
process.on("SIGHUP", () => forwardSignal("SIGHUP"));

process.exitCode = await tauri.exited;
await stopping;
await runtime?.stopSession(stateFile).catch((error) => console.error(`[dev-runtime] ${error}`));
await terminal.cancel().catch(() => {});
await writes;
rmSync(session, { recursive: true, force: true });
