#!/usr/bin/env bun
import { getGitHubSlug } from "@chord/dev.improve.chord.lib.github";

export interface OfficialChordRepo {
  packageName: string;
  displayName: string;
  description: string;
  categories: string[];
  url: string;
}

// The chords monorepo is checked out next to Chord, as apps/chord-tauri/src-tauri/build.rs expects.
export const chordsMonorepoUrl = new URL("../../../../../../../chords/", import.meta.url);
const defaultPackagesUrl = new URL(
  "../../../../../../apps/chord-tauri/data/default-packages.toml",
  import.meta.url,
);
const generatedFileUrl = new URL("./@assets/official-chord-repos.gen.json", import.meta.url);

/**
 * Every public package in the monorepo's @keychord directory, except the ones Chord installs
 * itself: those are already among the installed chords, so there's nothing to offer.
 */
export async function readOfficialChordRepos(
  chordsDir = chordsMonorepoUrl,
  defaultPackagesFile = defaultPackagesUrl,
): Promise<OfficialChordRepo[]> {
  const defaults = Bun.TOML.parse(await Bun.file(defaultPackagesFile).text()) as {
    packages: Record<string, unknown>;
  };
  const builtInPackages = new Set<unknown>(Object.keys(defaults.packages));
  const manifests = await Array.fromAsync(
    new Bun.Glob("@keychord/*/package.json").scan(Bun.fileURLToPath(chordsDir)),
  );

  const repos = await Promise.all(
    manifests.map(async (manifest) => {
      const pkg = (await Bun.file(new URL(manifest, chordsDir)).json()) as {
        name?: unknown;
        private?: unknown;
        displayName?: unknown;
        description?: unknown;
        categories?: unknown;
        repository?: string | { url?: unknown };
      };
      if (pkg.private === true || builtInPackages.has(pkg.name)) {
        return [];
      }

      const url = typeof pkg.repository === "string" ? pkg.repository : pkg.repository?.url;
      if (
        typeof pkg.name !== "string" ||
        typeof pkg.displayName !== "string" ||
        typeof pkg.description !== "string" ||
        !Array.isArray(pkg.categories) ||
        !pkg.categories.every((category) => typeof category === "string") ||
        typeof url !== "string" ||
        !getGitHubSlug(url)
      ) {
        throw new Error(
          `${manifest} needs a name, displayName, description, categories and a GitHub repository.url`,
        );
      }

      return [
        {
          packageName: pkg.name,
          displayName: pkg.displayName,
          description: pkg.description,
          categories: pkg.categories,
          url,
        },
      ];
    }),
  );

  return repos.flat().sort((a, b) => a.displayName.localeCompare(b.displayName, "en"));
}

if (import.meta.main) {
  if (process.argv.slice(2).some((arg) => arg === "--help" || arg === "-h")) {
    await Bun.write(
      Bun.stdout,
      "Usage: vpx _gen-official-chord-repos\n\nRegenerate the official package catalog from the sibling chords checkout.\n",
    );
  } else {
    await Bun.write(
      generatedFileUrl,
      `${JSON.stringify(await readOfficialChordRepos(), null, "\t")}\n`,
    );
  }
}
