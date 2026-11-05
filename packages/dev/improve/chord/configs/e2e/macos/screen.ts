/**
 * What a person sees and does: reads the VM's screen with Vision's OCR on the host, and acts
 * on visible text through the VM's virtual mouse and keyboard. No accessibility grants or
 * guest tooling are involved, so it also works on a pristine VM with SIP enabled, and a test
 * fails when the text a user is told to look for is not actually on screen.
 */
import { mkdirSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";
import { Keysym } from "#keysyms";
import { E2E_DIR } from "#vm";
import type { VncClient } from "#vnc";
import { waitFor, type WaitOptions } from "#wait";

const OCR_SOURCE = join(import.meta.dir, "ocr.swift");
const OCR_BINARY = join(E2E_DIR, "bin/ocr");

export interface TextBox {
  text: string;
  x: number;
  y: number;
  width: number;
  height: number;
}

async function ocrBinary(): Promise<string> {
  const built = statSync(OCR_BINARY, { throwIfNoEntry: false });
  if (built && built.mtimeMs >= statSync(OCR_SOURCE).mtimeMs) return OCR_BINARY;
  mkdirSync(join(E2E_DIR, "bin"), { recursive: true });
  const swiftc = Bun.spawn(["xcrun", "swiftc", "-O", OCR_SOURCE, "-o", OCR_BINARY], {
    stdout: "inherit",
    stderr: "inherit",
  });
  if ((await swiftc.exited) !== 0) throw new Error("cannot compile ocr.swift");
  return OCR_BINARY;
}

/** The text in a PNG file, top to bottom and then left to right. */
export async function recognizeText(png: string): Promise<TextBox[]> {
  const child = Bun.spawn([await ocrBinary(), png], { stdout: "pipe", stderr: "pipe" });
  const [stdout, stderr, status] = await Promise.all([
    new Response(child.stdout).text(),
    new Response(child.stderr).text(),
    child.exited,
  ]);
  if (status !== 0) throw new Error(`ocr failed (${status}): ${stderr.trim()}`);
  const boxes = stdout
    .split("\n")
    .filter(Boolean)
    .map((line) => JSON.parse(line) as TextBox);
  return boxes.toSorted((a, b) =>
    Math.abs(a.y - b.y) < Math.max(a.height, b.height) / 2 ? a.x - b.x : a.y - b.y,
  );
}

/** Normalizes the characters OCR confuses, so a test can match the text it expects. */
function normalize(text: string): string {
  return text.replace(/[’‘]/g, "'").replace(/[“”]/g, '"').replace(/…/g, "...").toLowerCase();
}

export type TextQuery = string | RegExp;

function matches(box: TextBox, query: TextQuery): boolean {
  return typeof query === "string"
    ? normalize(box.text).includes(normalize(query))
    : query.test(box.text);
}

export interface ChordInputOptions {
  /** How the chord runs: by releasing Space (default) or by pressing Caps while Space is held. */
  complete?: "release" | "caps";
}

export class Screen {
  private shots: number;

  constructor(
    readonly vnc: VncClient,
    /** Screenshots are numbered in order in this folder, across processes. */
    readonly directory: string,
  ) {
    mkdirSync(directory, { recursive: true });
    this.shots = readdirSync(directory).filter((file) => /^\d+-.*\.png$/.test(file)).length;
  }

  /**
   * Waits for a picture. The VNC server shows a black 1280x720 frame until it has seen input,
   * which a nudge of the pointer provides.
   */
  async wake() {
    await waitFor(
      "the VM's display to show something",
      async () => {
        this.vnc.move(Math.floor(this.vnc.width / 2), Math.floor(this.vnc.height / 2));
        await Bun.sleep(300);
        const { rgb } = await this.vnc.frame();
        return rgb.some((value) => value !== 0) ? true : undefined;
      },
      { timeoutMs: 30_000, intervalMs: 500 },
    );
  }

  /** Saves the screen as `<nn>-<name>.png`, or to `name` when it is an absolute path. */
  async capture(name = "screen"): Promise<string> {
    const path = name.startsWith("/")
      ? name
      : join(this.directory, `${String(++this.shots).padStart(3, "0")}-${slug(name)}.png`);
    await Bun.write(path, await this.vnc.screenshot());
    return path;
  }

  async read(): Promise<TextBox[]> {
    return await recognizeText(await this.capture(join(this.directory, ".ocr.png")));
  }

  async find(query: TextQuery): Promise<TextBox | undefined> {
    return (await this.read()).find((box) => matches(box, query));
  }

  /** Waits until `query` is on screen. A person would also wait, so this is not a test hack. */
  async waitForText(query: TextQuery, options: WaitOptions = {}): Promise<TextBox> {
    return await waitFor(`${String(query)} on screen`, () => this.find(query), {
      timeoutMs: 60_000,
      intervalMs: 1_000,
      ...options,
    });
  }

  async waitForNoText(query: TextQuery, options: WaitOptions = {}) {
    await waitFor(
      `${String(query)} to leave the screen`,
      async () => ((await this.find(query)) ? undefined : true),
      { timeoutMs: 60_000, intervalMs: 1_000, ...options },
    );
  }

  /** Clicks the middle of the text once it is on screen. */
  async clickText(
    query: TextQuery,
    { count, ...options }: WaitOptions & { count?: number } = {},
  ): Promise<TextBox> {
    const box = await this.waitForText(query, options);
    await this.vnc.click(center(box).x, center(box).y, { count });
    return box;
  }

  /** Enters the guest user's password into a macOS authorization prompt, if one came up. */
  async authorize(password = "admin", options: WaitOptions = {}) {
    await this.waitForText("Enter your password", { timeoutMs: 15_000, ...options });
    // The password field has focus.
    await this.vnc.type(`${password}\n`);
    await this.waitForNoText("Enter your password", { timeoutMs: 15_000, ...options });
  }

  /**
   * Types a chord as a user does: Caps+Space enters Chord mode, Caps is let go while Space
   * stays down, the keys are typed, and the chord runs.
   */
  async chord(sequence: string, { complete = "release" }: ChordInputOptions = {}) {
    const { vnc } = this;
    vnc.key(Keysym.CapsLock, true);
    await Bun.sleep(40);
    vnc.key(Keysym.Space, true);
    await Bun.sleep(40);
    vnc.key(Keysym.CapsLock, false);
    await Bun.sleep(80);
    await vnc.type(sequence);
    if (complete === "caps") await vnc.press(Keysym.CapsLock);
    vnc.key(Keysym.Space, false);
    await Bun.sleep(80);
  }
}

export function center(box: TextBox): { x: number; y: number } {
  return { x: Math.round(box.x + box.width / 2), y: Math.round(box.y + box.height / 2) };
}

function slug(name: string): string {
  return name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");
}
