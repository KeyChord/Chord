#!/usr/bin/env bun
import officialChordPackages from "#official-chord-packages.json";
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
const generatedFileUrl = new URL("./@assets/official-chord-repos.gen.json", import.meta.url);

export async function readOfficialChordRepos(
  chordsDir = chordsMonorepoUrl,
): Promise<OfficialChordRepo[]> {
  return Promise.all(
    officialChordPackages.map(async (packageName) => {
      const pkg = (await Bun.file(new URL(`${packageName}/package.json`, chordsDir)).json()) as {
        displayName?: unknown;
        description?: unknown;
        categories?: unknown;
        repository?: string | { url?: unknown };
      };
      const url = typeof pkg.repository === "string" ? pkg.repository : pkg.repository?.url;
      if (
        typeof pkg.displayName !== "string" ||
        typeof pkg.description !== "string" ||
        !Array.isArray(pkg.categories) ||
        !pkg.categories.every((category) => typeof category === "string") ||
        typeof url !== "string" ||
        !getGitHubSlug(url)
      ) {
        throw new Error(
          `${packageName}/package.json needs a displayName, description, categories and a GitHub repository.url`,
        );
      }

      return {
        packageName,
        displayName: pkg.displayName,
        description: pkg.description,
        categories: pkg.categories,
        url,
      };
    }),
  );
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
