import { toast } from "@chord/com.npmjs.sonner";
import { useMutation } from "@chord/com.npmjs.tanstack__react-query";
import { taurpc } from "@chord/dev.improve.chord.api.taurpc";
import { Badge } from "@chord/dev.improve.chord.components.ui.badge";
import { Button } from "@chord/dev.improve.chord.components.ui.button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@chord/dev.improve.chord.components.ui.card";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@chord/dev.improve.chord.components.ui.dropdown-menu";
import { Input } from "@chord/dev.improve.chord.components.ui.input";
import { StateQueries, useGitRepoStoreState } from "@chord/dev.improve.chord.lib.state";
import { AddRepoButton } from "@chord/dev.improve.chord.routes.settings._components.add-repo-button";
import { OpenRepoButton } from "@chord/dev.improve.chord.routes.settings._components.open-repo-button";
import { SyncRepoButton } from "@chord/dev.improve.chord.routes.settings._components.sync-repo-button";
import { Ellipsis, Link, Trash2 } from "lucide-react";
import { useState } from "react";
import type { ReactNode } from "react";

export function ChordReposCard({ monorepos = false, children }: { monorepos?: boolean; children?: ReactNode }) {
	return (
		<StateQueries queries={[useGitRepoStoreState()]}>
			{(gitRepoStoreData) => (
				<ChordReposCardContent monorepos={monorepos} gitRepoStoreData={gitRepoStoreData}>{children}</ChordReposCardContent>
			)}
		</StateQueries>
	);
}

function ChordReposCardContent({
	monorepos = false,
	children,
	gitRepoStoreData,
}: {
	monorepos?: boolean
	children?: ReactNode
	gitRepoStoreData: NonNullable<ReturnType<typeof useGitRepoStoreState>['data']>
}) {
  const { repos } = gitRepoStoreData;
  const visibleRepos = Object.values(repos).filter(
    (repo) => Boolean(repo.isMonorepo) === monorepos,
  );
  return (
    <Card size="sm">
      <CardHeader>
        <div className="flex items-center justify-between gap-3">
          <div>
            <CardTitle>{monorepos ? "Chord Monorepos" : "Chord Repos"}</CardTitle>
            <CardDescription>
              {monorepos
                ? "Load packages from packages/chords-* in a GitHub repo or local folder. Local packages override matching GitHub packages."
                : "Added GitHub repos are cloned into the app cache. Locally linked packages take precedence."}
            </CardDescription>
          </div>
        </div>
      </CardHeader>
      <CardContent className="space-y-4 pt-0">
        <AddRepoButton monorepo={monorepos} />
        <div className="space-y-3">
          {visibleRepos.length === 0 ? (
            <p className="text-sm text-muted-foreground">
              {monorepos ? "No GitHub monorepos added yet." : "No external repos added yet."}
            </p>
          ) : (
            visibleRepos.map((repo) => <GitRepoRow key={repo.slug} repo={repo} />)
          )}
        </div>
        {children}
      </CardContent>
    </Card>
  );
}

function GitRepoRow({
  repo,
}: {
  repo: {
    slug: string;
    headShortSha?: string;
    pinnedRev?: string | null;
    url: string;
    linkedLocalPath?: string | null;
    isMonorepo?: boolean;
  };
}) {
  const [isLinking, setIsLinking] = useState(false);
  const [folderPath, setFolderPath] = useState("");
  const linkMutation = useMutation({
    mutationFn: (path: string | null) => taurpc.setGitRepoLocalLink(repo.slug, path),
    onSuccess: () => {
      setIsLinking(false);
      setFolderPath("");
    },
  });
  const reloadMutation = useMutation({
    mutationFn: taurpc.reloadChords,
    onSuccess: () => toast.success("Chords reloaded."),
  });
  const isPending = linkMutation.isPending || reloadMutation.isPending;
  const error = linkMutation.error ?? reloadMutation.error;
  return (
    <div key={repo.slug} className="rounded-lg border bg-background/80 px-3 py-3">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div className="min-w-0 space-y-1">
          <div className="flex items-center gap-2">
            <p className="truncate font-medium">{repo.slug}</p>
            <Badge variant="secondary">GitHub</Badge>
            {repo.linkedLocalPath && <Badge variant="secondary">Linked locally</Badge>}
            {repo.pinnedRev && !repo.linkedLocalPath ? (
              <Badge variant="outline">Pinned</Badge>
            ) : null}
            {repo.headShortSha && !repo.linkedLocalPath ? (
              <Badge variant="outline" className="font-mono text-[11px]">
                {repo.headShortSha}
              </Badge>
            ) : null}
          </div>
          {repo.linkedLocalPath && (
            <p className="break-all text-xs text-muted-foreground">{repo.linkedLocalPath}</p>
          )}
        </div>
        <div className="flex flex-wrap items-center gap-2 self-end sm:self-center">
          <OpenRepoButton repo={repo} />
          {repo.linkedLocalPath ? (
            <>
              <Button
                variant="outline"
                disabled={isPending}
                onClick={() => {
                  linkMutation.reset();
                  reloadMutation.mutate();
                }}
              >
                {reloadMutation.isPending ? "Reloading..." : "Reload"}
              </Button>
              <Button
                variant="outline"
                disabled={isPending}
                onClick={() => {
                  reloadMutation.reset();
                  linkMutation.mutate(null);
                }}
              >
                Unlink
              </Button>
            </>
          ) : (
            <>
              {repo.pinnedRev ? null : <SyncRepoButton repo={repo} />}
              <Button
                type="button"
                variant="ghost"
                size="icon-sm"
                aria-label={`Link ${repo.slug} to a local folder`}
                title="Link Local Folder"
                disabled={isPending}
                onClick={() => setIsLinking(true)}
              >
                <Link aria-hidden="true" />
              </Button>
            </>
          )}
          <RepoActionsMenuButton repo={repo} />
        </div>
      </div>
      {isLinking && !repo.linkedLocalPath && (
        <form
          className="mt-3 space-y-3"
          onSubmit={(event) => {
            event.preventDefault();
            if (folderPath.trim() && !isPending) linkMutation.mutate(folderPath.trim());
          }}
        >
          <p className="text-xs text-muted-foreground">
            {repo.isMonorepo
              ? "Use a local monorepo with packages/chords-* folders. Click Reload after editing files or adding packages. Unlink to restore the GitHub version."
              : "Use a local copy of this package with the same package.json name. Edit its files, then click Reload to apply changes. Unlink to return to the installed version."}
          </p>
          <div className="flex flex-col gap-2 sm:flex-row">
            <Input
              aria-label={`Local folder for ${repo.slug}`}
              placeholder={repo.isMonorepo ? "/path/to/monorepo" : "/path/to/package"}
              value={folderPath}
              onChange={(event) => setFolderPath(event.target.value)}
              disabled={isPending}
              autoComplete="off"
              spellCheck={false}
            />
            <Button type="submit" disabled={isPending || !folderPath.trim()}>
              {linkMutation.isPending ? "Linking..." : "Link Folder"}
            </Button>
            <Button
              type="button"
              variant="ghost"
              disabled={isPending}
              onClick={() => {
                setIsLinking(false);
                linkMutation.reset();
              }}
            >
              Cancel
            </Button>
          </div>
        </form>
      )}
      {error && (
        <p role="alert" className="mt-3 text-sm text-destructive">
          {typeof error === "object" && "Message" in error ? String(error.Message) : String(error)}
        </p>
      )}
    </div>
  );
}

function RepoActionsMenuButton({ repo }: { repo: { slug: string } }) {
  const removeGitRepoMutation = useMutation({
    mutationFn: taurpc.removeGitRepo,
    onSuccess: () => {
      toast.success(`Removed ${repo.slug}.`);
    },
  });

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button
          type="button"
          variant="ghost"
          size="icon-sm"
          aria-label={`More actions for ${repo.slug}`}
          title="More actions"
          disabled={removeGitRepoMutation.isPending}
        >
          <Ellipsis />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="w-40">
        <DropdownMenuItem
          variant="destructive"
          disabled={removeGitRepoMutation.isPending}
          onSelect={() => {
            removeGitRepoMutation.mutate(repo.slug);
          }}
        >
          <Trash2 />
          {removeGitRepoMutation.isPending ? "Removing..." : "Remove Repo"}
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
