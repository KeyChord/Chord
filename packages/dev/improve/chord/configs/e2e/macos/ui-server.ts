/**
 * Keeps the one VNC connection of a VM that is being driven step by step, by a person or an
 * agent, with `_e2e-macos ui`. Each step is a short-lived process, and connecting from each of
 * them would crash the VM; see `VncClient.close`.
 */
import { rmSync } from "node:fs";
import { join } from "node:path";
import { allowKnownDevelopers } from "#fresh-mac";
import { Guest } from "#guest";
import { keysymNamed, Keysym } from "#keysyms";
import { center, recognizeText, Screen } from "#screen";
import type { SessionState } from "#vm";
import { VncClient } from "#vnc";
import { waitFor } from "#wait";

export interface UiRequest {
  action: string;
  args: string[];
  timeoutSeconds?: number;
  caps?: boolean;
}

function socketPath(vm: string): string {
  // A socket address has to fit in 104 bytes, which a checkout's path might not.
  return `/tmp/chord-e2e-ui-${vm}.sock`;
}

async function perform(
  screen: Screen,
  state: SessionState,
  { action, args, timeoutSeconds = 60, caps }: UiRequest,
): Promise<string> {
  const { vnc } = screen;
  const timeoutMs = timeoutSeconds * 1000;
  const numbers = () => args.map(Number);
  /** Clicking takes a point or some visible text. */
  const target = async () => {
    const [x, y] = numbers();
    if (args.length === 2 && Number.isFinite(x) && Number.isFinite(y)) return { x: x!, y: y! };
    return center(await screen.waitForText(args.join(" "), { timeoutMs }));
  };
  switch (action) {
    case "shot":
      break;
    case "click": {
      const { x, y } = await target();
      await vnc.click(x, y);
      break;
    }
    case "dclick": {
      const { x, y } = await target();
      await vnc.click(x, y, { count: 2 });
      break;
    }
    case "drag": {
      const [x1, y1, x2, y2] = numbers();
      await vnc.drag({ x: x1!, y: y1! }, { x: x2!, y: y2! });
      break;
    }
    case "type":
      await vnc.type(args.join(" "));
      break;
    case "key":
      await vnc.combo(args.join("+").split("+").map(keysymNamed));
      break;
    case "chord":
      await screen.chord(args.join(" "), caps ? { complete: "caps" } : {});
      break;
    case "wait":
      await screen.waitForText(args.join(" "), { timeoutMs });
      break;
    case "allow-known-developers": {
      const guest = await Guest.connect(state.host);
      try {
        await allowKnownDevelopers(guest, screen);
      } finally {
        await guest.close();
      }
      return "Gatekeeper allows the App Store and known developers";
    }
    default:
      throw new Error(`unknown ui action ${action}`);
  }
  for (const key of [Keysym.Space, Keysym.CapsLock, Keysym.Shift, Keysym.Command]) {
    vnc.key(key, false);
  }
  // Let the screen react before looking at it.
  if (action !== "shot") await Bun.sleep(800);
  const name = action === "shot" ? (args[0] ?? "shot") : `${action} ${args.join(" ").slice(0, 32)}`;
  const path = await screen.capture(name);
  const lines = (await recognizeText(path)).map(
    (box) => `${String(center(box).x).padStart(4)},${String(center(box).y).padEnd(4)} ${box.text}`,
  );
  return [path, `screen ${vnc.width}x${vnc.height}`, ...lines].join("\n");
}

/** Runs until the VM goes away or `stopUiServer` is called. */
export async function serveUi(state: SessionState) {
  const path = socketPath(state.vm);
  rmSync(path, { force: true });
  const vnc = await VncClient.connect(state.vncUrl);
  const screen = new Screen(vnc, join(state.artifacts, "screen"));
  await screen.wake();
  let last: Promise<unknown> = Promise.resolve();
  Bun.serve({
    unix: path,
    async fetch(request, server) {
      // A step can wait a long time for something to appear on screen.
      server.timeout(request, 0);
      const { pathname } = new URL(request.url);
      if (pathname === "/ping") return new Response("ok");
      if (pathname === "/stop") {
        setTimeout(() => void vnc.close().finally(() => process.exit(0)), 50);
        return new Response("stopping");
      }
      const body = (await request.json()) as UiRequest;
      // Steps run one at a time, in the order they arrive.
      const step = last.then(() => perform(screen, state, body));
      last = step.catch(() => {});
      try {
        return new Response(await step);
      } catch (error) {
        const shot = await screen.capture("failure").catch(() => "");
        return new Response(`${error instanceof Error ? error.message : error}\n${shot}`, {
          status: 500,
        });
      }
    },
  });
  await vnc.ended;
  rmSync(path, { force: true });
  process.exit(0);
}

async function reachable(path: string): Promise<boolean> {
  try {
    await fetch("http://ui/ping", { unix: path, method: "HEAD" });
    return true;
  } catch {
    return false;
  }
}

/** Sends a step to the VM's UI server, starting the server if it is not running. */
export async function uiRequest(state: SessionState, request: UiRequest): Promise<string> {
  const path = socketPath(state.vm);
  if (!(await reachable(path))) {
    const log = Bun.file(join(state.artifacts, "ui-server.log"));
    Bun.spawn(
      [process.execPath, join(import.meta.dir, "_e2e-macos.ts"), "__ui-server", "--vm", state.vm],
      { stdin: "ignore", stdout: log, stderr: log, detached: true },
    ).unref();
    await waitFor(`the UI server of ${state.vm}`, async () =>
      (await reachable(path)) ? true : undefined,
    );
  }
  const response = await fetch("http://ui/", {
    unix: path,
    method: "POST",
    body: JSON.stringify(request),
    timeout: false,
  });
  const reply = await response.text();
  if (!response.ok) throw new Error(`ui ${request.action} failed: ${reply}`);
  return reply;
}

export async function stopUiServer(vm: string) {
  const path = socketPath(vm);
  if (await reachable(path)) await fetch("http://ui/stop", { unix: path, method: "POST" });
}
