import { afterAll, expect, test } from "bun:test";
import { renderToString } from "react-dom/server";
import { QueryClient, QueryClientProvider } from "@chord/com.npmjs.tanstack__react-query";
import type { PersonalPackages } from "@chord/dev.improve.chord.api.taurpc";
import type { PersonalPackageState } from "@chord/dev.improve.chord.lib.typeshare";

// Queries, mutations, and the editor's mount never run during server rendering; TauRPC needs
// the browser's Proxy constructor.
const previousWindow = globalThis.window;
globalThis.window = { Proxy, screen: {} } as unknown as Window & typeof globalThis;
const { PersonalChordsContent } = await import("#personal-chords");
afterAll(() => {
  globalThis.window = previousWindow;
});

const root = "/Users/me/.chord/packages";

function render(
  listing: PersonalPackages,
  {
    states = listing.packages.map(({ dirname }) => ({ dirname, name: dirname })),
    errors = {},
  }: {
    states?: PersonalPackageState[];
    errors?: Record<string, string[]>;
  } = {},
) {
  const client = new QueryClient();
  return renderToString(
    <QueryClientProvider client={client}>
      <PersonalChordsContent
        listing={listing}
        states={states}
        errors={errors}
        appIds={["com.apple.finder"]}
      />
    </QueryClientProvider>,
  ).replaceAll("<!-- -->", "");
}

function pkg(dirname: string, files: string[]) {
  return { dirname, path: `${root}/${dirname}`, files };
}

test("packages list their files with Edit buttons and leave editors closed initially", () => {
  const html = render({
    root,
    packages: [
      pkg("alpha", ["chords/shared.toml"]),
      pkg("default", ["chords/com/apple/finder/macos.toml", "chords/macos.toml"]),
    ],
  });
  expect(html).toContain('<span class="mac-row-title">alpha</span>');
  expect(html).toContain('<span class="mac-row-title">default</span>');
  expect(html).toContain(">com.apple.finder</span>");
  expect(html).toContain(">Every app</span>");
  // Each file is a row that opens it in the editor.
  expect(html).toContain('aria-label="Edit chords/com/apple/finder/macos.toml"');
  expect(html).toContain('aria-label="Edit chords/macos.toml"');
  expect(html).toContain('aria-label="alpha actions"');
  expect(html).not.toContain('aria-label="Loading editor"');
  expect(html).not.toContain(">Save</button>");
  expect(html).not.toContain("Unsaved");
  // New files are added from a sheet, which stays closed until Add File is pressed.
  expect(html).toContain("Add File");
  expect(html).not.toContain('aria-label="Package for the new chords file"');
});

test("app files show the app's name when Chord knows it", () => {
  const client = new QueryClient();
  const html = renderToString(
    <QueryClientProvider client={client}>
      <PersonalChordsContent
        listing={{ root, packages: [pkg("default", ["chords/com/apple/finder/macos.toml"])] }}
        states={[{ dirname: "default", name: "default" }]}
        errors={{}}
        appIds={[]}
        apps={{ "com.apple.finder": { bundleId: "com.apple.finder", displayName: "Finder" } }}
      />
    </QueryClientProvider>,
  ).replaceAll("<!-- -->", "");
  expect(html).toContain('mac-row-title">Finder</span>');
  expect(html).toContain("chords/com/apple/finder/macos.toml");
});

test("folders that failed to load and files with load errors are reported per package", () => {
  const html = render(
    { root, packages: [pkg("broken", []), pkg("default", ["chords/macos.toml"])] },
    {
      states: [
        { dirname: "broken", error: "package source chords/macos.toml must be valid UTF-8" },
        { dirname: "default", name: "default" },
      ],
      errors: { default: ["Failed to compile chords/macos.toml: file vscode.js not found"] },
    },
  );
  expect(html).toContain("must be valid UTF-8");
  expect(html).toContain("file vscode.js not found");
  expect(html.match(/didn&#x27;t load on the last reload/g)).toHaveLength(2);
});

test("with no packages the page offers to create one", () => {
  const html = render({ root, packages: [] });
  expect(html).toContain("No packages yet");
  expect(html).toContain("New Package");
  expect(html).not.toContain('aria-label="Loading editor"');
});
