import { afterAll, expect, test } from "bun:test";
import { renderToString } from "react-dom/server";
import { QueryClient, QueryClientProvider } from "@chord/com.npmjs.tanstack__react-query";
import type { PersonalPackages } from "@chord/dev.improve.chord.api.taurpc";
import type { PersonalPackageState } from "@chord/dev.improve.chord.lib.typeshare";

// Queries, mutations, and the editor's mount never run during server rendering; TauRPC needs
// the browser's Proxy constructor.
const previousWindow = globalThis.window;
globalThis.window = { Proxy } as unknown as Window & typeof globalThis;
const { PersonalChordsContent } = await import("#settings-personal-page");
afterAll(() => {
  globalThis.window = previousWindow;
});

const root = "/Users/me/.chord/packages";

function render(
  listing: PersonalPackages,
  {
    states = listing.packages.map(({ dirname }) => ({ dirname, name: dirname })),
    errors = {},
    seed,
  }: {
    states?: PersonalPackageState[];
    errors?: Record<string, string[]>;
    seed?: [string, string, string];
  } = {},
) {
  const client = new QueryClient();
  if (seed) client.setQueryData(["personal-chords-file", seed[0], seed[1]], seed[2]);
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

test("packages are listed with their files, and the default package's first file opens", () => {
  const html = render(
    {
      root,
      packages: [
        pkg("alpha", ["chords/shared.toml"]),
        pkg("default", ["chords/com/apple/finder/macos.toml", "chords/macos.toml"]),
      ],
    },
    { seed: ["default", "chords/com/apple/finder/macos.toml", "name = 'Finder'\n[chords]\n"] },
  );
  expect(html).toContain(">alpha</h3>");
  expect(html).toContain(">default</h3>");
  expect(html).toContain(">com.apple.finder</span>");
  expect(html).toContain(">Every app</span>");
  expect(html).toMatch(/<button[^>]*aria-current="true"[^>]*>.*com\/apple\/finder/);
  expect(html).toContain(">default/chords/com/apple/finder/macos.toml</div>");
  // The editor loads and mounts in the browser; the server renders its placeholder.
  expect(html).toContain('aria-label="Loading editor"');
  expect(html).toMatch(/<button[^>]*type="submit"[^>]*disabled=""[^>]*>Save<\/button>/);
  expect(html).not.toContain("Unsaved");
  // With more than one package, new files ask which package they belong to.
  expect(html).toContain('aria-label="Package for the new chords file"');
});

test("a single package needs no package choice for new files", () => {
  const html = render({ root, packages: [pkg("default", ["chords/macos.toml"])] });
  expect(html).not.toContain('aria-label="Package for the new chords file"');
  expect(html).toContain("Add File");
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
