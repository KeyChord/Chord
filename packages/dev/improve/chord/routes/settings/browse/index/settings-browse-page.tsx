import { toast } from "@chord/dev.improve.chord.components.ui.toast";
import { useMutation } from "@chord/com.npmjs.tanstack__react-query";
import {
  type ColumnDef,
  columnFilteringFeature,
  createFilteredRowModel,
  globalFilteringFeature,
  tableFeatures,
  useTable,
} from "@chord/com.npmjs.tanstack__react-table";
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
import { useMemo } from "react";
import { CategoryFilter } from "#category-filter";
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
const categoryOptions = [
  { value: "all", label: "All" },
  ...categories.map((category) => ({ value: category, label: category })),
];
const browseFeatures = tableFeatures({
  columnFilteringFeature,
  globalFilteringFeature,
  filteredRowModel: createFilteredRowModel(),
});
type BrowseRepo = OfficialChordRepo & {
  slug: string;
  installedRepo?: GitReposState["repos"][string];
};
const displayNames = new Map(
  officialChordRepos.flatMap((repo) => {
    const slug = getGitHubSlug(repo.url);
    return slug ? [[slug.toLowerCase(), repo.displayName] as const] : [];
  }),
);
// Official chord package mirrors and community chord package repos carry the `chordpack` topic.
const communityChordPackagesUrl = "https://github.com/search?q=topic%3Achordpack&type=repositories";

type BrowsePageProps = { onConfigure: (packageName: string) => void };

export function SettingsBrowsePage({ onConfigure }: BrowsePageProps) {
  return (
    <StateQueries queries={[useGitRepoStoreState()]}>
      {(gitRepoStoreData) => (
        <SettingsBrowsePageContent state={gitRepoStoreData} onConfigure={onConfigure} />
      )}
    </StateQueries>
  );
}

export function SettingsBrowsePageContent({
  state,
  onConfigure,
}: BrowsePageProps & { state: GitReposState }) {
  const { repos } = state;
  const data = useMemo(() => {
    const installedRepos = new Map(Object.values(repos).map((repo) => [repo.slug.toLowerCase(), repo]));
    return officialChordRepos.map((repo): BrowseRepo => {
      const slug = getGitHubSlug(repo.url) ?? repo.url;
      return { ...repo, slug, installedRepo: installedRepos.get(slug.toLowerCase()) };
    });
  }, [repos]);
  const { mutate: install, isPending: installing, variables: installingUrl } = useMutation({
    mutationFn: taurpc.addGitRepo,
  });
  const { mutate: uninstall, isPending: uninstalling, variables: uninstallingSlug } = useMutation({
    mutationFn: (slug: string) => taurpc.removeGitRepo(slug),
  });
  const isPending = installing || uninstalling;
  const columns = useMemo<ColumnDef<typeof browseFeatures, BrowseRepo>[]>(
    () => [
      {
        id: "repository",
        header: "Repository",
        accessorFn: (repo) =>
          `${repo.displayName} ${repo.slug} ${repo.description} ${repo.categories.join(" ")}`.toLowerCase(),
        cell: ({ row }) => <RepositoryDetails repo={row.original} />,
      },
      {
        id: "category",
        header: "Category",
        accessorFn: (repo) => repo.categories,
        filterFn: (row, _id, value: string) => row.original.categories.includes(value),
        cell: ({ row }) => (
          <div className="flex flex-wrap gap-1">
            {row.original.categories.map((name) => (
              <Badge key={name} variant="outline" className="whitespace-nowrap">
                {name}
              </Badge>
            ))}
          </div>
        ),
      },
      {
        id: "actions",
        header: "Actions",
        accessorFn: (repo) => Boolean(repo.installedRepo),
        filterFn: (row, _id, value: InstallFilter) =>
          Boolean(row.original.installedRepo) === (value === "installed"),
        cell: ({ row }) => {
          const repo = row.original;
          const { installedRepo } = repo;
          const isInstalling = installing && installingUrl === repo.url;
          const isUninstalling = uninstalling && uninstallingSlug === installedRepo?.slug;

          return (
            <div className="flex flex-wrap items-center justify-end gap-1 whitespace-nowrap @min-[34rem]:flex-nowrap">
              {installedRepo ? (
                <>
                  <Button
                    type="button"
                    variant="link"
                    size="sm"
                    className="text-destructive underline"
                    onClick={() =>
                      uninstall(installedRepo.slug, {
                        onSuccess: () => toast.success(`Uninstalled ${repo.displayName}.`),
                      })
                    }
                    disabled={isPending}
                  >
                    {isUninstalling ? "Uninstalling..." : "Uninstall"}
                  </Button>
                  <Button
                    type="button"
                    size="sm"
                    onClick={() => onConfigure(repo.packageName)}
                    disabled={isUninstalling}
                  >
                    Configure
                  </Button>
                </>
              ) : (
                <Button
                  type="button"
                  size="sm"
                  onClick={() =>
                    install(repo.url, {
                      onSuccess: () => toast.success(`Installed ${repo.displayName}.`),
                    })
                  }
                  disabled={isPending}
                >
                  <Download />
                  {isInstalling ? "Installing..." : "Install"}
                </Button>
              )}
              <OpenRepoButton repo={repo} />
            </div>
          );
        },
      },
    ],
    [install, installing, installingUrl, isPending, onConfigure, uninstall, uninstalling, uninstallingSlug],
  );
  const table = useTable({
    features: browseFeatures,
    data,
    columns,
    getRowId: (repo) => repo.url,
    getColumnCanGlobalFilter: (column) => column.id === "repository",
    globalFilterFn: (row, columnId, value: string) =>
      row.getValue<string>(columnId).includes(value.trim().toLowerCase()),
  });
  const rows = table.getRowModel().rows;
  const categoryColumn = table.getColumn("category");
  const actionsColumn = table.getColumn("actions");

  return (
    <div className="space-y-4">
      <div className="space-y-1">
        <h2 className="text-base font-semibold">Browse</h2>
        <p className="text-sm text-muted-foreground">Official chord repos for popular apps.</p>
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
              value={String(table.state.globalFilter ?? "")}
              onChange={(event) => table.setGlobalFilter(event.target.value)}
              placeholder="Search chord repos..."
              className="pl-9"
            />
          </div>

          <ToggleGroup
            type="single"
            variant="outline"
            spacing={1}
            aria-label="Install status"
            value={String(actionsColumn?.getFilterValue() ?? "all")}
            onValueChange={(value) => {
              if (value) {
                actionsColumn?.setFilterValue(value === "all" ? undefined : value);
              }
            }}
          >
            <ToggleGroupItem value="all">All</ToggleGroupItem>
            <ToggleGroupItem value="installed">Installed</ToggleGroupItem>
            <ToggleGroupItem value="not-installed">Not Installed</ToggleGroupItem>
          </ToggleGroup>
        </div>

        <CategoryFilter
          options={categoryOptions}
          value={String(categoryColumn?.getFilterValue() ?? "all")}
          onChange={(value) => categoryColumn?.setFilterValue(value === "all" ? undefined : value)}
        />
      </div>

      {rows.length === 0 ? (
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
        <div className="@container overflow-x-auto rounded-lg border">
          <table className="w-full border-collapse text-left text-sm">
            <caption className="sr-only">Official chord repositories</caption>
            <thead className="border-b bg-default/40 text-xs text-muted-foreground">
              {table.getHeaderGroups().map((group) => (
                <tr key={group.id}>
                  {group.headers.map((header) => (
                    <th
                      key={header.id}
                      scope="col"
                      className="px-3 py-2 font-medium last:text-right"
                    >
                      <table.FlexRender header={header} />
                    </th>
                  ))}
                </tr>
              ))}
            </thead>
            <tbody>
              {rows.map((row) => (
                <tr key={row.id} className="border-b last:border-b-0 hover:bg-default/30">
                  {row.getAllCells().map((cell) => (
                    <td key={cell.id} className="px-3 py-2.5 first:w-full first:min-w-44">
                      <table.FlexRender cell={cell} />
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      <CommunityChordPackages />
    </div>
  );
}

function RepositoryDetails({ repo }: { repo: BrowseRepo }) {
  const { installedRepo } = repo;
  return (
    <div className="space-y-1">
      <h3 className="font-medium">{repo.displayName}</h3>
      <p className="text-muted-foreground">{repo.description}</p>
      <div className="flex flex-wrap items-center gap-x-2 gap-y-1 text-xs text-muted-foreground">
        <span className="flex items-center gap-1.5">
          <FolderGit2 className="size-3.5 shrink-0" />
          <span className="break-all">{repo.slug}</span>
        </span>
        {installedRepo?.headShortSha && !installedRepo.linkedLocalPath && (
          <span className="font-mono">{installedRepo.headShortSha}</span>
        )}
        {installedRepo?.linkedLocalPath ? (
          <span>Linked locally</span>
        ) : (
          installedRepo?.pinnedRev && <span>Pinned</span>
        )}
      </div>
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
