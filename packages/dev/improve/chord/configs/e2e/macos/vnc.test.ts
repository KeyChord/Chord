import { afterEach, describe, expect, test } from "bun:test";
import { inflateSync } from "node:zlib";
import { Keysym, strokeFor } from "#keysyms";
import { VncClient, vncAuthResponse } from "#vnc";

interface FakeServer {
  url: string;
  /** Key events received, as [keysym, down]. */
  keys: [number, boolean][];
  /** Client messages received, by type. */
  messages: number[];
  stop(): void;
}

/**
 * Speaks the server side of RFB 3.8 with VNC authentication and answers framebuffer
 * requests with a fixed 3x2 image, announcing a resize first to exercise DesktopSize.
 */
function fakeServer(password: string): FakeServer {
  const challenge = crypto.getRandomValues(new Uint8Array(16));
  const keys: [number, boolean][] = [];
  const messages: number[] = [];
  let buffer = new Uint8Array();
  let stage: "version" | "security" | "response" | "init" | "messages" = "version";
  let resized = false;

  const take = (size: number) => {
    if (buffer.length < size) return undefined;
    const bytes = buffer.slice(0, size);
    buffer = buffer.slice(size);
    return bytes;
  };

  const frame = (width: number, height: number) => {
    const out = new Uint8Array(4 + 12 + width * height * 4);
    const view = new DataView(out.buffer);
    view.setUint16(2, 1);
    view.setUint16(8, width);
    view.setUint16(10, height);
    for (let pixel = 0; pixel < width * height; pixel++) {
      // B, G, R, padding: red grows with the pixel index, blue is constant.
      out.set([200, 100, pixel * 10, 0], 16 + pixel * 4);
    }
    return out;
  };

  const server = Bun.listen({
    hostname: "127.0.0.1",
    port: 0,
    socket: {
      open(socket) {
        socket.write("RFB 003.008\n");
      },
      data(socket, data) {
        buffer = new Uint8Array([...buffer, ...data]);
        for (;;) {
          if (stage === "version") {
            if (!take(12)) return;
            socket.write(new Uint8Array([1, 2]));
            stage = "security";
          } else if (stage === "security") {
            const [type] = take(1) ?? [];
            if (type === undefined) return;
            expect(type).toBe(2);
            socket.write(challenge);
            stage = "response";
          } else if (stage === "response") {
            const response = take(16);
            if (!response) return;
            const ok = Buffer.from(response).equals(vncAuthResponse(password, challenge));
            socket.write(new Uint8Array([0, 0, 0, ok ? 0 : 1]));
            if (!ok) {
              const reason = new TextEncoder().encode("bad password");
              socket.write(new Uint8Array([0, 0, 0, reason.length, ...reason]));
              socket.end();
              return;
            }
            stage = "init";
          } else if (stage === "init") {
            if (!take(1)) return;
            const name = new TextEncoder().encode("fake");
            const init = new Uint8Array(24 + name.length);
            const view = new DataView(init.buffer);
            view.setUint16(0, 2);
            view.setUint16(2, 2);
            view.setUint32(20, name.length);
            init.set(name, 24);
            socket.write(init);
            stage = "messages";
          } else {
            const type = buffer[0];
            if (type === undefined) return;
            const view = new DataView(buffer.buffer, buffer.byteOffset, buffer.byteLength);
            const size =
              type === 0
                ? 20
                : type === 2
                  ? buffer.length >= 4
                    ? 4 + 4 * view.getUint16(2)
                    : Infinity
                  : type === 3
                    ? 10
                    : type === 4
                      ? 8
                      : type === 5
                        ? 6
                        : Infinity;
            const bytes = take(size);
            if (!bytes) return;
            messages.push(type);
            const message = new DataView(bytes.buffer);
            if (type === 4) keys.push([message.getUint32(4), bytes[1] === 1]);
            if (type === 3) {
              if (!resized) {
                // Resize to 3x2 without pixels; the client must ask again.
                resized = true;
                const resize = new Uint8Array(16);
                const resizeView = new DataView(resize.buffer);
                resizeView.setUint16(2, 1);
                resizeView.setUint16(8, 3);
                resizeView.setUint16(10, 2);
                resizeView.setInt32(12, -223);
                socket.write(resize);
              } else {
                expect([message.getUint16(6), message.getUint16(8)]).toEqual([3, 2]);
                socket.write(frame(3, 2));
              }
            }
          }
        }
      },
    },
  });
  return {
    url: `vnc://:${encodeURIComponent(password)}@127.0.0.1:${server.port}`,
    keys,
    messages,
    stop: () => server.stop(true),
  };
}

let server: FakeServer | undefined;
afterEach(() => server?.stop());

describe("VncClient", () => {
  test("authenticates, types with Shift, and captures the framebuffer", async () => {
    server = fakeServer("lazy-cat-tree-moon");
    const client = await VncClient.connect(server.url);
    expect([client.width, client.height]).toEqual([2, 2]);

    await client.type("a!", { delayMs: 1 });
    await client.combo([Keysym.Command, "q".charCodeAt(0)], { delayMs: 1 });
    const frame = await client.frame();
    client.close();

    expect(server.keys).toEqual([
      [0x61, true],
      [0x61, false],
      [Keysym.Shift, true],
      [0x31, true],
      [0x31, false],
      [Keysym.Shift, false],
      [Keysym.Command, true],
      [0x71, true],
      [0x71, false],
      [Keysym.Command, false],
    ]);
    // SetPixelFormat and SetEncodings precede every other message.
    expect(server.messages.slice(0, 2)).toEqual([0, 2]);
    expect([frame.width, frame.height]).toEqual([3, 2]);
    expect([...frame.rgb.subarray(0, 6)]).toEqual([0, 100, 200, 10, 100, 200]);
  });

  test("reports a rejected password", async () => {
    server = fakeServer("right");
    expect(VncClient.connect(server.url.replace(":right@", ":wrong@"))).rejects.toThrow(
      "VNC authentication failed: bad password",
    );
  });

  test("encodes screenshots as PNG", async () => {
    server = fakeServer("pw");
    const client = await VncClient.connect(server.url);
    const png = await client.screenshot();
    client.close();

    expect([...png.subarray(1, 4)]).toEqual([0x50, 0x4e, 0x47]);
    const view = new DataView(png.buffer, png.byteOffset);
    expect([view.getUint32(16), view.getUint32(20)]).toEqual([3, 2]);
    const idatLength = view.getUint32(33);
    const scanlines = inflateSync(png.subarray(41, 41 + idatLength));
    expect([...scanlines.subarray(0, 7)]).toEqual([0, 0, 100, 200, 10, 100, 200]);
  });
});

describe("strokeFor", () => {
  test("sends shifted characters as their base key with Shift", () => {
    expect(strokeFor("G")).toEqual({ keysym: 0x67, shift: true });
    expect(strokeFor("?")).toEqual({ keysym: 0x2f, shift: true });
    expect(strokeFor("/")).toEqual({ keysym: 0x2f, shift: false });
    expect(strokeFor("\n")).toEqual({ keysym: Keysym.Return, shift: false });
    expect(() => strokeFor("é")).toThrow();
  });
});

test("VNC authentication uses bit-reversed DES keys", () => {
  // Each password byte reverses to a byte of the classic DES test key 0123456789ABCDEF,
  // up to the parity bit DES ignores: 0x44 ("D") reverses to 0x22, 0x11 to 0x88, and so on.
  const password = '\x00D"f\x11U3w';
  const challenge = new TextEncoder().encode("Now is tNow is t");
  const response = vncAuthResponse(password, challenge);
  expect(Buffer.from(response).toString("hex")).toBe("3fa40e8a984d4815".repeat(2));
});
