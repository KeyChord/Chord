import { afterAll, expect, test } from "bun:test";
import { renderToString } from "react-dom/server";
import { ChordLayout, type ChordPackage } from "@chord/dev.improve.chord.lib.typeshare";

// The passive overlay only uses the viewport during server rendering. Effects
// that talk to Tauri do not run, and TauRPC needs the browser's Proxy constructor.
const previousWindow = globalThis.window;
globalThis.window = { innerHeight: 720, Proxy, screen: {} } as unknown as Window & typeof globalThis;
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
      chordPackageManagerData: { packages: [], errors: {}, personalPackages: [], thirdPartyPackages: [] },
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
      chordPackageManagerData: { packages: [], errors: {}, personalPackages: [], thirdPartyPackages: [] },
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
      function packageFor(app: string, name: string): ChordPackage {
        const path = `chords/${app.replaceAll(".", "/")}/macos.toml`;
        return {
          name,
          rawChordsFiles: {},
          globalChords: [],
          compiledChordsFiles: {
            [path]: {
              name, pathslug: path, meta: {}, handlers: [], executables: [],
              chords: [{ rawTrigger: "gh", trigger: { keys: ["KeyG", "KeyH"] }, name, index: 0, actions: [] }],
              chordHints: [{ rawPattern: "gh", pattern: { keys: ["KeyG", "KeyH"] }, description: name }],
            },
          },
        };
      }
      const html = renderToString(<ChordsPageContent
        chordInputData={{ input: ["KeyG", "KeyH"], quotes: [], inserting: false, blocks: [], openBlock: false }}
        chordPanelData={{ isVisible: true, layout }}
        frontmostData={{ frontmostAppBundleId: "dev.fixture", filePickerActive }}
        chordPackageManagerData={{ errors: {}, personalPackages: [], thirdPartyPackages: [], packages: [packageFor("dev.fixture", "Host navigation"), packageFor("com.apple.finder", "Finder navigation")] }}
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
      chordPackageManagerData={{ errors: {}, personalPackages: [], thirdPartyPackages: [], packages: [{
        name: "@fixture/test",
        rawChordsFiles: {},
        globalChords: [],
        compiledChordsFiles: {
          [path]: {
            name: "fixture", pathslug: path, meta: {}, handlers: [], executables: [],
            chords: [{ rawTrigger: "/w]", trigger: { keys: ["Slash", "KeyW", "BracketRight"] }, name: "Close tab", index: 0, actions: [] }],
            chordHints: [],
          },
        },
      }] }}
      chordExecutionData={{ loops: [], metaCommands: [], haltLoopTemplate: "0h{id}" }}
      chordMacrosData={{ entries: [] }}
    />);
    expect(html).toContain("↩");
    if (layout === ChordLayout.SidebarLeft) {
      expect(html).toContain("Repeat 3 times · `d2,i13` · Step 2");
      // The next column offers the rest of the chord being typed.
      expect(html).toContain("Close tab");
    }
  });

  test(`${layout} shows insert-mode text in place of the open quote`, () => {
    const html = renderToString(<ChordsPageContent
      chordInputData={{ input: ["Digit3", "Quote"], quotes: ["Hello, World"], inserting: true, blocks: [], openBlock: false, description: "Insert mode · Caps+Space closes the quote · Esc cancels" }}
      chordPanelData={{ isVisible: true, layout }}
      frontmostData={{ filePickerActive: false }}
      chordPackageManagerData={{ packages: [], errors: {}, personalPackages: [], thirdPartyPackages: [] }}
      chordExecutionData={{ loops: [], metaCommands: [], haltLoopTemplate: "0h{id}" }}
      chordMacrosData={{ entries: [] }}
    />);
    expect(html).toContain("Hello, World");
    expect(html).toContain("Caps+Space closes the quote");
  });

  const renderInput = (chordInputData: Parameters<typeof ChordsPageContent>[0]["chordInputData"]) => renderToString(<ChordsPageContent
    chordInputData={chordInputData}
    chordPanelData={{ isVisible: true, layout }}
    frontmostData={{ filePickerActive: false }}
    chordPackageManagerData={{ packages: [], errors: {}, personalPackages: [], thirdPartyPackages: [] }}
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
