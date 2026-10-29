// Chords typed on the VM's keyboard, run by the installed production app.
import { afterAll, beforeAll, beforeEach, expect } from "bun:test";
import { Session } from "#session";

const MARKERS = "/tmp/chord-e2e";

const PACKAGE = {
  "package.json": JSON.stringify({ name: "e2e", type: "module" }),
  "chords/macos.toml": `name = "e2e"

[on.mark]
file = "mark.js"

[chords]
"/er" = { name = "E2E: shell", shell = "echo shell > ${MARKERS}/shell" }
"/ej" = { name = "E2E: JavaScript", "@input:mark" = ["js"] }
`,
  "js/mark.js": `export default function buildMark() {
  return async function mark(name) {
    await Bun.write("${MARKERS}/" + name, JSON.stringify({ input: this.input }));
  };
}
`,
};

const session = await Session.open();

beforeAll(() => session.restartChord({ e2e: PACKAGE }), 180_000);
beforeEach(() => session.guest.run(`rm -rf ${MARKERS} && mkdir -p ${MARKERS}`));
afterAll(() => session.close());

session.test("a chord runs when Space is released", async () => {
  await session.chord("/er");
  expect(await session.waitForFile(`${MARKERS}/shell`)).toBe("shell\n");
});

session.test("Caps runs the chord while Space is held", async () => {
  await session.chord("/er", { complete: "caps" });
  expect(await session.waitForFile(`${MARKERS}/shell`)).toBe("shell\n");
});

session.test("a JavaScript handler runs in the package's Bun worker", async () => {
  await session.chord("/ej");
  expect(JSON.parse(await session.waitForFile(`${MARKERS}/js`))).toEqual({ input: "/ej" });
});

session.test("keys typed in Chord mode do not reach the focused app", async () => {
  const recorder = await session.openRecorder(`${MARKERS}/typed`);
  try {
    await session.vnc.type("ab");
    await session.chord("/er");
    await session.waitForFile(`${MARKERS}/shell`);
    await session.vnc.type("cd\n");
    expect(await session.waitForFile(`${MARKERS}/typed`)).toBe("abcd\n");
  } finally {
    await recorder.close();
  }
});
