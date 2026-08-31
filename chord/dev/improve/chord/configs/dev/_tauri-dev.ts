#!/usr/bin/env bun

export {};

const tempDirectory = process.env.TMPDIR ?? process.env.TEMP ?? process.env.TMP ?? "/tmp";
const controlFile = `${tempDirectory.replace(/[/\\]$/, "")}/chord-log-control-${process.pid}`;
await Bun.write(controlFile, "");

const tauri = Bun.spawn(
  [
    "bun",
    "run",
    "tauri",
    "dev",
    "--no-watch",
    "--config",
    "src-tauri/tauri.dev.conf.json",
    "--config",
    '{"build":{"beforeDevCommand":null}}',
  ],
  {
    cwd: process.cwd(),
    env: {
      ...process.env,
      CHORD_LOG_CONTROL_FILE: controlFile,
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

const forwardSignal = (signal: NodeJS.Signals) => {
  try {
    tauri.kill(signal);
  } catch {
    // The Tauri process may already have exited.
  }
};

process.on("SIGINT", () => forwardSignal("SIGINT"));
process.on("SIGTERM", () => forwardSignal("SIGTERM"));

process.exitCode = await tauri.exited;
await terminal.cancel().catch(() => {});
await writes;
await Bun.file(controlFile)
  .delete()
  .catch(() => {});
