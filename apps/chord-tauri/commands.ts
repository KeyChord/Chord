import { delimiter, join } from "node:path";

export const APP_DIR = import.meta.dir;
export const REPO_ROOT = Bun.fileURLToPath(new URL("../../", import.meta.url));

/** Run an installed tool, preserving its arguments, exit status and stop signals. */
export async function runCommand(argv: string[], { cwd = APP_DIR } = {}) {
  const child = Bun.spawn(argv, {
    cwd,
    env: {
      ...process.env,
      PATH: [
        join(APP_DIR, "node_modules/.bin"),
        join(REPO_ROOT, "node_modules/.bin"),
        process.env.PATH,
      ]
        .filter(Boolean)
        .join(delimiter),
    },
    stdio: ["inherit", "inherit", "inherit"],
  });
  const forward = (signal: NodeJS.Signals) => child.kill(signal);
  const handlers = (["SIGINT", "SIGTERM", "SIGHUP"] as const).map((signal) => {
    const handler = () => forward(signal);
    process.on(signal, handler);
    return { signal, handler };
  });
  const status = await child.exited;
  for (const { signal, handler } of handlers) process.off(signal, handler);
  if (status !== 0) process.exit(status);
}
