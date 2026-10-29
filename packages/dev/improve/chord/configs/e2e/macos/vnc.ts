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

  private constructor(
    private readonly socket: Socket<undefined>,
    private readonly input: ByteQueue,
  ) {}

  /** Connects to a `vnc://:password@host:port` URL, as printed by `tart run --vnc-experimental`. */
  static async connect(url: string | URL, { timeoutMs = 15_000 } = {}): Promise<VncClient> {
    const target = new URL(url);
    const input = new ByteQueue();
    // The socket exists before the client that writes to it.
    let drain = () => {};
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
        },
        error(_socket, error) {
          input.fail(error);
        },
      },
    });
    const client = new VncClient(socket, input);
    drain = () => client.flush();
    try {
      await withTimeout(
        client.handshake(decodeURIComponent(target.password)),
        timeoutMs,
        `VNC handshake with ${target.host}`,
      );
    } catch (error) {
      client.close();
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

  /** Clicks at framebuffer pixel coordinates. */
  async click(x: number, y: number, { delayMs = 60 } = {}) {
    const pointer = (buttons: number) =>
      message(6, (view) => {
        view.setUint8(0, 5);
        view.setUint8(1, buttons);
        view.setUint16(2, x);
        view.setUint16(4, y);
      });
    this.send(pointer(0));
    await Bun.sleep(delayMs);
    this.send(pointer(1));
    await Bun.sleep(delayMs);
    this.send(pointer(0));
    await Bun.sleep(delayMs);
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
    await withTimeout(updated, timeoutMs, "VNC framebuffer update");
    return { width: this.width, height: this.height, rgb: this.rgb.slice() };
  }

  async screenshot(options: { timeoutMs?: number } = {}): Promise<Uint8Array> {
    const { width, height, rgb } = await this.frame(options);
    return encodePng(width, height, rgb);
  }

  close() {
    if (this.closed) return;
    this.closed = true;
    this.socket.end();
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
