import { afterAll, expect, test } from "bun:test";
import { renderToString } from "react-dom/server";
import { QueryClient, QueryClientProvider } from "@chord/com.npmjs.tanstack__react-query";
import type {
  ChordPackage,
  ChordPackageManagerState,
  ThirdPartyPackage,
} from "@chord/dev.improve.chord.lib.typeshare";

const previousWindow = globalThis.window;
globalThis.window = { Proxy, screen: {} } as unknown as Window & typeof globalThis;
const { SettingsChordsPageContent } = await import("#settings-chords-page");
const { applyPackageOrder, ChordLibrary, listLibraryChords, listLibraryPackages, movePackage } =
  await import("#chord-library");
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
      <SettingsChordsPageContent chordPackageManagerData={state} configurePackage={configurePackage} />
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
  expect(html).toMatch(/role="tab"[^>]*>Packages</);
  expect(html).toMatch(/role="tab"[^>]*>Local Chords</);
  expect(html).toContain("Default Global Prefix for Placeholders");
  expect(html).toMatch(/<input[^>]+maxLength="1"[^>]+value=";"/i);
  expect(html).toContain("2 packages");
  expect(html).toContain('aria-label="Show chords in @keychord/finder" aria-expanded="false"');
  expect(html).not.toContain('aria-label="Show chords in @me/work"');
  expect(html).toContain('aria-label="Local Chords"');
  expect(html).toContain('aria-label="Edit chords/macos.toml"');
  expect(html).not.toContain('title="@me/work action"');
  expect(html).not.toContain('title="@keychord/finder action"');
});

test("Local Chords remains editable when no external packages are installed", () => {
  const html = render([pkg("@me/work")]);
  expect(html).not.toContain('aria-label="Show chords in @me/work"');
  expect(html).toContain("Your package library is empty");
  expect(html).toMatch(/role="tab"[^>]*>Local Chords</);
  expect(html).toContain('aria-label="Edit chords/macos.toml"');
  expect(html).toContain("Manage sources");
});

test("opening a package for configuration expands only that package", () => {
  const finder = "@keychord/chords-finder";
  const terminal = "@keychord/chords-terminal";
  const html = render([pkg(finder), pkg(terminal)], [], [], terminal);

  expect(html).toContain(`aria-label="Hide chords in ${terminal}" aria-expanded="true"`);
  expect(html).toContain(`title="${terminal} action"`);
  expect(html).toContain(`aria-label="Show chords in ${finder}" aria-expanded="false"`);
  expect(html).not.toContain(`title="${finder} action"`);
});

test("an empty personal package stays editable in Local Chords", () => {
  const empty = { ...pkg("@me/work"), compiledChordsFiles: {} };
  const html = render([pkg("@keychord/finder"), empty]);
  expect(html).not.toContain('aria-label="Show chords in @me/work"');
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
  expect(html).toMatch(/title="Approval required"[^>]*>(<[^>]*>)*Blocked</);
  expect(html).toContain('aria-label="Approve @untrusted/test"');
  expect(html).toContain("/Users/me/.chord/packages/downloaded");
  expect(html).toContain("arbitrary code");
  expect(html).toContain("after auditing their code");
  expect(html).toContain("quit and restart Chord");
  expect(html).not.toContain('aria-label="Show chords in @untrusted/test"');
  expect(html).not.toContain('aria-label="Revoke approval for @untrusted/test"');
});

test("approved packages expose revocation and an empty approval queue is explicit", () => {
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
  expect(html).toContain("No packages awaiting approval.");
  expect(html).toMatch(/title="Can run code"[^>]*>(<[^>]*>)*Approved</);
  expect(html).toContain('aria-label="Revoke approval for test"');
  expect(html).not.toContain('aria-label="Approve test"');
});

test("third-party packages share one selectable table with blocked packages first", () => {
  const html = render(
    [],
    [
      { name: "zeta", path: "/p/zeta", identity: "zeta-identity", approved: false },
      { name: "alpha", path: "/p/alpha", identity: "alpha-identity", approved: true },
      { name: "beta", path: "/p/beta", identity: "beta-identity", approved: false },
    ],
  );
  expect(html).toContain("2 awaiting approval");
  expect(html).toContain("3 packages");
  expect(html).toContain('aria-label="Search third-party packages"');
  expect(html).toContain('aria-label="Select all packages"');
  const order = ["beta", "zeta", "alpha"].map((name) =>
    html.indexOf(`aria-label="Select ${name}"`),
  );
  expect(order.every((index) => index >= 0)).toBe(true);
  expect(order).toEqual(order.toSorted((a, b) => a - b));
  // Bulk actions only appear once something is selected.
  expect(html).not.toContain('aria-label="Bulk actions"');
});

test("the approval table is omitted when no third-party packages are installed", () => {
  const html = render([]);
  expect(html).toContain("No packages awaiting approval.");
  expect(html).not.toContain('aria-label="Select all packages"');
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
  const names = ["@keychord/chords-chord", "@keychord/raycast", "@me/work", "@keychord/warp", "@keychord/finder", "@keychord/spotlight"];
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
  const moved = [
    "@keychord/warp",
    "@keychord/raycast",
    "@keychord/finder",
    "@keychord/spotlight",
  ];
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

test("the library groups installed and built-in packages with drag handles", () => {
  const html = render(
    ["@keychord/chords-chord", "@keychord/raycast", "@keychord/warp", "@keychord/finder"].map(pkg),
    [],
    ["@keychord/finder"],
  );
  const installed = html.indexOf(">Installed<");
  const builtin = html.indexOf(">Built-in<");
  expect(installed).toBeGreaterThan(html.indexOf('aria-label="Show chords in @keychord/chords-chord"'));
  expect(builtin).toBeGreaterThan(html.indexOf('aria-label="Show chords in @keychord/warp"'));
  expect(builtin).toBeLessThan(html.indexOf('aria-label="Show chords in @keychord/finder"'));
  expect(html).toContain("Installed packages always override them.");
  expect(html).not.toContain('aria-label="Reorder @keychord/chords-chord"');
  for (const name of ["@keychord/raycast", "@keychord/warp", "@keychord/finder"]) {
    expect(html).toMatch(new RegExp(`aria-label="Reorder ${name}"[^>]*title="Drag to reorder"`));
  }
  expect(html).toContain("press the Up or Down arrow key");
  expect(html).not.toMatch(/aria-label="Move [^"]*(up|down)"/);
});

test("placeholders list each token with its keys and default", () => {
  const rift = pkg("@keychord/chords-rift");
  const placeholder = { name: "<>", label: "Window management prefix", default: ";", assigned: "-" };
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
      <ChordLibrary packages={rows} apps={{}} personalDirnames={{}} />
    </QueryClientProvider>,
  ).replaceAll("<!-- -->", "");
  expect(html.match(/needs keys/g)).toHaveLength(1);
  // Placeholder fields render only once their package is expanded.
  expect(html).not.toContain("Window management prefix");
  // Without onReorder, there is nothing to drag.
  expect(html).not.toContain('aria-label="Reorder');
});
