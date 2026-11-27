import { afterAll, expect, test } from "bun:test";
import { renderToString } from "react-dom/server";
import { QueryClient, QueryClientProvider } from "@chord/com.npmjs.tanstack__react-query";
import type {
  ChordPackage,
  ChordPackageManagerState,
  ThirdPartyPackage,
} from "@chord/dev.improve.chord.lib.typeshare";

const previousWindow = globalThis.window;
globalThis.window = {
  Proxy,
  navigator: { userAgent: "Macintosh" },
  screen: {},
} as unknown as Window & typeof globalThis;
const { SettingsChordsPageContent } = await import("#settings-chords-page");
const {
  applyPackageOrder,
  ChordKeys,
  ChordLibrary,
  listLibraryChords,
  listLibraryPackages,
  movePackage,
  packageTitle,
} = await import("#chord-library");
const { listPlaceholders, PackagePlaceholders } =
  await import("@chord/dev.improve.chord.routes.settings._components.package-placeholders");
afterAll(() => {
  globalThis.window = previousWindow;
});

function pkg(name: string): ChordPackage {
  return {
    name,
    rawChordsFiles: {},
    globalChords: [],
    compiledChordsFiles: {
      "chords/macos.toml": {
        name,
        pathslug: "chords/macos.toml",
        meta: {},
        handlers: [],
        executables: [],
        placeholders: [],
        chordHints: [],
        chords: [
          {
            name: `${name} action`,
            rawTrigger: "/t",
            trigger: { keys: ["/", "t"] },
            index: 0,
            actions: [],
            fallback: false,
          },
        ],
      },
    },
  };
}

function render(
  packages: ChordPackage[],
  thirdPartyPackages: ThirdPartyPackage[] = [],
  builtinPackages: string[] = [],
  configurePackage?: string,
) {
  const state: ChordPackageManagerState = {
    packages,
    errors: {},
    thirdPartyPackages,
    builtinPackages,
    personalPackages: [{ dirname: "work", name: "@me/work" }],
  };
  const client = new QueryClient();
  client.setQueryData(["tauri-observable-states"], {
    "chord-package-manager": state,
    settings: { defaultGlobalPrefixForPlaceholders: ";" },
  });
  client.setQueryData(["personal-packages"], {
    root: "/Users/me/.chord/packages",
    packages: [
      { dirname: "work", path: "/Users/me/.chord/packages/work", files: ["chords/macos.toml"] },
    ],
  });
  return renderToString(
    <QueryClientProvider client={client}>
      <SettingsChordsPageContent
        chordPackageManagerData={state}
        configurePackage={configurePackage}
      />
    </QueryClientProvider>,
  ).replaceAll("<!-- -->", "");
}

test("Packages lists installed packages while Local Chords keeps personal editing separate", () => {
  const packages = [pkg("@keychord/finder"), pkg("@me/work"), pkg("@keychord/terminal")];
  const rows = listLibraryPackages(packages, new Set(["@me/work"]), {});
  expect(rows.map((row) => row.name)).toEqual([
    "@me/work",
    "@keychord/finder",
    "@keychord/terminal",
  ]);
  expect(packages[0]?.name).toBe("@keychord/finder");
  const html = render(packages);
  expect(html).toContain('role="tablist"');
  expect(html).toMatch(/role="tab"[^>]*aria-selected="true"[^>]*>Packages</);
  expect(html).toMatch(/role="tab"[^>]*>Local Chords</);
  expect(html).toMatch(/role="tab"[^>]*>Third-Party</);
  expect(html).toContain("Default global prefix for placeholders");
  expect(html).toMatch(/<input[^>]+maxLength="1"[^>]+value=";"/i);
  expect(html).toContain("2 packages");
  expect(html).toContain('aria-label="Open @keychord/finder"');
  expect(html).not.toContain('aria-label="Open @me/work"');
  expect(html).toContain('aria-label="Local Chords"');
  expect(html).toContain('aria-label="Edit chords/macos.toml"');
  expect(html).not.toContain('title="@me/work action"');
  expect(html).not.toContain('title="@keychord/finder action"');
});

test("Local Chords remains editable when no external packages are installed", () => {
  const html = render([pkg("@me/work")]);
  expect(html).not.toContain('aria-label="Open @me/work"');
  expect(html).toContain("Your package library is empty");
  expect(html).toMatch(/role="tab"[^>]*>Local Chords</);
  expect(html).toContain('aria-label="Edit chords/macos.toml"');
  // Local Chords covers personal folders, so the Packages tab no longer manages sources.
  expect(html).not.toContain("Manage sources");
});

test("opening Chords to configure a package shows only that package", () => {
  const finder = "@keychord/chords-finder";
  const terminal = "@keychord/chords-terminal";
  const html = render([pkg(finder), pkg(terminal)], [], [], terminal);

  expect(html).toContain(`aria-label="${terminal} details"`);
  expect(html).toContain(`title="${terminal} action"`);
  expect(html).toContain(">Packages</button>");
  expect(html).not.toContain(`aria-label="Open ${finder}"`);
  expect(html).not.toContain(`title="${finder} action"`);

  // Without a package to configure, the library lists every package and no chords.
  const library = render([pkg(finder), pkg(terminal)]);
  expect(library).toContain(`aria-label="Open ${finder}"`);
  expect(library).toContain(`aria-label="Open ${terminal}"`);
  expect(library).not.toContain(`title="${terminal} action"`);
});

test("an empty personal package stays editable in Local Chords", () => {
  const empty = { ...pkg("@me/work"), compiledChordsFiles: {} };
  const html = render([pkg("@keychord/finder"), empty]);
  expect(html).not.toContain('aria-label="Open @me/work"');
  expect(html).toContain('aria-label="Edit chords/macos.toml"');
  expect(html).toContain("Add File");
});

test("blocked packages are reviewable in the Chords tab without appearing as enabled chords", () => {
  const html = render(
    [pkg("@keychord/finder")],
    [
      {
        name: "@untrusted/test",
        path: "/Users/me/.chord/packages/downloaded",
        identity: "location-identity",
        approved: false,
      },
    ],
  );
  expect(html).toContain('aria-label="Third-party packages"');
  expect(html).toContain("1 awaiting approval");
  expect(html).toContain('aria-label="Review @untrusted/test for approval"');
  expect(html).toContain('title="/Users/me/.chord/packages/downloaded"');
  expect(html).toContain("~/.chord/packages/downloaded");
  expect(html).toContain("Install a third-party package");
  expect(html).not.toContain('aria-label="Open @untrusted/test"');
  expect(html).not.toContain('aria-label="Select @untrusted/test"');
  expect(html).not.toContain('aria-label="Search third-party packages"');
  // Approvals have their own tab, which flags the packages waiting for review.
  expect(html).toMatch(/role="tab"[^>]*>Third-Party<span[^>]*aria-label="1 awaiting approval"/);
  expect(html.indexOf('aria-label="Chord packages"')).toBeLessThan(
    html.indexOf('aria-label="Third-party packages"'),
  );
});

test("approved packages are listed in the Third-Party tab with Revoke", () => {
  const html = render(
    [],
    [
      {
        name: "test",
        path: "/Users/me/.chord/packages/test",
        identity: "test-identity",
        approved: true,
      },
    ],
  );
  expect(html).toContain("Approved packages (1)");
  expect(html).toContain('aria-label="Revoke approval for test"');
  expect(html).toContain("Manage which packages can run code.");
  // Nothing awaits approval, so the tab carries no badge.
  expect(html).not.toContain("awaiting approval");
  expect(html.indexOf('aria-label="Chord packages"')).toBeLessThan(
    html.indexOf("Default global prefix"),
  );
});

test("blocked packages stay alphabetized and longer approval queues expose search and selection", () => {
  const html = render(
    [],
    [
      { name: "approved", path: "/p/approved", identity: "approved", approved: true },
      ...["zeta", "gamma", "alpha", "delta", "beta"].map((name) => ({
        name,
        path: `/p/${name}`,
        identity: name,
        approved: false,
      })),
    ],
  );
  expect(html).toContain("5 awaiting approval");
  expect(html).toContain('aria-label="Search third-party packages"');
  expect(html).toContain('aria-label="Select all visible third-party packages"');
  const order = ["alpha", "beta", "delta", "gamma", "zeta"].map((name) =>
    html.indexOf(`aria-label="Select ${name}"`),
  );
  expect(order.every((index) => index >= 0)).toBe(true);
  expect(order).toEqual(order.toSorted((a, b) => a - b));
  // Approved packages are listed after the queue.
  expect(html.indexOf('aria-label="Select approved"')).toBeGreaterThan(order.at(-1)!);
  expect(html).not.toContain('aria-label="Bulk actions"');
});

test("empty approval queues keep the installation instructions available", () => {
  const html = render([]);
  expect(html).toContain("No third-party packages installed.");
  expect(html).toContain("Install a third-party package");
  expect(html).not.toContain('aria-label="Select all visible third-party packages"');
});

test("each package aggregates its files, chord count, and app scopes", () => {
  const source = pkg("finder");
  const pathslug = "chords/com/apple/finder/macos.toml";
  source.compiledChordsFiles[pathslug] = {
    ...source.compiledChordsFiles["chords/macos.toml"]!,
    pathslug,
    name: "Finder",
  };
  const empty = { ...pkg("empty"), compiledChordsFiles: {} };
  const rows = listLibraryPackages([source, empty], new Set(), {});
  expect(rows).toHaveLength(2);
  expect(rows[0]?.fileCount).toBe(2);
  expect(rows[0]?.chords).toHaveLength(2);
  expect(rows[0]?.appLabels).toEqual(["Every app", "Finder"]);
  expect(rows[0]?.appId).toBeUndefined();
  expect(rows[1]?.chords).toHaveLength(0);
  expect(rows[1]?.fileCount).toBe(0);
});

test("app-scoped chords use friendly names while global chords remain available everywhere", () => {
  const source = pkg("finder");
  const pathslug = "chords/com/apple/finder/macos.toml";
  const file = { ...source.compiledChordsFiles["chords/macos.toml"]!, pathslug, name: "Finder" };
  source.compiledChordsFiles = { [pathslug]: file };
  const fallback = listLibraryChords([source], new Set(), {});
  expect(fallback[0]?.appId).toBe("com.apple.finder");
  expect(fallback[0]?.appLabel).toBe("Finder");
  const metadata = {
    "com.apple.finder": { bundleId: "com.apple.finder", displayName: "My Finder" },
  };
  expect(listLibraryChords([source], new Set(), metadata)[0]?.appLabel).toBe("My Finder");
  file.chords.push({
    ...file.chords[0]!,
    name: "New window",
    rawTrigger: "nw",
    trigger: { keys: ["n", "w"] },
  });
  source.globalChords = [
    { packageName: source.name, chordsFilePathslug: pathslug, chord: file.chords[0]! },
  ];
  const global = listLibraryChords([source], new Set(), metadata);
  expect(global[0]?.appId).toBeUndefined();
  expect(global[0]?.appLabel).toBe("Every app");
  expect(global[1]?.appId).toBe("com.apple.finder");
});

test("packages keep their native rank, with installed ones above built-ins", () => {
  const names = [
    "@keychord/chords-chord",
    "@keychord/raycast",
    "@me/work",
    "@keychord/warp",
    "@keychord/finder",
    "@keychord/spotlight",
  ];
  const rows = listLibraryPackages(
    names.map(pkg),
    new Set(["@me/work"]),
    {},
    new Set(["@keychord/finder", "@keychord/spotlight"]),
  );
  expect(rows.map((row) => [row.name, row.rank])).toEqual([
    ["@keychord/chords-chord", "core"],
    ["@me/work", "personal"],
    ["@keychord/raycast", "installed"],
    ["@keychord/warp", "installed"],
    ["@keychord/finder", "builtin"],
    ["@keychord/spotlight", "builtin"],
  ]);

  // Moves stay within their group, and the saved order lists only packages the user ranks.
  const moved = ["@keychord/warp", "@keychord/raycast", "@keychord/finder", "@keychord/spotlight"];
  expect(movePackage(rows, "@keychord/warp", "@keychord/raycast", "before")).toEqual(moved);
  expect(movePackage(rows, "@keychord/raycast", "@keychord/warp", "after")).toEqual(moved);
  expect(movePackage(rows, "@keychord/spotlight", "@keychord/finder", "before")).toEqual([
    "@keychord/raycast",
    "@keychord/warp",
    "@keychord/spotlight",
    "@keychord/finder",
  ]);
  // Dropping a package where it already is changes nothing.
  expect(movePackage(rows, "@keychord/warp", "@keychord/raycast", "after")).toBeUndefined();
  expect(movePackage(rows, "@keychord/raycast", "@keychord/warp", "before")).toBeUndefined();
  expect(movePackage(rows, "@keychord/warp", "@keychord/warp", "before")).toBeUndefined();
  // Packages never cross groups, and core and personal packages aren't ranked.
  expect(movePackage(rows, "@keychord/finder", "@keychord/warp", "after")).toBeUndefined();
  expect(movePackage(rows, "@keychord/raycast", "@keychord/finder", "before")).toBeUndefined();
  expect(movePackage(rows, "@keychord/chords-chord", "@keychord/raycast", "after")).toBeUndefined();
  expect(movePackage(rows, "@me/work", "@keychord/raycast", "after")).toBeUndefined();

  // While the order saves, the rows show it.
  expect(applyPackageOrder(rows, moved).map((row) => row.name)).toEqual([
    "@keychord/chords-chord",
    "@me/work",
    "@keychord/warp",
    "@keychord/raycast",
    "@keychord/finder",
    "@keychord/spotlight",
  ]);
});

test("moves among many packages land exactly where they are dropped", () => {
  const names = ["a", "b", "c", "d", "e"];
  const rows = listLibraryPackages(names.map(pkg), new Set(), {});
  expect(movePackage(rows, "a", "d", "after")).toEqual(["b", "c", "d", "a", "e"]);
  expect(movePackage(rows, "a", "d", "before")).toEqual(["b", "c", "a", "d", "e"]);
  expect(movePackage(rows, "e", "b", "before")).toEqual(["a", "e", "b", "c", "d"]);
  expect(movePackage(rows, "e", "a", "before")).toEqual(["e", "a", "b", "c", "d"]);
  expect(movePackage(rows, "a", "e", "after")).toEqual(["b", "c", "d", "e", "a"]);
});

test("the library groups core, installed and built-in packages, reordered in Edit Order", () => {
  const html = render(
    ["@keychord/chords-chord", "@keychord/raycast", "@keychord/warp", "@keychord/finder"].map(pkg),
    [],
    ["@keychord/finder"],
  );
  const core = html.indexOf(">Core<");
  expect(core).toBeGreaterThan(-1);
  expect(core).toBeLessThan(html.indexOf('aria-label="Open @keychord/chords-chord"'));
  const installed = html.indexOf(">Installed<");
  const builtin = html.indexOf(">Built-in<");
  expect(installed).toBeGreaterThan(html.indexOf('aria-label="Open @keychord/chords-chord"'));
  expect(builtin).toBeGreaterThan(html.indexOf('aria-label="Open @keychord/warp"'));
  expect(builtin).toBeLessThan(html.indexOf('aria-label="Open @keychord/finder"'));
  expect(html).toContain("Installed packages always override them.");
  // Packages read by name, with the full package name beneath.
  expect(html).toContain(">Raycast</span>");
  // Handles appear only after Edit Order, so rows stay plain until the user reorders.
  expect(html).toMatch(/aria-pressed="false"[^>]*>Edit Order</);
  expect(html).not.toContain('aria-label="Reorder');
  expect(html).toContain("press the Up or Down arrow key");
});

test("package titles use the app or the name after chords-", () => {
  const entry = (name: string) => listLibraryPackages([pkg(name)], new Set(), {})[0]!;
  expect(packageTitle(entry("@keychord/chords-spotify"))).toBe("Spotify");
  expect(packageTitle(entry("@keychord/chords-com.apple.ActivityMonitor"))).toBe(
    "Activity Monitor",
  );
  expect(packageTitle(entry("@me/work"))).toBe("Work");
  expect(
    packageTitle(entry("@keychord/chords-com.apple.Safari"), {
      bundleId: "com.apple.Safari",
      displayName: "Safari",
    }),
  ).toBe("Safari");
});

test("chord sequences show placeholders as one key", () => {
  const html = renderToString(<ChordKeys sequence="<>ab" />);
  // The whole sequence is one keycap; `<>` stays one token rather than two characters.
  expect(html.match(/<kbd/g)).toHaveLength(1);
  expect(html.match(/<span/g)).toHaveLength(3);
  expect(html).toContain('data-placeholder="true"');
  expect(html).toContain("&lt;&gt;</span>");
});

test("chord keys show the keys a placeholder stands for", () => {
  const html = renderToString(
    <ChordKeys sequence="<>ab" resolve={(name) => (name === "<>" ? ";s" : undefined)} />,
  );
  expect(html).toContain('aria-label=";sab"');
  expect(html).toContain('data-prefix="true"');
  expect(html).not.toContain("&lt;&gt;");
});

test("placeholders list each token with its keys and default", () => {
  const rift = pkg("@keychord/chords-rift");
  const placeholder = {
    name: "<>",
    label: "Window management prefix",
    default: ";",
    assigned: "-",
  };
  const named = { name: "<mode>" };
  rift.compiledChordsFiles["chords/macos.toml"]!.placeholders = [placeholder, named];
  const entries = listPlaceholders([pkg("@keychord/finder"), rift]);
  expect(entries).toEqual([
    { packageName: "@keychord/chords-rift", pathslug: "chords/macos.toml", placeholder },
    { packageName: "@keychord/chords-rift", pathslug: "chords/macos.toml", placeholder: named },
  ]);

  const section = (rows: typeof entries) =>
    renderToString(
      <QueryClientProvider client={new QueryClient()}>
        <PackagePlaceholders entries={rows} apps={{}} />
      </QueryClientProvider>,
    ).replaceAll("<!-- -->", "");
  const html = section(entries);
  expect(html).toContain("Window management prefix");
  expect(html).toContain("&lt;&gt;");
  expect(html).toContain('text-foreground">&lt;mode&gt;</span>');
  expect(html).toMatch(/Default <code[^>]*>;<\/code>/);
  expect(html).toMatch(/<input[^>]*value="-"/);
  expect(html).toContain("Use default");
  // Nothing to configure, so no section.
  expect(section([])).toBe("");
});

test("placeholders are configured inside their package, not in a separate section", () => {
  const rift = pkg("@keychord/chords-rift");
  rift.compiledChordsFiles["chords/macos.toml"]!.placeholders = [
    { name: "<>", label: "Window management prefix", default: ";" },
  ];
  const unset = pkg("@keychord/chords-menu");
  unset.compiledChordsFiles["chords/macos.toml"]!.placeholders = [{ name: "<mode>" }];
  const rows = listLibraryPackages([rift, unset, pkg("@keychord/finder")], new Set(), {});
  expect(rows.map((row) => row.placeholders.length)).toEqual([1, 1, 0]);

  // The Chords page has no standalone placeholder section any more.
  const page = render([rift]);
  expect(page).not.toContain("Window management prefix");
  expect(page).toContain("1 placeholder");

  // Collapsed packages flag a placeholder whose chords stay off until it gets keys.
  const html = renderToString(
    <QueryClientProvider client={new QueryClient()}>
      <ChordLibrary packages={rows} apps={{}} />
    </QueryClientProvider>,
  ).replaceAll("<!-- -->", "");
  expect(html.match(/needs keys/g)).toHaveLength(1);
  // Placeholder fields render only once their package is expanded.
  expect(html).not.toContain("Window management prefix");
  // Without onReorder, there is nothing to reorder.
  expect(html).not.toContain("Edit Order");
});
