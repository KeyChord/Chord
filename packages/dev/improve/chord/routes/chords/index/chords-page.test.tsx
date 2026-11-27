import { afterAll, expect, test } from "bun:test";
import { renderToString } from "react-dom/server";
import { ChordLayout, type HintPanelPackage } from "@chord/dev.improve.chord.lib.typeshare";

// The passive overlay only uses the viewport during server rendering. Effects
// that talk to Tauri do not run, and TauRPC needs the browser's Proxy constructor.
const previousWindow = globalThis.window;
globalThis.window = { innerHeight: 720, Proxy, screen: {}, navigator: { userAgent: "Macintosh" } } as unknown as Window & typeof globalThis;
const { ChordsPageContent } = await import("#chords-page");
afterAll(() => {
  globalThis.window = previousWindow;
});

for (const layout of [ChordLayout.SidebarLeft, ChordLayout.FloatingBottom]) {
  test(`${layout} displays macros before arrow escapes when backslash is typed`, () => {
    const props: Parameters<typeof ChordsPageContent>[0] = {
      chordInputData: { input: ["Backslash"], quotes: [], inserting: false, blocks: [], openBlock: false },
      chordPanelData: { isVisible: true, layout },
      frontmostData: { filePickerActive: false },
      hintPanelData: { packages: [] },
      chordExecutionData: { loops: [], metaCommands: [], haltLoopTemplate: "0h{id}" },
      chordMacrosData: {
        entries: [
          {
            target: { type: "named", value: "window-size" },
            sequence: "1720x864",
            persistent: true,
          },
          { target: { type: "buffer", value: 1 }, sequence: "800x600", persistent: false },
        ],
      },
    };
    const html = renderToString(<ChordsPageContent {...props} />);
    expect(html).toContain("Escape suggestions");
    expect(html).toContain("1720x864");
    expect(html).toContain("800x600");
    expect(html.indexOf("\\1")).toBeLessThan(html.indexOf("\\window-size="));
    expect(html.indexOf("\\window-size=")).toBeLessThan(html.indexOf("\\up"));
  });

  test(`${layout} keeps failed input on screen in red with its error`, () => {
    const props: Parameters<typeof ChordsPageContent>[0] = {
      chordInputData: {
        input: [],
        quotes: [],
        inserting: false, blocks: [], openBlock: false,
        selectedInputEvent: { input: ["KeyX", "KeyQ"], quotes: [] },
        error: "repeat count must fit in an unsigned 32-bit integer",
      },
      chordPanelData: { isVisible: true, layout },
      frontmostData: { filePickerActive: false },
      hintPanelData: { packages: [] },
      chordExecutionData: { loops: [], metaCommands: [], haltLoopTemplate: "0h{id}" },
      chordMacrosData: { entries: [] },
    };
    const failed = renderToString(<ChordsPageContent {...props} />);
    expect(failed).toContain('role="alert"');
    expect(failed).toContain("Repeat count must fit in an unsigned 32-bit integer");
    expect(failed).toContain("border-red-400/90");
    expect(failed).not.toContain("border-emerald-400/90");
    for (const token of ["X", "Q"]) expect(failed).toContain(`>${token}</kbd>`);

    const executed = renderToString(<ChordsPageContent {...props} chordInputData={{ input: [], quotes: [], inserting: false, blocks: [], openBlock: false, selectedInputEvent: { input: ["KeyX", "KeyQ"], quotes: [] } }} />);
    expect(executed).not.toContain('role="alert"');
    expect(executed).not.toContain("border-red-400/90");
  });

  for (const filePickerActive of [false, true]) {
    test(`${layout} follows native file-picker priority when active=${filePickerActive}`, () => {
      function packageFor(app: string, name: string): HintPanelPackage {
        const path = `chords/${app.replaceAll(".", "/")}/macos.toml`;
        return {
          name,
          globalChords: [],
          chordsFiles: {
            [path]: {
              chords: [{ rawTrigger: "gh", trigger: { keys: ["KeyG", "KeyH"] }, name, index: 0, actions: [], fallback: false }],
              chordHints: [{ rawPattern: "gh", pattern: { keys: ["KeyG", "KeyH"] }, description: name }],
            },
          },
        };
      }
      const html = renderToString(<ChordsPageContent
        chordInputData={{ input: ["KeyG", "KeyH"], quotes: [], inserting: false, blocks: [], openBlock: false }}
        chordPanelData={{ isVisible: true, layout }}
        frontmostData={{ frontmostAppBundleId: "dev.fixture", filePickerActive }}
        hintPanelData={{ packages: [packageFor("dev.fixture", "Host navigation"), packageFor("com.apple.finder", "Finder navigation")] }}
        chordExecutionData={{ loops: [], metaCommands: [], haltLoopTemplate: "0h{id}" }}
        chordMacrosData={{ entries: [] }}
      />);
      expect(html).toContain(filePickerActive ? "Finder navigation" : "Host navigation");
      expect(html).not.toContain(filePickerActive ? "Host navigation" : "Finder navigation");
    });
  }
  test(`${layout} completes the chord after Enter in a configured sequence`, () => {
    const path = "chords/dev/fixture/macos.toml";
    const html = renderToString(<ChordsPageContent
      // 3`d2,i13`/w]⏎/w
      chordInputData={{ input: ["Digit3", "Backquote", "KeyD", "Digit2", "Comma", "KeyI", "Digit1", "Digit3", "Backquote", "Slash", "KeyW", "BracketRight", "Enter", "Slash", "KeyW"], quotes: [], inserting: false, blocks: [], openBlock: false }}
      chordPanelData={{ isVisible: true, layout }}
      frontmostData={{ frontmostAppBundleId: "dev.fixture", filePickerActive: false }}
      hintPanelData={{ packages: [{
        name: "@fixture/test",
        globalChords: [],
        chordsFiles: {
          [path]: {
            chords: [{ rawTrigger: "/w]", trigger: { keys: ["Slash", "KeyW", "BracketRight"] }, name: "Close tab", index: 0, actions: [], fallback: false }],
            chordHints: [],
          },
        },
      }] }}
      chordExecutionData={{ loops: [], metaCommands: [], haltLoopTemplate: "0h{id}" }}
      chordMacrosData={{ entries: [] }}
    />);
    expect(html).toContain('aria-label="Return"');
    if (layout === ChordLayout.SidebarLeft) {
      // No label row above the keys.
      expect(html).not.toContain("Repeat 3 times");
      // The next column offers the rest of the chord being typed.
      expect(html).toContain("Close tab");
    }
  });

  test(`${layout} displays unmatched keys while keeping launcher chords available`, () => {
    const path = "chords/dev/fixture/macos.toml";
    const render = (input: string[], executed = false) => renderToString(<ChordsPageContent
      chordInputData={{
        input: executed ? [] : input,
        quotes: [], inserting: false, blocks: [], openBlock: false,
        selectedInputEvent: executed ? { input, quotes: [] } : undefined,
      }}
      chordPanelData={{ isVisible: true, layout }}
      frontmostData={{ frontmostAppBundleId: "dev.fixture", filePickerActive: false }}
      hintPanelData={{ packages: [{
        name: "@fixture/test",
        globalChords: [],
        chordsFiles: {
          [path]: {
            chords: [
              { rawTrigger: "/f", trigger: { keys: ["Slash", "KeyF"] }, name: "Open Finder", index: 0, actions: [], fallback: false },
              { rawTrigger: "/b", trigger: { keys: ["Slash", "KeyB"] }, name: "Open Browser", index: 1, actions: [], fallback: false },
              { rawTrigger: "/4p", trigger: { keys: ["Slash", "Digit4", "KeyP"] }, name: "Open Project", index: 2, actions: [], fallback: false },
            ],
            chordHints: [{ rawPattern: "/", pattern: { keys: ["Slash"] }, description: "Launch" }],
          },
        },
      }] }}
      chordExecutionData={{ loops: [], metaCommands: [], haltLoopTemplate: "0h{id}" }}
      chordMacrosData={{ entries: [] }}
    />);

    const notion = render(["Slash", "KeyN", "KeyO", "KeyT", "KeyI", "KeyO", "KeyN"]);
    expect(notion).toContain(">/</kbd>");
    expect(notion).toContain(">NOTION</kbd>");
    expect(notion).not.toContain(">N</kbd>");
    expect(render(["Slash", "KeyN", "KeyO"])).toContain(">NO</kbd>");

    const onePassword = ["Slash", "Digit1", "KeyP"];
    const launcherText = render(onePassword);
    expect(launcherText).toContain(">/</kbd>");
    expect(launcherText).toContain(">1P</kbd>");
    expect(launcherText).not.toContain(">1</kbd>");
    expect(launcherText).not.toContain(">P</kbd>");
    expect(render(["Digit3", ...onePassword])).toMatch(/>3(?:<\/span>)?<\/kbd>.*>1P<\/kbd>/);
    expect(render(["Slash", "KeyP", "Digit1"])).toContain(">P1</kbd>");
    expect(render(["Slash", "Digit4", "KeyP"]).match(/>(?:4|P|4P)<\/kbd>/g)).toEqual([">4</kbd>", ">P</kbd>"]);
    const punctuation = ["Slash", "Period", "Digit1", "Minus", "KeyP", "Quote", "KeyS"];
    expect(render(punctuation)).toContain(">.1-P&#x27;S</kbd>");
    const withArrow = render(["Slash", "KeyA", "ArrowUp", "KeyB"]);
    expect(withArrow).toMatch(/<kbd[^>]*><span>A[^]*?aria-label="Up Arrow"[^]*?B<\/span><\/kbd>/);
    if (layout === ChordLayout.FloatingBottom) {
      expect(render(onePassword, true)).toContain(">1P</kbd>");
      expect(render(punctuation, true)).toContain(">.1-P&#x27;S</kbd>");
    }

    const finder = render(["Slash", "KeyF"]);
    expect(finder).toContain(">/</kbd>");
    expect(finder).toContain(">F</kbd>");
    if (layout === ChordLayout.SidebarLeft) {
      expect(render(["Slash"])).toContain("Open Finder");
      expect(notion.match(/>(?:NOTION|B|F)<\/kbd>/g)).toEqual([">NOTION</kbd>", ">B</kbd>", ">F</kbd>"]);
      expect(render(["Slash", "KeyZ"]).match(/>(?:Z|B|F)<\/kbd>/g)).toEqual([">Z</kbd>", ">B</kbd>", ">F</kbd>"]);
      expect(render(["Slash", "Period"]).match(/>(?:\.|B|F)<\/kbd>/g)).toEqual([">.</kbd>", ">B</kbd>", ">F</kbd>"]);
      expect(finder.match(/>(?:B|F)<\/kbd>/g)).toEqual([">B</kbd>", ">F</kbd>"]);
    }
  });

  test(`${layout} shows insert-mode text in place of the open quote`, () => {
    const html = renderToString(<ChordsPageContent
      chordInputData={{ input: ["Digit3", "Quote"], quotes: ["Hello, World"], inserting: true, blocks: [], openBlock: false, description: "Insert mode · Caps+Space closes the quote · Esc cancels" }}
      chordPanelData={{ isVisible: true, layout }}
      frontmostData={{ filePickerActive: false }}
      hintPanelData={{ packages: [] }}
      chordExecutionData={{ loops: [], metaCommands: [], haltLoopTemplate: "0h{id}" }}
      chordMacrosData={{ entries: [] }}
    />);
    expect(html).toContain("Hello, World");
    expect(html).toContain("Caps+Space closes the quote");
  });

  test(`${layout} uses More for an unlabeled default placeholder prefix`, () => {
    const path = "chords/macos.toml";
    const render = (prefix: string, key: string, label?: string) => renderToString(<ChordsPageContent
      chordInputData={{ input: layout === ChordLayout.FloatingBottom ? [key] : [], quotes: [], inserting: false, blocks: [], openBlock: false }}
      chordPanelData={{ isVisible: true, layout }}
      frontmostData={{ filePickerActive: false }}
      hintPanelData={{ packages: [{
        name: "@fixture/test",
        globalChords: [],
        chordsFiles: {
          [path]: {
            chords: [],
            chordHints: [
              { rawPattern: `${prefix}m`, pattern: { keys: [key, "KeyM"] }, description: "Menu" },
              ...(label ? [{ rawPattern: prefix, pattern: { keys: [key] }, description: label }] : []),
            ],
          },
        },
      }] }}
      chordExecutionData={{ loops: [], metaCommands: [], haltLoopTemplate: "0h{id}" }}
      chordMacrosData={{ entries: [] }}
      defaultGlobalPrefixForPlaceholders={prefix}
    />);

    expect(render(";", "Semicolon")).toContain(">More</div>");
    expect(render("=", "Equal")).toContain(">More</div>");
    const overridden = render("=", "Equal", "Utilities");
    expect(overridden).toContain(">Utilities</div>");
    expect(overridden).not.toContain(">More</div>");
  });

  test(`${layout} describes a key that also prefixes a range hint`, () => {
    const path = "chords/macos.toml";
    const html = renderToString(<ChordsPageContent
      chordInputData={{ input: ["Equal"], quotes: [], inserting: false, blocks: [], openBlock: false }}
      chordPanelData={{ isVisible: true, layout }}
      frontmostData={{ filePickerActive: false }}
      hintPanelData={{ packages: [{
        name: "@fixture/test",
        globalChords: [],
        chordsFiles: {
          [path]: {
            chords: [],
            chordHints: [
              { rawPattern: "=", pattern: { keys: ["Equal"] }, description: "Menu/Tray" },
              { rawPattern: '="1-9"', pattern: { range: { prefix: ["Equal"], keys: ["Digit1", "Digit2"] } }, description: "Tray: Select Icon" },
            ],
          },
        },
      }] }}
      chordExecutionData={{ loops: [], metaCommands: [], haltLoopTemplate: "0h{id}" }}
      chordMacrosData={{ entries: [] }}
    />);
    expect(html).toContain("Menu/Tray");
  });

  test(`${layout} shows one repeat-count row at the start of a sequence`, () => {
    const path = "chords/com/leonsilicon/chord/macos.toml";
    const html = renderToString(<ChordsPageContent
      chordInputData={{ input: [], quotes: [], inserting: false, blocks: [], openBlock: false }}
      chordPanelData={{ isVisible: true, layout }}
      frontmostData={{ filePickerActive: false }}
      hintPanelData={{ packages: [{
        name: "@keychord/chords-chord",
        globalChords: [],
        chordsFiles: {
          [path]: {
            chords: [],
            chordHints: [{ rawPattern: '"1-9"', pattern: { range: { prefix: [], keys: ["Digit1", "Digit2", "Digit3", "Digit4", "Digit5", "Digit6", "Digit7", "Digit8", "Digit9"] } }, description: "Repeat count" }],
          },
        },
      }] }}
      chordExecutionData={{ loops: [], metaCommands: [], haltLoopTemplate: "0h{id}" }}
      chordMacrosData={{ entries: [] }}
    />);
    expect(html).not.toContain(">1</kbd>");
    if (layout === ChordLayout.SidebarLeft) expect(html).toContain(">1-9</kbd>");
  });

  test(`${layout} moves a hidden Hint Panel off screen until Tab shows it`, () => {
    const render = (isVisible: boolean) => renderToString(<ChordsPageContent
      chordInputData={{ input: [], quotes: [], inserting: false, blocks: [], openBlock: false }}
      chordPanelData={{ isVisible, layout }}
      frontmostData={{ filePickerActive: false }}
      hintPanelData={{ packages: [] }}
      chordExecutionData={{ loops: [], metaCommands: [], haltLoopTemplate: "0h{id}" }}
      chordMacrosData={{ entries: [] }}
    />);
    const offScreen = layout === ChordLayout.FloatingBottom ? "translateY(calc(100% + 40px))" : "translateX(calc(-100% - 40px))";
    expect(render(false)).toContain(`transform:${offScreen};opacity:0`);
    expect(render(true)).not.toContain(offScreen);
    expect(render(true)).toContain("opacity:1");
  });

  const renderInput = (chordInputData: Parameters<typeof ChordsPageContent>[0]["chordInputData"]) => renderToString(<ChordsPageContent
    chordInputData={chordInputData}
    chordPanelData={{ isVisible: true, layout }}
    frontmostData={{ filePickerActive: false }}
    hintPanelData={{ packages: [] }}
    chordExecutionData={{ loops: [], metaCommands: [], haltLoopTemplate: "0h{id}" }}
    chordMacrosData={{ entries: [] }}
  />);

  test(`${layout} marks the blocks that stay after a run`, () => {
    // ⇧g, then d<caps>: `gd` ran and the G block stays.
    const html = renderInput({ input: ["KeyG"], quotes: [], inserting: false, blocks: [1], openBlock: false, selectedInputEvent: { input: ["KeyG", "KeyD"], quotes: [] } });
    expect(html).toContain("border-sky-400/90");
    expect(html).not.toContain("border-dashed");
    if (layout === ChordLayout.FloatingBottom) {
      // The chord that ran is still shown, its block marked.
      expect(html).toContain("border-emerald-400/90");
      for (const token of ["G", "D"]) expect(html).toContain(`>${token}</kbd>`);
    }
  });

  test(`${layout} shows keys typed after the blocks as the buffer`, () => {
    const html = renderInput({ input: ["KeyG", "KeyA", "KeyE"], quotes: [], inserting: false, blocks: [1, 2], openBlock: false });
    expect(html).toContain("border-sky-400/90");
    expect(html).toContain(">E</kbd>");
    // Adjacent blocks G and A stay apart.
    expect(html).toContain(layout === ChordLayout.FloatingBottom ? "mr-1.5" : "ml-3");
    if (layout === ChordLayout.FloatingBottom) expect(html).not.toContain("border-emerald-400/90");
  });

  test(`${layout} shows the block being typed with Shift held as open`, () => {
    const html = renderInput({ input: ["KeyG", "KeyA"], quotes: [], inserting: false, blocks: [], openBlock: true });
    expect(html).toContain("border-dashed");
  });
}

test("the floating panel keeps insert-mode quote text between its quote keys", () => {
  const html = renderToString(<ChordsPageContent
    chordInputData={{ input: ["Digit3", "Quote", "Quote"], quotes: ["Hello, World"], inserting: false, blocks: [], openBlock: false }}
    chordPanelData={{ isVisible: true, layout: ChordLayout.FloatingBottom }}
    frontmostData={{ filePickerActive: false }}
    hintPanelData={{ packages: [] }}
    chordExecutionData={{ loops: [], metaCommands: [], haltLoopTemplate: "0h{id}" }}
    chordMacrosData={{ entries: [] }}
  />);
  expect(html).toMatch(/>&#x27;<\/kbd><kbd[^>]*>Hello, World<\/kbd><kbd[^>]*>&#x27;<\/kbd>/);
});

test("keys before the last three columns fold into a trail beside them, unless turned off", () => {
  const path = "chords/dev/fixture/macos.toml";
  const sequence = "abcdefghijklmnopqrstuv";
  const keys = Array.from(sequence, letter => `Key${letter.toUpperCase()}`);
  const render = (input: string[]) => renderToString(<ChordsPageContent
    chordInputData={{ input, quotes: [], inserting: false, blocks: [], openBlock: false }}
    chordPanelData={{ isVisible: true, layout: ChordLayout.SidebarLeft }}
    frontmostData={{ frontmostAppBundleId: "dev.fixture", filePickerActive: false }}
    hintPanelData={{ packages: [{ name: "@fixture/deep", globalChords: [], chordsFiles: { [path]: {
      chords: [{ rawTrigger: sequence, trigger: { keys }, name: "Deep chord", index: 0, actions: [], fallback: false }],
      chordHints: [],
    } } }] }}
    chordExecutionData={{ loops: [], metaCommands: [], haltLoopTemplate: "0h{id}" }}
    chordMacrosData={{ entries: [] }}
  />);
  expect(render(keys.slice(0, 2))).not.toContain('aria-label="Earlier keys"');
  const deep = render(keys.slice(0, -1));
  expect(deep).toContain('aria-label="Earlier keys"');
  expect(deep).toContain('aria-label="16 earlier key groups omitted"');
  expect(deep).toContain("Deep chord");

  const storage = (window as { localStorage?: unknown }).localStorage;
  (window as { localStorage?: unknown }).localStorage = { getItem: (key: string) => key === "chord-collapse-earlier-keys" ? "false" : null };
  try {
    expect(render(keys.slice(0, -1))).not.toContain('aria-label="Earlier keys"');
  } finally {
    (window as { localStorage?: unknown }).localStorage = storage;
  }
});
