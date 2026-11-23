// This source is embedded in the launcher and evaluated by the bundled Bun binary.
import { createConnection } from "node:net";
import { dirname, isAbsolute, join, relative, resolve, sep } from "node:path";

const MAX_FRAME = 1024 * 1024;
const HOST_TIMEOUT = 15_000;

async function framedConnection(address) {
  const [host, port] = address.split(":");
  const socket = createConnection({ host, port: Number(port), noDelay: true });
  const messages = (async function* () {
    let pending = Buffer.alloc(0);
    for await (const chunk of socket) {
      pending = pending.length ? Buffer.concat([pending, chunk]) : chunk;
      while (pending.length >= 4) {
        const size = pending.readUInt32BE(0);
        if (size > MAX_FRAME) throw new Error("Package IPC message exceeds 1 MiB");
        if (pending.length < size + 4) break;
        yield JSON.parse(pending.toString("utf8", 4, size + 4));
        pending = pending.subarray(size + 4);
      }
    }
    if (pending.length) throw new Error("Truncated package IPC message");
  })();
  return {
    async read() {
      const next = await messages.next();
      if (next.done) throw new Error("Chord closed the package connection");
      return next.value;
    },
    write(value) {
      const body = Buffer.from(JSON.stringify(value));
      if (body.length > MAX_FRAME) throw new Error("Package IPC message exceeds 1 MiB");
      const frame = Buffer.allocUnsafe(4 + body.length);
      frame.writeUInt32BE(body.length);
      body.copy(frame, 4);
      return new Promise((resolve, reject) => socket.write(frame, (error) => {
        if (error) reject(error);
        else resolve();
      }));
    },
    onClose(callback) {
      socket.once("close", callback);
    },
  };
}

// Package handlers expect press/release/tap to return synchronously. A separate
// thread services their host connection while the script thread waits.
function hostWorkerSource() {
  return `
    import { createConnection } from "node:net";
    const MAX_FRAME = ${MAX_FRAME};
    const framedConnection = ${framedConnection.toString()};
    self.onmessage = async ({ data }) => {
      const { address, token, shared } = data;
      const state = new Int32Array(shared, 0, 2);
      const bytes = new Uint8Array(shared, 8);
      const host = await framedConnection(address);
      await host.write({ token, channel: "host" });
      host.onClose(() => process.exit(0));
      self.onmessage = async ({ data: request }) => {
        let result;
        try {
          await host.write(request);
          result = (await host.read()).result;
        } catch (error) {
          result = { Err: String(error) };
        }
        const json = JSON.stringify(result);
        const encoded = new TextEncoder().encode(json);
        if (encoded.length <= bytes.length) {
          bytes.set(encoded);
          Atomics.store(state, 1, encoded.length);
        } else {
          Atomics.store(state, 1, -1);
        }
        Atomics.store(state, 0, 1);
        Atomics.notify(state, 0);
      };
    };
  `;
}

function connectHost(address, token) {
  const shared = new SharedArrayBuffer(MAX_FRAME + 8);
  const state = new Int32Array(shared, 0, 2);
  const bytes = new Uint8Array(shared, 8);
  const url = URL.createObjectURL(new Blob([hostWorkerSource()], { type: "text/javascript" }));
  const worker = new Worker(url);
  worker.addEventListener("error", (error) => {
    console.error(error.message);
    process.exit(1);
  });
  worker.postMessage({ address, token, shared });
  return (request) => {
    Atomics.store(state, 0, 0);
    worker.postMessage(request);
    if (Atomics.wait(state, 0, 0, HOST_TIMEOUT) === "timed-out") {
      console.error("Chord host request timed out");
      process.exit(1);
    }
    const length = Atomics.load(state, 1);
    if (length < 0) throw new Error("Package host response exceeds 1 MiB");
    const result = JSON.parse(new TextDecoder().decode(bytes.slice(0, length)));
    if ("Err" in result) throw new Error(result.Err);
    return result.Ok;
  };
}

function unsupported(feature) {
  throw new Error(`${feature} is unavailable on ${process.platform}`);
}

function localPath(path) {
  if (process.platform !== "win32") return path;
  const extended = "\\\\?\\";
  const unc = extended + "UNC\\";
  if (path.startsWith(unc)) return "\\\\" + path.slice(unc.length);
  return path.startsWith(extended) ? path.slice(extended.length) : path;
}

function chordModule(root, host, launches, terminations) {
  function hostCall(request) {
    if (!host) throw new Error("Chord host APIs are unavailable in CLI mode");
    return host(request);
  }

  function subscribe(callbacks, bundleId, callback) {
    if (!host) throw new Error("Chord host APIs are unavailable in CLI mode");
    const entry = { bundleId, callback };
    callbacks.add(entry);
    return () => callbacks.delete(entry);
  }

  function resolvePackageFile(meta, value) {
    const modulePath = localPath(meta.path ?? Bun.fileURLToPath(meta.url));
    let packageRoot = root;
    if (!packageRoot || relative(packageRoot, modulePath).startsWith("..")) {
      packageRoot = dirname(modulePath);
      while (!Bun.file(join(packageRoot, "package.json")).size) {
        const parent = dirname(packageRoot);
        if (parent === packageRoot) throw new Error(`${modulePath} is outside a chord package`);
        packageRoot = parent;
      }
    }
    if (isAbsolute(value) || value.split(/[\\/]+/).includes("..")) {
      throw new Error("Invalid package-relative path");
    }
    const parts = relative(packageRoot, modulePath).split(sep);
    const prefix = [];
    for (let i = 0; parts[i] === "js" && parts[i + 1]?.startsWith("@") && parts[i + 3] === "js"; i += 3) {
      prefix.push(parts[i + 1], parts[i + 2]);
    }
    if (prefix.length && value.startsWith("target/")) {
      return join(packageRoot, "target", ...prefix, value);
    }
    return join(packageRoot, ...prefix, value);
  }

  Bun.plugin({
    name: "chord",
    setup(builder) {
      builder.module("chord", () => ({
        loader: "object",
        exports: {
          press: (key) => hostCall({ Press: key }),
          release: (key) => hostCall({ Release: key }),
          tap: (key) => hostCall({ Tap: key }),
          getGlobalHotkey: (bundle_id, hotkey_id) => hostCall({ GetGlobalHotkey: { bundle_id, hotkey_id } }),
          registerGlobalHotkey: (bundle_id, hotkey_id) => hostCall({ RegisterGlobalHotkey: { bundle_id, hotkey_id } }),
          setAppNeedsRelaunch: (bundle_id, needs_relaunch) => hostCall({ SetAppNeedsRelaunch: { bundle_id, needs_relaunch } }),
          getAppWindows: (app_id) => hostCall({ AppWindows: { app_id } }),
          getScreenBounds: () => hostCall("ScreenBounds"),
          mouse: async (options) => hostCall({ Mouse: options }),
          click: async ({ moveCursor: _unused, ...options }) => hostCall({ Mouse: { ...options, action: "click" } }),
          onAppLaunch: (id, callback) => { subscribe(launches, id, callback); },
          onAppTerminate: (id, callback) => subscribe(terminations, id, callback),
          Applescript: class { constructor() { unsupported("AppleScript"); } },
          runSudoCommand: async () => unsupported("runSudoCommand"),
          runCommand: (command, ...args) => Bun.spawn([command, ...args]),
          resolvePackageFile,
          resolveNativeModulePath: (meta, path) => resolvePackageFile(meta, join(
            "target", process.env.CHORD_TARGET_TRIPLE, path, `${path.split(/[\\/]+/).at(-1)}.node`,
          )),
        },
      }));
    },
  });
}

function decode(value) {
  return "Json" in value ? value.Json : Bun.TOML.parse(value.Toml).value;
}

function resolveArgument(value, scope) {
  if (typeof value === "string" && /^\$[a-z_][\w]*$/i.test(value)) {
    const identifier = value.slice(1);
    if (!scope?.has(identifier)) {
      throw new ReferenceError(`${value} has not been assigned for this chord's matching interval`);
    }
    return scope.get(identifier);
  }
  if (Array.isArray(value)) return value.map((item) => resolveArgument(item, scope));
  if (value && Object.getPrototypeOf(value) === Object.prototype) {
    return Object.fromEntries(Object.entries(value).map(([key, item]) => [key, resolveArgument(item, scope)]));
  }
  return value;
}

function keyboardEvent(data) {
  if (!data) return null;
  const event = new Event(data.type);
  for (const [key, value] of Object.entries(data)) {
    if (key !== "type") Object.defineProperty(event, key, { value, enumerable: true });
  }
  Object.defineProperties(event, {
    view: { value: null },
    detail: { value: 0 },
    getModifierState: { value(key) {
      return ({ Alt: this.altKey, Control: this.ctrlKey, Meta: this.metaKey, Shift: this.shiftKey })[key] === true;
    } },
  });
  return event;
}

async function serve(address, token) {
  const control = await framedConnection(address);
  await control.write({ token, channel: "control" });
  control.onClose(() => process.exit(0));
  const host = connectHost(address, token);
  const config = await control.read();
  process.chdir(localPath(config.cwd));
  const handlers = new Map();
  const sequences = new Map();
  const launches = new Set();
  const terminations = new Set();
  chordModule(localPath(config.root), host, launches, terminations);

  async function handle(request) {
    if (request === "Ping") return null;
    if ("Register" in request) {
      const { module, raw, pathslug, app_id, args } = request.Register;
      const build = await (await import(Bun.pathToFileURL(localPath(module)).href)).default;
      if (typeof build !== "function") throw new TypeError("JS default export did not resolve to a function");
      const handler = await build.apply({ chordsFile: raw, chordsFilePath: pathslug, chordsFileAppId: app_id }, decode(args));
      if (typeof handler !== "function") throw new TypeError("The default export function must return a function");
      const id = crypto.randomUUID();
      handlers.set(id, handler);
      return id;
    }
    if ("Invoke" in request) {
      const { action: payload, app_id, count } = request.Invoke;
      const action = decode(payload);
      const handler = handlers.get(action.handlerId);
      if (!handler) throw new Error(`Handler ID '${action.handlerId}' not found in registry`);
      let scope;
      if (action.sequenceId !== null) {
        scope = sequences.get(action.sequenceId);
        if (!scope) {
          scope = new Map();
          sequences.set(action.sequenceId, scope);
        }
      }
      for (let i = 0; i < count; i++) {
        const result = await handler.apply({
          focusedAppId: app_id,
          focusedWindowId: action.focusedWindowId,
          input: action.input,
          event: keyboardEvent(action.keyboardEvent),
        }, action.eventArgs.map((arg) => resolveArgument(arg, scope)));
        if (action.assignTo !== null) {
          if (!scope) throw new Error("Return-value assignment requires a chord matching interval");
          scope.set(action.assignTo, result);
        }
      }
    } else if ("ReleaseSequences" in request) {
      for (const id of request.ReleaseSequences) sequences.delete(id);
    } else if ("RetainHandlers" in request) {
      const keep = new Set(request.RetainHandlers);
      for (const id of handlers.keys()) if (!keep.has(id)) handlers.delete(id);
    } else if ("Lifecycle" in request) {
      const { app, launch } = request.Lifecycle;
      for (const { bundleId, callback } of launch ? launches : terminations) {
        if (bundleId === app.bundleId) await callback(app);
      }
    } else {
      throw new Error("Unknown package request");
    }
    return null;
  }

  while (true) {
    let request;
    try {
      request = await control.read();
    } catch (error) {
      if (error.message === "Chord closed the package connection") process.exit(0);
      throw error;
    }
    let result;
    try {
      result = { Ok: await handle(request) };
    } catch (error) {
      result = { Err: String(error.stack ?? error).slice(0, 16_384) };
    }
    await control.write({ result });
  }
}

const [bun, mode, path, ...args] = process.argv;
if (mode === "worker") {
  await serve(path, args[0]);
} else {
  const absolutePath = resolve(localPath(path));
  process.argv = [bun, absolutePath, ...(mode === "export" ? args.slice(1) : args)];
  chordModule(null, null, new Set(), new Set());
  const module = await import(Bun.pathToFileURL(absolutePath).href);
  if (mode === "export") {
    const [name, ...values] = args;
    if (typeof module[name] !== "function") throw new TypeError(`No exported function named '${name}'`);
    await module[name](...values.map((value) => {
      try { return JSON.parse(value); } catch { return value; }
    }));
  }
}
