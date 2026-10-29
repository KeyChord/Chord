import { afterAll, expect, test } from "bun:test";
import { renderToString } from "react-dom/server";
import { QueryClient, QueryClientProvider } from "@chord/com.npmjs.tanstack__react-query";
import type { ChordMacroEntry } from "@chord/dev.improve.chord.lib.typeshare";

// Mutations never run during server rendering; TauRPC needs the browser's Proxy constructor.
const previousWindow = globalThis.window;
globalThis.window = { Proxy, screen: {} } as unknown as Window & typeof globalThis;
const { MacrosContent } = await import("#settings-macros-page");
afterAll(() => {
  globalThis.window = previousWindow;
});

function render(entries: ChordMacroEntry[]) {
  return renderToString(
    <QueryClientProvider client={new QueryClient()}>
      <MacrosContent entries={entries} />
    </QueryClientProvider>,
  ).replaceAll("<!-- -->", "");
}

const ENTER = '<span title="Enter"[^>]*>↩</span>';

test("named macros show each Enter key press as a marked line break", () => {
  const html = render([
    { target: { type: "named", value: "login" }, sequence: "ab\\ncd\\n", persistent: true },
  ]);
  // The trailing Enter stays visible even though it ends the last line.
  expect(html).toMatch(new RegExp(`>ab${ENTER}\ncd${ENTER}\n</p>`));
  expect(html).not.toContain("ab\\ncd");
});

test("buffers edit Enter key presses as lines of a multiline field", () => {
  const html = render([
    { target: { type: "buffer", value: 1 }, sequence: "3`d2,i5`ab\\ncd", persistent: false },
  ]);
  const field = /<textarea[^>]*id="macro-buffer-1"[^>]*>([^<]*)<\/textarea>/.exec(html);
  expect(field?.[0]).toContain('rows="2"');
  expect(field?.[1]).toBe("3`d2,i5`ab\ncd");
  expect(html).toMatch(/<textarea[^>]*id="macro-buffer-2"[^>]*rows="1"/);
});

test("named macros show the shortest reference that works when typed and in macro text", () => {
  const html = render([
    { target: { type: "named", value: "size" }, sequence: "1720x864", persistent: true },
    { target: { type: "named", value: "notes" }, sequence: "x", persistent: true },
    { target: { type: "named", value: "1temporary" }, sequence: "y", persistent: false },
  ]);
  expect(html).toContain(">\\size=</code>");
  expect(html).toContain(">\\=notes=</code>");
  expect(html).toContain(">\\=1temporary=</code>");
});
