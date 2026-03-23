import { Check, ExternalLink } from "lucide-react";
import { Button } from "#/components/ui/button.tsx";
import { Badge } from "#/components/ui/badge.tsx";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "#/components/ui/card.tsx";
import { Checkbox } from "#/components/ui/checkbox.tsx";
import { Collapsible, CollapsibleContent } from "#/components/ui/collapsible.tsx";
import { Input } from "#/components/ui/input.tsx";
import { Label } from "#/components/ui/label.tsx";
import { AppIcon } from "#/components/settings/app-icon.tsx";
import { ChordGroupList } from "#/components/settings/chord-views.tsx";
import { buildChordGroups, getAppLabel } from "#/utils/settings.ts";
import type { SettingsPageData } from "#/utils/use-settings-page.ts";

export function SettingsTab({
  settings,
  appMetadataByBundleId,
}: {
  settings: SettingsPageData["settingsTab"];
  appMetadataByBundleId: SettingsPageData["appMetadataByBundleId"];
}) {
  return (
    <div className="space-y-4">
      <Card size="sm">
        <CardHeader>
          <div className="flex items-center justify-between gap-3">
            <div>
              <CardTitle>Chord Repos</CardTitle>
              <CardDescription>
                Added GitHub repos are cloned into the app cache and merged with bundled chords.
              </CardDescription>
            </div>
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={() => {
                void settings.refreshRepos({ showSuccessToast: true });
              }}
              disabled={settings.reposBusy || settings.addingRepo || settings.syncingRepo !== null}
            >
              {settings.reposBusy ? "Refreshing..." : "Refresh"}
            </Button>
          </div>
        </CardHeader>
        <CardContent className="space-y-4 pt-0">
          <form className="flex flex-col gap-3 sm:flex-row" onSubmit={settings.handleAddRepo}>
            <Input
              value={settings.repoInput}
              onChange={(event) => {
                settings.setRepoInput(event.target.value);
              }}
              placeholder="owner/name or https://github.com/owner/name"
              disabled={settings.addingRepo}
            />
            <Button type="submit" disabled={settings.addingRepo}>
              {settings.addingRepo ? "Adding..." : "Add Repo"}
            </Button>
          </form>

          <div className="space-y-3">
            {settings.reposBusy ? (
              <p className="text-sm text-muted-foreground">Loading cached repos...</p>
            ) : settings.repos.length === 0 ? (
              <p className="text-sm text-muted-foreground">
                No external repos added yet. Bundled chords still load by default.
              </p>
            ) : (
              settings.repos.map((repo) => (
                <div key={repo.slug} className="rounded-lg border bg-background/80 px-3 py-3">
                  <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
                    <div className="min-w-0 space-y-1">
                      <div className="flex items-center gap-2">
                        <p className="truncate font-medium">{repo.slug}</p>
                        <Badge variant="secondary">GitHub</Badge>
                        {repo.headShortSha ? (
                          <Badge variant="outline" className="font-mono text-[11px]">
                            {repo.headShortSha}
                          </Badge>
                        ) : null}
                      </div>
                    </div>
                    <div className="flex flex-wrap items-center gap-2 self-end sm:self-center">
                      <Button
                        type="button"
                        variant="ghost"
                        size="icon-sm"
                        aria-label={`Open ${repo.slug} on GitHub`}
                        title="Open on GitHub"
                        onClick={() => {
                          void settings.handleOpenRepoUrl(repo);
                        }}
                      >
                        <ExternalLink />
                      </Button>
                      <Button
                        type="button"
                        variant="outline"
                        size="sm"
                        onClick={() => {
                          void settings.handleRepoChordsToggle(repo.slug, !settings.openRepoChords[repo.slug]);
                        }}
                        disabled={settings.repoChordsBusy[repo.slug] === true}
                      >
                        {settings.openRepoChords[repo.slug] ? "Hide Chords" : "View Chords"}
                      </Button>
                      <Button
                        type="button"
                        variant="outline"
                        size="sm"
                        onClick={() => {
                          void settings.handleSyncRepo(repo.slug);
                        }}
                        disabled={settings.addingRepo || settings.syncingRepo === repo.slug}
                      >
                        {settings.syncingRepo === repo.slug ? "Syncing..." : "Sync Latest"}
                      </Button>
                    </div>
                  </div>

                  <Collapsible open={settings.openRepoChords[repo.slug] === true}>
                    <CollapsibleContent className="pt-3">
                      {settings.repoChordsBusy[repo.slug] === true ? (
                        <p className="text-sm text-muted-foreground">
                          Loading chords from {repo.slug}...
                        </p>
                      ) : (settings.repoChordsByRepo[repo.slug]?.length ?? 0) === 0 ? (
                        <p className="text-sm text-muted-foreground">
                          No chords found in {repo.slug}.
                        </p>
                      ) : (
                        <ChordGroupList
                          groups={buildChordGroups(settings.repoChordsByRepo[repo.slug] ?? [])}
                          appMetadataByBundleId={appMetadataByBundleId}
                          openGroups={settings.openRepoChordGroups[repo.slug] ?? {}}
                          onGroupOpenChange={(groupKey, open) => {
                            settings.setRepoChordGroupOpen(repo.slug, groupKey, open);
                          }}
                        />
                      )}
                    </CollapsibleContent>
                  </Collapsible>
                </div>
              ))
            )}
          </div>
        </CardContent>
      </Card>

      <Card size="sm">
        <CardHeader>
          <div className="flex items-center justify-between gap-3">
            <div>
              <CardTitle>Local Folders</CardTitle>
              <CardDescription>
                Local folders are loaded in place. Use the tray reload action after editing files to rebuild the JS runtime.
              </CardDescription>
            </div>
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={() => {
                void settings.refreshLocalChordFolders({ showSuccessToast: true });
              }}
              disabled={settings.localChordFoldersBusy || settings.addingLocalChordFolder}
            >
              {settings.localChordFoldersBusy ? "Refreshing..." : "Refresh"}
            </Button>
          </div>
        </CardHeader>
        <CardContent className="space-y-4 pt-0">
          <div className="flex justify-end">
            <Button
              type="button"
              onClick={() => {
                void settings.handleAddLocalChordFolder();
              }}
              disabled={settings.addingLocalChordFolder}
            >
              {settings.addingLocalChordFolder ? "Adding..." : "Add Folder"}
            </Button>
          </div>

          <div className="space-y-3">
            {settings.localChordFoldersBusy ? (
              <p className="text-sm text-muted-foreground">Loading local folders...</p>
            ) : settings.localChordFolders.length === 0 ? (
              <p className="text-sm text-muted-foreground">No local folders added yet.</p>
            ) : (
              settings.localChordFolders.map((folder) => (
                <div key={folder.localPath} className="rounded-lg border bg-background/80 px-3 py-3">
                  <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
                    <div className="min-w-0 space-y-1">
                      <div className="flex items-center gap-2">
                        <p className="truncate font-medium">{folder.name}</p>
                        <Badge variant="secondary">Local</Badge>
                      </div>
                      <p className="truncate text-xs text-muted-foreground">{folder.localPath}</p>
                    </div>
                    <div className="flex flex-wrap items-center gap-2 self-end sm:self-center">
                      <Button
                        type="button"
                        variant="outline"
                        size="sm"
                        onClick={() => {
                          void settings.handleLocalFolderChordsToggle(
                            folder.localPath,
                            !settings.openLocalFolderChords[folder.localPath],
                          );
                        }}
                        disabled={settings.localFolderChordsBusy[folder.localPath] === true}
                      >
                        {settings.openLocalFolderChords[folder.localPath] ? "Hide Chords" : "View Chords"}
                      </Button>
                    </div>
                  </div>

                  <Collapsible open={settings.openLocalFolderChords[folder.localPath] === true}>
                    <CollapsibleContent className="pt-3">
                      {settings.localFolderChordsBusy[folder.localPath] === true ? (
                        <p className="text-sm text-muted-foreground">
                          Loading chords from {folder.name}...
                        </p>
                      ) : (settings.localFolderChordsByPath[folder.localPath]?.length ?? 0) === 0 ? (
                        <p className="text-sm text-muted-foreground">
                          No chords found in {folder.name}.
                        </p>
                      ) : (
                        <ChordGroupList
                          groups={buildChordGroups(settings.localFolderChordsByPath[folder.localPath] ?? [])}
                          appMetadataByBundleId={appMetadataByBundleId}
                          openGroups={settings.openLocalFolderChordGroups[folder.localPath] ?? {}}
                          onGroupOpenChange={(groupKey, open) => {
                            settings.setLocalFolderChordGroupOpen(folder.localPath, groupKey, open);
                          }}
                        />
                      )}
                    </CollapsibleContent>
                  </Collapsible>
                </div>
              ))
            )}
          </div>
        </CardContent>
      </Card>

      <Card size="sm">
        <CardHeader>
          <div className="flex items-center justify-between gap-3">
            <div>
              <CardTitle>Apps Needing Relaunch</CardTitle>
              <CardDescription>
                Scripts can flag apps that should be restarted after they change app state.
              </CardDescription>
            </div>
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={() => {
                void settings.refreshAppsNeedingRelaunch({ showSuccessToast: true });
              }}
              disabled={settings.appsNeedingRelaunchBusy || settings.relaunchingApp !== null}
            >
              {settings.appsNeedingRelaunchBusy ? "Refreshing..." : "Refresh"}
            </Button>
          </div>
        </CardHeader>
        <CardContent className="space-y-2 pt-0">
          {settings.appsNeedingRelaunchBusy ? (
            <p className="text-sm text-muted-foreground">Loading relaunch requests...</p>
          ) : settings.appsNeedingRelaunch.length === 0 ? (
            <p className="text-sm text-muted-foreground">
              No apps are currently marked as needing a relaunch.
            </p>
          ) : (
            settings.appsNeedingRelaunch.map((app) => {
              const isRelaunching = settings.relaunchingApp === app.bundleId;
              const appMetadata = appMetadataByBundleId[app.bundleId];
              const appLabel = getAppLabel(app.bundleId, appMetadata, app.displayName);

              return (
                <div
                  key={app.bundleId}
                  className="flex items-center justify-between gap-3 rounded-lg border bg-background/80 px-3 py-2"
                >
                  <div className="flex min-w-0 items-center gap-2">
                    <AppIcon appMetadata={appMetadata} label={appLabel} />
                    <div className="min-w-0">
                      <div className="flex items-center gap-2">
                        <p className="truncate font-medium">{appLabel}</p>
                        <Badge variant="secondary">Needs relaunch</Badge>
                      </div>
                      {appLabel !== app.bundleId ? (
                        <p className="truncate text-xs text-muted-foreground">{app.bundleId}</p>
                      ) : null}
                    </div>
                  </div>

                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    onClick={() => {
                      void settings.handleRelaunchApp(app.bundleId);
                    }}
                    disabled={isRelaunching}
                  >
                    {isRelaunching ? "Relaunching..." : "Relaunch"}
                  </Button>
                </div>
              );
            })
          )}
        </CardContent>
      </Card>

      <Card size="sm">
        <CardHeader>
          <CardTitle>Permissions</CardTitle>
          <CardDescription>
            Grant macOS access for clicking chords and listening for the global shortcut.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-2 pt-0">
          <div className="flex items-center justify-between gap-3 rounded-lg border bg-background/80 px-3 py-2">
            <div className="min-w-0">
              <p className="truncate font-medium">Accessibility</p>
              <p className="truncate text-xs text-muted-foreground">Needed for automated clicking.</p>
            </div>
            {settings.hasAccessibilityPermission ? (
              <Button
                type="button"
                variant="ghost"
                size="icon-sm"
                aria-label="Open Accessibility settings"
                title="Open Accessibility settings"
                onClick={() => {
                  void settings.handleAccessibilityButtonClick();
                }}
                disabled={settings.accessibilityBusy}
              >
                <Check className="text-emerald-600" />
              </Button>
            ) : (
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={() => {
                  void settings.handleAccessibilityButtonClick();
                }}
                disabled={settings.accessibilityBusy}
              >
                {settings.accessibilityBusy ? "Requesting..." : "Grant"}
              </Button>
            )}
          </div>

          <div className="flex items-center justify-between gap-3 rounded-lg border bg-background/80 px-3 py-2">
            <div className="min-w-0">
              <p className="truncate font-medium">Input Monitoring</p>
              <p className="truncate text-xs text-muted-foreground">
                Needed for the global shortcut; restart after enabling.
              </p>
            </div>
            {settings.hasInputMonitoringPermission ? (
              <Button
                type="button"
                variant="ghost"
                size="icon-sm"
                aria-label="Open Input Monitoring settings"
                title="Open Input Monitoring settings"
                onClick={() => {
                  void settings.handleInputMonitoringButtonClick();
                }}
                disabled={settings.inputMonitoringBusy}
              >
                <Check className="text-emerald-600" />
              </Button>
            ) : (
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={() => {
                  void settings.handleInputMonitoringButtonClick();
                }}
                disabled={settings.inputMonitoringBusy}
              >
                {settings.inputMonitoringBusy ? "Opening..." : "Grant"}
              </Button>
            )}
          </div>
        </CardContent>
      </Card>

      <Card size="sm">
        <CardHeader>
          <CardTitle>Launch on Login</CardTitle>
          <CardDescription>{settings.autostartStatus}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-3 pt-0">
          <div className="flex items-start gap-3">
            <Checkbox
              id="launch-on-login"
              checked={settings.autostartEnabled}
              disabled={settings.autostartBusy}
              onCheckedChange={(checked) => {
                void settings.handleAutostartChange(checked === true);
              }}
            />
            <div className="space-y-1">
              <Label htmlFor="launch-on-login">Launch Chords on login</Label>
              <p className="text-sm text-muted-foreground">
                The app stays in the tray, reuses a single instance, and launches hidden on login.
              </p>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
