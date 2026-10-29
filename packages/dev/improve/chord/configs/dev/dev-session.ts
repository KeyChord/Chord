type DevSessionOptions = {
  force: boolean;
  mprocsBin: string;
  serverPort: number;
  servicePorts: number[];
};

// Do not probe the control server by connecting: mprocs 0.9.6 panics on an empty request.
async function listeners(port: number): Promise<number[]> {
  const result = Bun.spawnSync(["lsof", "-nP", "-t", `-iTCP:${port}`, "-sTCP:LISTEN"], {
    stdout: "pipe",
    stderr: "pipe",
    timeout: 15_000,
  });
  const stdout = result.stdout.toString().trim();
  const stderr = result.stderr.toString().trim();
  if (result.exitCode === 1 && !stdout && !stderr) return [];
  if (result.exitCode !== 0) {
    throw new Error(
      `Could not inspect port ${port}: ${stderr || result.signalCode || result.exitCode}`,
    );
  }
  const pids = stdout ? [...new Set(stdout.split(/\s+/).map(Number))] : [];
  if (pids.some((pid) => !Number.isSafeInteger(pid) || pid <= 0)) {
    throw new Error(`Could not inspect port ${port}: lsof returned an invalid PID`);
  }
  return pids;
}

async function waitForRelease(port: number, timeout: number): Promise<boolean> {
  const deadline = Date.now() + timeout;
  do {
    if ((await listeners(port)).length === 0) return true;
    await Bun.sleep(100);
  } while (Date.now() < deadline);
  return false;
}

async function freePort(port: number): Promise<void> {
  for (const signal of ["SIGTERM", "SIGKILL"] as const) {
    for (const pid of await listeners(port)) {
      try {
        process.kill(pid, signal);
      } catch (error) {
        if ((error as NodeJS.ErrnoException).code !== "ESRCH") throw error;
      }
    }
    if (await waitForRelease(port, 2_000)) return;
  }
  throw new Error(
    `Port ${port} is still in use after --force. Inspect it with lsof -nP -iTCP:${port} -sTCP:LISTEN.`,
  );
}

export async function prepareDevSession({
  force,
  mprocsBin,
  serverPort,
  servicePorts,
}: DevSessionOptions): Promise<void> {
  const server = `127.0.0.1:${serverPort}`;
  const ports = [serverPort, ...servicePorts];
  if (!force) {
    for (const port of ports) {
      if ((await listeners(port)).length > 0) {
        throw new Error(
          `Port ${port} is already in use; another dev session may be running. Re-run with --force to quit it first.`,
        );
      }
    }
    return;
  }

  if ((await listeners(serverPort)).length > 0) {
    console.warn(`[dev] --force: quitting existing mprocs session on ${server}`);
    const quit = Bun.spawnSync([mprocsBin, "--ctl", "{c: force-quit}", "--server", server], {
      stdout: "ignore",
      stderr: "inherit",
      timeout: 5_000,
    });
    if (quit.exitCode !== 0)
      console.warn("[dev] mprocs force-quit failed; stopping the port holder directly.");
    await waitForRelease(serverPort, 2_000);
  }
  // Also reclaim orphaned Vite/HMR listeners from sessions started before the control server existed.
  for (const port of ports) await freePort(port);
}
