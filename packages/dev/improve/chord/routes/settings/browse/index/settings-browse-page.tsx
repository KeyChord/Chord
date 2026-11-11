import { toast } from "@chord/dev.improve.chord.components.ui.toast";
import { useMutation } from "@chord/com.npmjs.tanstack__react-query";
import { taurpc } from "@chord/dev.improve.chord.api.taurpc";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@chord/dev.improve.chord.components.ui.alert-dialog";
import { Badge } from "@chord/dev.improve.chord.components.ui.badge";
import { Button } from "@chord/dev.improve.chord.components.ui.button";
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@chord/dev.improve.chord.components.ui.empty";
import { Input } from "@chord/dev.improve.chord.components.ui.input";
import { ToggleGroup, ToggleGroupItem } from "@chord/dev.improve.chord.components.ui.toggle-group";
import { officialChordReposData } from "@chord/dev.improve.chord.data.official-chord-repos";
import { getGitHubSlug } from "@chord/dev.improve.chord.lib.github";
import { StateQueries, useGitRepoStoreState } from "@chord/dev.improve.chord.lib.state";
import { OpenRepoButton } from "@chord/dev.improve.chord.routes.settings._components.open-repo-button";
import type { GitReposState } from "@chord/dev.improve.chord.lib.typeshare";
import { openUrl } from "@tauri-apps/plugin-opener";
import { Download, ExternalLink, FolderGit2, Search } from "lucide-react";
import { useState } from "react";
import { PackageUpdates } from "#package-updates";

interface OfficialChordRepo {
  packageName: string;
  displayName: string;
  description: string;
  categories: string[];
  url: string;
}

type InstallFilter = "all" | "installed" | "not-installed";

const officialChordRepos: OfficialChordRepo[] = officialChordReposData;
const categories = [...new Set(officialChordRepos.flatMap((repo) => repo.categories))].sort(
  (a, b) => a.localeCompare(b, "en"),
);
const displayNames = new Map(
  officialChordRepos.flatMap((repo) => {
    const slug = getGitHubSlug(repo.url);
    return slug ? [[slug.toLowerCase(), repo.displayName] as const] : [];
  }),
);
// Official chord package mirrors and community chord package repos carry the `chordpack` topic.
const communityChordPackagesUrl = "https://github.com/search?q=topic%3Achordpack&type=repositories";

export function SettingsBrowsePage() {
  return (
    <StateQueries queries={[useGitRepoStoreState()]}>
      {(gitRepoStoreData) => <SettingsBrowsePageContent state={gitRepoStoreData} />}
    </StateQueries>
  );
}

export function SettingsBrowsePageContent({ state }: { state: GitReposState }) {
  const { repos } = state;
  const [searchInput, setSearchInput] = useState("");
  const [installFilter, setInstallFilter] = useState<InstallFilter>("all");
  const [category, setCategory] = useState("all");

  const installedRepos = new Map(
    Object.values(repos).map((repo) => [repo.slug.toLowerCase(), repo]),
  );
  const addGitRepoMutation = useMutation({
    mutationFn: taurpc.addGitRepo,
  });
  const removeGitRepoMutation = useMutation({
    mutationFn: (slug: string) => taurpc.removeGitRepo(slug),
  });
  const isPending = addGitRepoMutation.isPending || removeGitRepoMutation.isPending;

  function getInstalledRepo(repo: OfficialChordRepo) {
    const slug = getGitHubSlug(repo.url);
    return slug ? installedRepos.get(slug.toLowerCase()) : undefined;
  }

  const missingRepos = officialChordRepos.filter((repo) => !getInstalledRepo(repo));
  const normalizedSearch = searchInput.trim().toLowerCase();
  const filteredRepos = officialChordRepos.filter((repo) => {
    const isInstalled = Boolean(getInstalledRepo(repo));
    if (installFilter === "installed" && !isInstalled) {
      return false;
    }
    if (installFilter === "not-installed" && isInstalled) {
      return false;
    }
    if (category !== "all" && !repo.categories.includes(category)) {
      return false;
    }

    const slug = getGitHubSlug(repo.url) ?? repo.url;
    return `${repo.displayName} ${slug} ${repo.description} ${repo.categories.join(" ")}`
      .toLowerCase()
      .includes(normalizedSearch);
  });

  async function installAll() {
    let installedCount = 0;
    for (const repo of missingRepos) {
      try {
        await addGitRepoMutation.mutateAsync(repo.url);
        installedCount += 1;
      } catch {
        // The mutation cache reports the error.
        break;
      }
    }

    if (installedCount > 0) {
      toast.success(`Installed ${installedCount} repo${installedCount === 1 ? "" : "s"}.`);
    }
  }

  return (
    <div className="space-y-4">
      <div className="flex items-start justify-between gap-3">
        <div className="space-y-1">
          <h2 className="text-base font-semibold">Browse</h2>
          <p className="text-sm text-muted-foreground">Official chord repos for popular apps.</p>
        </div>
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={() => {
            void installAll();
          }}
          disabled={isPending || missingRepos.length === 0}
        >
          <Download />
          Install All
        </Button>
      </div>

      <PackageUpdates
        repos={repos}
        updates={state.updates}
        updateCheck={state.updateCheck}
        displayNames={displayNames}
      />

      <div className="space-y-3 border-b pb-4">
        <div className="flex flex-col gap-2 sm:flex-row sm:items-center">
          <div className="relative flex-1">
            <Search className="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground" />
            <Input
              aria-label="Search chord repos"
              value={searchInput}
              onChange={(event) => {
                setSearchInput(event.target.value);
              }}
              placeholder="Search chord repos..."
              className="pl-9"
            />
          </div>

          <ToggleGroup
            type="single"
            variant="outline"
            spacing={1}
            aria-label="Install status"
            value={installFilter}
            onValueChange={(value) => {
              if (value) {
                setInstallFilter(value as InstallFilter);
              }
            }}
          >
            <ToggleGroupItem value="all">All</ToggleGroupItem>
            <ToggleGroupItem value="installed">Installed</ToggleGroupItem>
            <ToggleGroupItem value="not-installed">Not Installed</ToggleGroupItem>
          </ToggleGroup>
        </div>

        <ToggleGroup
          type="single"
          size="sm"
          spacing={1}
          aria-label="Category"
          className="flex-wrap"
          value={category}
          onValueChange={(value) => {
            if (value) {
              setCategory(value);
            }
          }}
        >
          <ToggleGroupItem value="all">All</ToggleGroupItem>
          {categories.map((name) => (
            <ToggleGroupItem key={name} value={name}>
              {name}
            </ToggleGroupItem>
          ))}
        </ToggleGroup>
      </div>

      {filteredRepos.length === 0 ? (
        <Empty className="rounded-lg border bg-default/20 py-10">
          <EmptyHeader>
            <EmptyMedia variant="icon">
              <Search />
            </EmptyMedia>
            <EmptyTitle>No matching repos</EmptyTitle>
            <EmptyDescription>Try a different search or filter.</EmptyDescription>
          </EmptyHeader>
        </Empty>
      ) : (
        <div className="space-y-3">
          {filteredRepos.map((repo) => {
            const slug = getGitHubSlug(repo.url) ?? repo.url;
            const installedRepo = getInstalledRepo(repo);
            const isInstalling =
              addGitRepoMutation.isPending && addGitRepoMutation.variables === repo.url;
            const isRemoving =
              removeGitRepoMutation.isPending &&
              removeGitRepoMutation.variables === installedRepo?.slug;

            return (
              <article key={repo.url} className="rounded-lg border bg-default/30 px-4 py-3">
                <div className="flex items-start justify-between gap-3">
                  <div className="flex min-w-0 flex-wrap items-center gap-x-2 gap-y-1">
                    <h3 className="font-medium">{repo.displayName}</h3>
                    {installedRepo?.headShortSha && !installedRepo.linkedLocalPath ? (
                      <span className="font-mono text-xs text-muted-foreground">
                        {installedRepo.headShortSha}
                      </span>
                    ) : null}
                    {repo.categories.map((name) => (
                      <Badge key={name} variant="outline">
                        {name}
                      </Badge>
                    ))}
                  </div>

                  {!installedRepo ? (
                    <Button
                      type="button"
                      size="sm"
                      onClick={() => {
                        addGitRepoMutation.mutate(repo.url, {
                          onSuccess: () => {
                            toast.success(`Installed ${repo.displayName}.`);
                          },
                        });
                      }}
                      disabled={isPending}
                    >
                      <Download />
                      {isInstalling ? "Installing..." : "Install"}
                    </Button>
                  ) : (
                    <Button
                      type="button"
                      variant="outline"
                      size="sm"
                      onClick={() => {
                        removeGitRepoMutation.mutate(installedRepo.slug, {
                          onSuccess: () => {
                            toast.success(`Uninstalled ${repo.displayName}.`);
                          },
                        });
                      }}
                      disabled={isPending}
                    >
                      {isRemoving ? "Uninstalling..." : "Uninstall"}
                    </Button>
                  )}
                </div>

                <div className="mt-1 flex items-start justify-between gap-3">
                  <p className="text-sm text-muted-foreground">{repo.description}</p>
                  {installedRepo?.linkedLocalPath ? (
                    <span className="shrink-0 text-xs text-muted-foreground">Linked locally</span>
                  ) : installedRepo?.pinnedRev ? (
                    <span className="shrink-0 text-xs text-muted-foreground">Pinned</span>
                  ) : null}
                </div>

                <div className="mt-2 flex items-center justify-between gap-3">
                  <p className="flex min-w-0 items-center gap-1.5 text-xs text-muted-foreground">
                    <FolderGit2 className="size-3.5 shrink-0" />
                    <span className="truncate">{slug}</span>
                  </p>
                  <OpenRepoButton repo={{ url: repo.url, slug }} />
                </div>
              </article>
            );
          })}
        </div>
      )}

      <CommunityChordPackages />
    </div>
  );
}

function CommunityChordPackages() {
  const openCommunityPackages = useMutation({
    mutationFn: () => openUrl(communityChordPackagesUrl),
  });

  return (
    <div className="flex items-center justify-between gap-3 rounded-lg border border-dashed px-4 py-3">
      <div className="space-y-1">
        <h3 className="font-medium">Community chord packages</h3>
        <p className="text-sm text-muted-foreground">
          Find more on GitHub under the chordpack topic. Download and extract a package into{" "}
          <code>~/.chord/packages/&lt;name&gt;</code>, then quit and restart Chord and approve it in
          the Chords tab.
        </p>
      </div>

      <AlertDialog>
        <Button type="button" variant="outline" size="sm">
          <ExternalLink />
          Search GitHub
        </Button>

        <AlertDialogContent size="sm">
          <AlertDialogHeader>
            <AlertDialogTitle>Only install chords you trust</AlertDialogTitle>
            <AlertDialogDescription>
              Community chord packages aren't reviewed. Their chords run with unrestricted access to
              your computer, so only install packages from authors you trust or after auditing their
              code. Download them manually and approve them in the Chords tab.
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => {
                openCommunityPackages.mutate();
              }}
            >
              Open GitHub
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
