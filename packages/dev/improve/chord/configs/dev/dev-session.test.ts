import { expect, test } from "bun:test";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { prepareDevSession } from "#dev-session";

const mprocsBin = `${import.meta.dir}/node_modules/.bin/mprocs`;

function unusedPort(): number {
  const server = Bun.listen({ hostname: "127.0.0.1", port: 0, socket: { data() {} } });
  const port = server.port;
  server.stop(true);
  return port;
}

async function startListener(hostname = "127.0.0.1", ignoreTerm = false) {
  const child = Bun.spawn(
    [
      process.execPath,
      "-e",
      `${ignoreTerm ? 'process.on("SIGTERM", () => {});' : ""}
			const server = Bun.serve({ hostname: ${JSON.stringify(hostname)}, port: 0,
				fetch: () => new Response("ready") });
			console.log(server.port);`,
    ],
    { stdout: "pipe", stderr: "inherit" },
  );
  const reader = child.stdout.getReader();
  const { value } = await reader.read();
  reader.releaseLock();
  const port = Number(new TextDecoder().decode(value).trim());
  if (!port) {
    child.kill();
    throw new Error("Test listener did not publish a port");
  }
  return { child, port };
}

test("normal startup refuses occupied ports and preserves the listener", async () => {
  const { child, port } = await startListener();
  try {
    await expect(
      prepareDevSession({
        force: false,
        mprocsBin,
        serverPort: unusedPort(),
        servicePorts: [port],
      }),
    ).rejects.toThrow("Re-run with --force");
    expect(child.exitCode).toBeNull();
    expect(await (await fetch(`http://127.0.0.1:${port}`)).text()).toBe("ready");
  } finally {
    child.kill("SIGKILL");
    await child.exited;
  }
});

test("force frees an orphaned Vite listener and leaves other listeners running", async () => {
  const orphan = await startListener();
  const other = await startListener();
  try {
    await prepareDevSession({
      force: true,
      mprocsBin,
      serverPort: unusedPort(),
      servicePorts: [orphan.port],
    });
    await orphan.child.exited;
    const replacement = Bun.listen({
      hostname: "127.0.0.1",
      port: orphan.port,
      socket: { data() {} },
    });
    replacement.stop(true);
    expect(other.child.exitCode).toBeNull();
  } finally {
    orphan.child.kill("SIGKILL");
    other.child.kill("SIGKILL");
    await Promise.all([orphan.child.exited, other.child.exited]);
  }
});

test("force handles IPv6 listeners that ignore SIGTERM", async () => {
  const { child, port } = await startListener("::1", true);
  try {
    await prepareDevSession({
      force: true,
      mprocsBin,
      serverPort: unusedPort(),
      servicePorts: [port],
    });
    await child.exited;
    expect(child.signalCode).toBe("SIGKILL");
    const replacement = Bun.listen({ hostname: "::1", port, socket: { data() {} } });
    replacement.stop(true);
  } finally {
    child.kill("SIGKILL");
    await child.exited;
  }
}, 10_000);

test("force quits a real mprocs session through its control server", async () => {
  const directory = mkdtempSync(join(tmpdir(), "chord-dev-session-test-"));
  const config = join(directory, "mprocs.yaml");
  await Bun.write(config, "procs: {}\n");
  const serverPort = unusedPort();
  const child = Bun.spawn([mprocsBin, "--config", config, "--server", `127.0.0.1:${serverPort}`], {
    cwd: directory,
    terminal: { cols: 80, rows: 24, data() {} },
  });
  try {
    const deadline = Date.now() + 5_000;
    let ready = false;
    while (Date.now() < deadline) {
      const result = Bun.spawnSync(["lsof", "-nP", "-t", `-iTCP:${serverPort}`, "-sTCP:LISTEN"]);
      if (result.stdout.toString().trim()) {
        ready = true;
        break;
      }
      if (child.exitCode !== null) break;
      await Bun.sleep(50);
    }
    expect(ready).toBe(true);
    await expect(
      prepareDevSession({ force: false, mprocsBin, serverPort, servicePorts: [] }),
    ).rejects.toThrow("Re-run with --force");
    expect(child.exitCode).toBeNull();
    await prepareDevSession({ force: true, mprocsBin, serverPort, servicePorts: [] });
    expect(await child.exited).toBe(0);
  } finally {
    child.terminal?.close();
    if (child.exitCode === null) child.kill("SIGKILL");
    await child.exited;
    rmSync(directory, { recursive: true, force: true });
  }
}, 15_000);

test("force with help is accepted without stopping the running development stack", () => {
  const result = Bun.spawnSync(
    [process.execPath, `${import.meta.dir}/_dev.ts`, "--force", "--help"],
    {
      stdout: "pipe",
      stderr: "pipe",
      timeout: 5_000,
    },
  );
  expect(result.exitCode).toBe(0);
  expect(result.stdout.toString()).toContain("Usage: mprocs");
  expect(result.stderr.toString()).not.toContain("--force: quitting");
});
