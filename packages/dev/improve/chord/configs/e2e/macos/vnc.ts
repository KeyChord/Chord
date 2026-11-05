/**
 * A minimal RFB 3.8 client for Virtualization.framework's VNC server
 * (`tart run --vnc-experimental`). It sends keyboard and pointer input to the VM's
 * virtual devices and reads the framebuffer for screenshots. Only the Raw encoding is
 * requested, which every server must support.
 */
// Bun has no DES; VNC authentication encrypts the challenge with it.
import { createCipheriv } from "node:crypto";
import type { Socket } from "bun";
import { Keysym, strokeFor } from "#keysyms";
import { encodePng } from "#png";
import { withTimeout } from "#wait";

const SECURITY_NONE = 1;
const SECURITY_VNC = 2;
const ENCODING_RAW = 0;
const ENCODING_DESKTOP_SIZE = -223;
const BUTTON_LEFT = 1;
const FRAME_RETRY_MS = 2_000;

/** Incoming bytes, read in exact sizes by a single reader. */
class ByteQueue {
  private chunks: Uint8Array[] = [];
  private length = 0;
  private error: Error | undefined;
  private waiter:
    | { size: number; resolve: (bytes: Uint8Array) => void; reject: (error: Error) => void }
    | undefined;

  push(data: Uint8Array) {
    this.chunks.push(data);
    this.length += data.length;
    this.wake();
  }

  fail(error: Error) {
    this.error ??= error;
    this.wake();
  }

  read(size: number): Promise<Uint8Array> {
    if (this.waiter) throw new Error("ByteQueue supports a single reader");
    return new Promise((resolve, reject) => {
      this.waiter = { size, resolve, reject };
      this.wake();
    });
  }

  private wake() {
    const waiter = this.waiter;
    if (!waiter) return;
    if (this.length >= waiter.size) {
      this.waiter = undefined;
      waiter.resolve(this.take(waiter.size));
    } else if (this.error) {
      this.waiter = undefined;
      waiter.reject(this.error);
    }
  }

  private take(size: number): Uint8Array {
    const out = new Uint8Array(size);
    let offset = 0;
    while (offset < size) {
      const head = this.chunks[0]!;
      const count = Math.min(head.length, size - offset);
      out.set(head.subarray(0, count), offset);
      offset += count;
      if (count === head.length) this.chunks.shift();
      else this.chunks[0] = head.subarray(count);
    }
    this.length -= size;
    return out;
  }
}

/** VNC authentication: DES-encrypt the challenge with the password's first 8 bytes, bit-reversed. */
export function vncAuthResponse(password: string, challenge: Uint8Array): Uint8Array {
  const key = new Uint8Array(8);
  const bytes = new TextEncoder().encode(password);
  for (let i = 0; i < Math.min(8, bytes.length); i++) {
    let byte = bytes[i]!;
    let reversed = 0;
    for (let bit = 0; bit < 8; bit++) {
      reversed = (reversed << 1) | (byte & 1);
      byte >>= 1;
    }
    key[i] = reversed;
  }
  const cipher = createCipheriv("des-ecb", key, null);
  cipher.setAutoPadding(false);
  return new Uint8Array(Buffer.concat([cipher.update(challenge), cipher.final()]));
}

const u16 = (bytes: Uint8Array, offset: number) =>
  new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength).getUint16(offset);
const u32 = (bytes: Uint8Array, offset: number) =>
  new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength).getUint32(offset);
const i32 = (bytes: Uint8Array, offset: number) =>
  new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength).getInt32(offset);

function message(size: number, write: (view: DataView) => void): Uint8Array {
  const bytes = new Uint8Array(size);
  write(new DataView(bytes.buffer));
  return bytes;
}

export interface TypeOptions {
  /** How long each key is held, and the pause after it. */
  delayMs?: number;
}

export class VncClient {
  width = 0;
  height = 0;
  /** The framebuffer as RGB, 3 bytes per pixel. */
  private rgb = new Uint8Array();
  private pending: Uint8Array[] = [];
  private frameWaiters: { resolve: () => void; reject: (error: Error) => void }[] = [];
  private failure: Error | undefined;
  private closed = false;
  /** Framebuffer updates requested and not yet read. */
  private framesRequested = 0;

  private constructor(
    private readonly socket: Socket<undefined>,
    private readonly input: ByteQueue,
    /** Resolves when the connection is gone, as it is when the VM stops. */
    readonly ended: Promise<void>,
  ) {}

  /** Connects to a `vnc://:password@host:port` URL, as printed by `tart run --vnc-experimental`. */
  static async connect(url: string | URL, { timeoutMs = 15_000 } = {}): Promise<VncClient> {
    const target = new URL(url);
    const input = new ByteQueue();
    // The socket exists before the client that writes to it.
    let drain = () => {};
    let resolveEnded = () => {};
    const ended = new Promise<void>((resolve) => {
      resolveEnded = resolve;
    });
    const socket = await Bun.connect({
      hostname: target.hostname,
      port: Number(target.port || 5900),
      socket: {
        data(_socket, data) {
          input.push(new Uint8Array(data));
        },
        drain() {
          drain();
        },
        close() {
          input.fail(new Error("VNC connection closed"));
          resolveEnded();
        },
        error(_socket, error) {
          input.fail(error);
        },
      },
    });
    const client = new VncClient(socket, input, ended);
    drain = () => client.flush();
    try {
      await withTimeout(
        client.handshake(decodeURIComponent(target.password)),
        timeoutMs,
        `VNC handshake with ${target.host}`,
      );
    } catch (error) {
      await client.close();
      throw error;
    }
    void client.readMessages();
    return client;
  }

  /** Presses (`down`) or releases a key. Events are sent in order. */
  key(keysym: number, down: boolean) {
    this.send(
      message(8, (view) => {
        view.setUint8(0, 4);
        view.setUint8(1, down ? 1 : 0);
        view.setUint32(4, keysym);
      }),
    );
  }

  async press(keysym: number, { delayMs = 40 }: TypeOptions = {}) {
    this.key(keysym, true);
    await Bun.sleep(delayMs);
    this.key(keysym, false);
    await Bun.sleep(delayMs);
  }

  /** Presses `keys` together: the leading keys are held, in order, while the last is pressed. */
  async combo(keys: number[], options: TypeOptions = {}) {
    const modifiers = keys.slice(0, -1);
    for (const modifier of modifiers) this.key(modifier, true);
    await this.press(keys.at(-1)!, options);
    for (const modifier of modifiers.toReversed()) this.key(modifier, false);
    await Bun.sleep(options.delayMs ?? 40);
  }

  /** Types `text` on a US layout, holding Shift for shifted characters. */
  async type(text: string, options: TypeOptions = {}) {
    for (const char of text) {
      const { keysym, shift } = strokeFor(char);
      if (shift) await this.combo([Keysym.Shift, keysym], options);
      else await this.press(keysym, options);
    }
  }

  /** Moves the pointer to framebuffer pixel coordinates, with `buttons` held. */
  move(x: number, y: number, buttons = 0) {
    this.send(
      message(6, (view) => {
        view.setUint8(0, 5);
        view.setUint8(1, buttons);
        view.setUint16(2, x);
        view.setUint16(4, y);
      }),
    );
  }

  /** Clicks at framebuffer pixel coordinates; `count: 2` is a double-click. */
  async click(x: number, y: number, { delayMs = 60, count = 1 } = {}) {
    this.move(x, y);
    await Bun.sleep(delayMs);
    for (let i = 0; i < count; i++) {
      this.move(x, y, BUTTON_LEFT);
      await Bun.sleep(delayMs);
      this.move(x, y);
      await Bun.sleep(delayMs);
    }
  }

  /** Presses at one point, moves in steps as a hand would, and releases at another. */
  async drag(
    from: { x: number; y: number },
    to: { x: number; y: number },
    { steps = 24, delayMs = 30 } = {},
  ) {
    this.move(from.x, from.y);
    await Bun.sleep(200);
    this.move(from.x, from.y, BUTTON_LEFT);
    await Bun.sleep(400);
    for (let step = 1; step <= steps; step++) {
      this.move(
        Math.round(from.x + ((to.x - from.x) * step) / steps),
        Math.round(from.y + ((to.y - from.y) * step) / steps),
        BUTTON_LEFT,
      );
      await Bun.sleep(delayMs);
    }
    await Bun.sleep(400);
    this.move(to.x, to.y);
    await Bun.sleep(200);
  }

  /** The current framebuffer. */
  async frame({ timeoutMs = 30_000 } = {}): Promise<{
    width: number;
    height: number;
    rgb: Uint8Array;
  }> {
    if (this.failure) throw this.failure;
    const updated = new Promise<void>((resolve, reject) =>
      this.frameWaiters.push({ resolve, reject }),
    );
    this.requestFrame();
    // The server occasionally drops a request while the screen is changing, and the next one
    // works, so asking once and waiting would just stall for the whole timeout.
    const retry = setInterval(() => this.requestFrame(), FRAME_RETRY_MS);
    try {
      await withTimeout(updated, timeoutMs, "VNC framebuffer update");
    } finally {
      clearInterval(retry);
    }
    return { width: this.width, height: this.height, rgb: this.rgb.slice() };
  }

  async screenshot(options: { timeoutMs?: number } = {}): Promise<Uint8Array> {
    const { width, height, rgb } = await this.frame(options);
    return encodePng(width, height, rgb);
  }

  /**
   * Reads every requested framebuffer update, then ends the connection and waits for the
   * server to do the same. Await this before the process exits.
   *
   * Prefer one connection for a VM's whole life. Virtualization.framework's VNC server crashes
   * the VM process when clients come and go on a busy screen: a hang-up with an update in
   * flight does it within two connections, and even clean hang-ups eventually leave the next
   * client without frames and the one after it crashing the VM (an assertion in
   * `-[_VZVirtualMachineAccessor addAccessorObserver:]`).
   */
  async close({ timeoutMs = FRAME_RETRY_MS * 2 } = {}) {
    if (this.closed) return;
    const settled = (promise: Promise<unknown>, what: string) =>
      withTimeout(promise, timeoutMs, what).catch(() => {});
    await settled(this.framesRead(), "outstanding VNC framebuffer updates");
    this.closed = true;
    this.socket.end();
    await settled(this.ended, "the VNC server to close the connection");
  }

  /** A request the server dropped never gets an answer, so the caller bounds this wait. */
  private async framesRead() {
    while (this.framesRequested > 0 && !this.failure) await Bun.sleep(10);
  }

  private async handshake(password: string) {
    const version = new TextDecoder().decode(await this.input.read(12));
    const match = /^RFB (\d{3})\.(\d{3})\n$/.exec(version);
    if (!match || Number(match[1]) < 3 || (Number(match[1]) === 3 && Number(match[2]) < 8)) {
      throw new Error(`unsupported VNC server version ${JSON.stringify(version)}`);
    }
    this.send(new TextEncoder().encode("RFB 003.008\n"));

    const [count] = await this.input.read(1);
    if (count === 0)
      throw new Error(`VNC server refused the connection: ${await this.readReason()}`);
    const types = await this.input.read(count!);
    if (password && types.includes(SECURITY_VNC)) {
      this.send(new Uint8Array([SECURITY_VNC]));
      this.send(vncAuthResponse(password, await this.input.read(16)));
    } else if (types.includes(SECURITY_NONE)) {
      this.send(new Uint8Array([SECURITY_NONE]));
    } else {
      throw new Error(`no supported VNC security type (server offers ${[...types].join(", ")})`);
    }
    if (u32(await this.input.read(4), 0) !== 0) {
      throw new Error(`VNC authentication failed: ${await this.readReason()}`);
    }

    // ClientInit: share the session rather than disconnecting other clients.
    this.send(new Uint8Array([1]));
    const init = await this.input.read(24);
    await this.input.read(u32(init, 20)); // desktop name
    this.resize(u16(init, 0), u16(init, 2));

    // SetPixelFormat: 32 bpp little-endian true colour, 8 bits per channel, so each pixel
    // arrives as B, G, R, padding.
    this.send(
      message(20, (view) => {
        view.setUint8(0, 0);
        const format = [32, 24, 0, 1, 0, 255, 0, 255, 0, 255, 16, 8, 0];
        format.forEach((value, index) => view.setUint8(4 + index, value));
      }),
    );
    // SetEncodings: Raw, plus DesktopSize so a resolution change keeps the stream in sync.
    const encodings = [ENCODING_RAW, ENCODING_DESKTOP_SIZE];
    this.send(
      message(4 + 4 * encodings.length, (view) => {
        view.setUint8(0, 2);
        view.setUint16(2, encodings.length);
        encodings.forEach((encoding, index) => view.setInt32(4 + 4 * index, encoding));
      }),
    );
  }

  private async readReason(): Promise<string> {
    const length = u32(await this.input.read(4), 0);
    return new TextDecoder().decode(await this.input.read(length));
  }

  private async readMessages() {
    try {
      for (;;) {
        const [type] = await this.input.read(1);
        switch (type) {
          case 0:
            await this.readFramebufferUpdate();
            break;
          case 1: {
            // SetColourMapEntries; unused with a true-colour format.
            const header = await this.input.read(5);
            await this.input.read(u16(header, 3) * 6);
            break;
          }
          case 2: // Bell
            break;
          case 3: {
            // ServerCutText
            const header = await this.input.read(7);
            await this.input.read(u32(header, 3));
            break;
          }
          default:
            throw new Error(`unexpected VNC server message type ${type}`);
        }
      }
    } catch (error) {
      this.failure = error instanceof Error ? error : new Error(String(error));
      for (const waiter of this.frameWaiters.splice(0)) waiter.reject(this.failure);
    }
  }

  private async readFramebufferUpdate() {
    const header = await this.input.read(3);
    let resized = false;
    for (let index = u16(header, 1); index > 0; index--) {
      const rect = await this.input.read(12);
      const [x, y, width, height] = [u16(rect, 0), u16(rect, 2), u16(rect, 4), u16(rect, 6)];
      const encoding = i32(rect, 8);
      if (encoding === ENCODING_DESKTOP_SIZE) {
        this.resize(width, height);
        resized = true;
        continue;
      }
      if (encoding !== ENCODING_RAW) throw new Error(`unexpected VNC encoding ${encoding}`);
      const pixels = await this.input.read(width * height * 4);
      for (let row = 0; row < height; row++) {
        let source = row * width * 4;
        let target = ((y + row) * this.width + x) * 3;
        for (let column = 0; column < width; column++) {
          this.rgb[target++] = pixels[source + 2]!;
          this.rgb[target++] = pixels[source + 1]!;
          this.rgb[target++] = pixels[source]!;
          source += 4;
        }
      }
    }
    this.framesRequested = Math.max(0, this.framesRequested - 1);
    // A resize invalidates the frame; ask again at the new size.
    if (resized && this.frameWaiters.length > 0) {
      this.requestFrame();
      return;
    }
    for (const waiter of this.frameWaiters.splice(0)) waiter.resolve();
  }

  private resize(width: number, height: number) {
    this.width = width;
    this.height = height;
    this.rgb = new Uint8Array(width * height * 3);
  }

  private requestFrame() {
    this.framesRequested++;
    this.send(
      message(10, (view) => {
        view.setUint8(0, 3);
        view.setUint8(1, 0); // full update, not incremental
        view.setUint16(6, this.width);
        view.setUint16(8, this.height);
      }),
    );
  }

  private send(bytes: Uint8Array) {
    if (this.closed) throw new Error("VNC connection is closed");
    this.pending.push(bytes);
    this.flush();
  }

  private flush() {
    while (this.pending.length > 0) {
      const head = this.pending[0]!;
      const written = this.socket.write(head);
      if (written < 0) throw new Error("VNC connection is closed");
      if (written < head.length) {
        this.pending[0] = head.subarray(written);
        return;
      }
      this.pending.shift();
    }
  }
}
