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
const { listLibraryChords, listLibraryPackages } = await import("#chord-library");
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
        chordHints: [],
        chords: [
          {
            name: `${name} action`,
            rawTrigger: "/t",
            trigger: { keys: ["/", "t"] },
            index: 0,
            actions: [],
          },
        ],
      },
    },
  };
}

function render(packages: ChordPackage[], thirdPartyPackages: ThirdPartyPackage[] = []) {
  const state: ChordPackageManagerState = {
    packages,
    errors: {},
    thirdPartyPackages,
    personalPackages: [{ dirname: "work", name: "@me/work" }],
  };
  const client = new QueryClient();
  client.setQueryData(["tauri-observable-states"], { "chord-package-manager": state });
  client.setQueryData(["personal-packages"], {
    root: "/Users/me/.chord/packages",
    packages: [
      { dirname: "work", path: "/Users/me/.chord/packages/work", files: ["chords/macos.toml"] },
    ],
  });
  return renderToString(
    <QueryClientProvider client={client}>
      <SettingsChordsPageContent chordPackageManagerData={state} />
    </QueryClientProvider>,
  ).replaceAll("<!-- -->", "");
}

test("Packages lists installed packages while My Chords keeps personal editing separate", () => {
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
  expect(html).toMatch(/role="tab"[^>]*>My Chords</);
  expect(html).toContain("2 packages");
  expect(html).toContain('aria-label="Show chords in @keychord/finder" aria-expanded="false"');
  expect(html).not.toContain('aria-label="Show chords in @me/work"');
  expect(html).toContain('aria-label="My Chords"');
  expect(html).toContain('aria-label="Edit chords/macos.toml"');
  expect(html).not.toContain('title="@me/work action"');
  expect(html).not.toContain('title="@keychord/finder action"');
});

test("My Chords remains editable when no external packages are installed", () => {
  const html = render([pkg("@me/work")]);
  expect(html).not.toContain('aria-label="Show chords in @me/work"');
  expect(html).toContain("Your package library is empty");
  expect(html).toMatch(/role="tab"[^>]*>My Chords</);
  expect(html).toContain('aria-label="Edit chords/macos.toml"');
  expect(html).toContain("Manage sources");
});

test("an empty personal package stays editable in My Chords", () => {
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
