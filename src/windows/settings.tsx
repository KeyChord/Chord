import { useEffect, useState, type FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  disable as disableAutostart,
  enable as enableAutostart,
  isEnabled as isAutostartEnabled,
} from "@tauri-apps/plugin-autostart";
import { openPath } from "@tauri-apps/plugin-opener";
import { Check, ChevronRight, FolderOpen } from "lucide-react";
import { toast } from "sonner";
import {
  checkAccessibilityPermission,
  checkInputMonitoringPermission,
  requestAccessibilityPermission,
  requestInputMonitoringPermission,
} from "tauri-plugin-macos-permissions-api";
import { Badge } from "#/components/ui/badge.tsx";
import { Button } from "#/components/ui/button.tsx";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "#/components/ui/card.tsx";
import { Checkbox } from "#/components/ui/checkbox.tsx";
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "#/components/ui/collapsible.tsx";
import { Input } from "#/components/ui/input.tsx";
import { Label } from "#/components/ui/label.tsx";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "#/components/ui/tabs.tsx";

type GitRepoInfo = {
  owner: string;
  name: string;
  slug: string;
  url: string;
  localPath: string;
  headShortSha: string | null;
};

type ActiveChordInfo = {
  scope: string;
  scopeKind: "global" | "app";
  sequence: string;
  name: string;
  action: string;
};

function getErrorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

export function SettingsWindow() {
  const currentWindow = getCurrentWindow();
  const isMacOS = navigator.userAgent.includes("Mac");
  const [accessibilityBusy, setAccessibilityBusy] = useState(false);
  const [inputMonitoringBusy, setInputMonitoringBusy] = useState(false);
  const [autostartBusy, setAutostartBusy] = useState(false);
  const [hasAccessibilityPermission, setHasAccessibilityPermission] = useState(!isMacOS);
  const [hasInputMonitoringPermission, setHasInputMonitoringPermission] = useState(!isMacOS);
  const [autostartEnabled, setAutostartEnabled] = useState(false);
  const [autostartStatus, setAutostartStatus] = useState("Checking launch on login...");
  const [repos, setRepos] = useState<GitRepoInfo[]>([]);
  const [reposBusy, setReposBusy] = useState(true);
  const [repoInput, setRepoInput] = useState("");
  const [addingRepo, setAddingRepo] = useState(false);
  const [syncingRepo, setSyncingRepo] = useState<string | null>(null);
  const [activeChords, setActiveChords] = useState<ActiveChordInfo[]>([]);
  const [activeChordsBusy, setActiveChordsBusy] = useState(true);
  const [chordSearch, setChordSearch] = useState("");
  const [openChordGroups, setOpenChordGroups] = useState<Record<string, boolean>>({});

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    void currentWindow
      .onCloseRequested((event) => {
        event.preventDefault();
        void currentWindow.hide();
      })
      .then((callback) => {
        unlisten = callback;
      });

    return () => {
      unlisten?.();
    };
  }, [currentWindow]);

  async function refreshAccessibilityPermissionState() {
    if (!isMacOS) {
      setHasAccessibilityPermission(true);
      return true;
    }

    const granted = await checkAccessibilityPermission();
    setHasAccessibilityPermission(granted);
    return granted;
  }

  async function refreshInputMonitoringPermissionState() {
    if (!isMacOS) {
      setHasInputMonitoringPermission(true);
      return true;
    }

    const granted = await checkInputMonitoringPermission();
    setHasInputMonitoringPermission(granted);
    return granted;
  }

  async function refreshAutostartState() {
    const enabled = await isAutostartEnabled();
    setAutostartEnabled(enabled);
    setAutostartStatus(
      enabled ? "Chords launches automatically when you log in." : "Chords will not launch on login.",
    );
    return enabled;
  }

  async function refreshRepos(options?: { showSuccessToast?: boolean; showErrorToast?: boolean }) {
    const { showSuccessToast = false, showErrorToast = true } = options ?? {};
    setReposBusy(true);

    try {
      const nextRepos = await invoke<GitRepoInfo[]>("list_git_repos");
      setRepos(nextRepos);

      if (showSuccessToast) {
        toast.success("Repo list refreshed.");
      }

      return nextRepos;
    } catch (error) {
      const message = `Failed to load repos: ${getErrorMessage(error)}`;
      if (showErrorToast) {
        toast.error(message);
      }
      return [];
    } finally {
      setReposBusy(false);
    }
  }

  async function refreshActiveChords(options?: {
    showSuccessToast?: boolean;
    showErrorToast?: boolean;
  }) {
    const { showSuccessToast = false, showErrorToast = true } = options ?? {};
    setActiveChordsBusy(true);

    try {
      const nextChords = await invoke<ActiveChordInfo[]>("list_active_chords_command");
      setActiveChords(nextChords);

      if (showSuccessToast) {
        toast.success("Active chord list refreshed.");
      }

      return nextChords;
    } catch (error) {
      const message = `Failed to load active chords: ${getErrorMessage(error)}`;
      if (showErrorToast) {
        toast.error(message);
      }
      return [];
    } finally {
      setActiveChordsBusy(false);
    }
  }

  async function ensureAccessibilityPermission() {
    const granted = await refreshAccessibilityPermissionState();

    if (granted) {
      return true;
    }

    setAccessibilityBusy(true);
    let updated = false;

    try {
      await requestAccessibilityPermission();
    } finally {
      updated = await refreshAccessibilityPermissionState();
      setAccessibilityBusy(false);
    }

    return updated;
  }

  async function ensureInputMonitoringPermission() {
    const granted = await refreshInputMonitoringPermissionState();

    if (granted) {
      return true;
    }

    setInputMonitoringBusy(true);
    let updated = false;

    try {
      await requestInputMonitoringPermission();
    } finally {
      updated = await refreshInputMonitoringPermissionState();
      setInputMonitoringBusy(false);
    }

    return updated;
  }

  async function handleAutostartChange(nextValue: boolean) {
    setAutostartBusy(true);

    try {
      if (nextValue) {
        await enableAutostart();
      } else {
        await disableAutostart();
      }

      setAutostartEnabled(nextValue);
      setAutostartStatus(
        nextValue ? "Chords launches automatically when you log in." : "Chords will not launch on login.",
      );
      toast.success(nextValue ? "Launch on login enabled." : "Launch on login disabled.");
    } catch (error) {
      const message = getErrorMessage(error);
      setAutostartStatus(`Launch on login update failed: ${message}`);
      toast.error(`Launch on login update failed: ${message}`);
      await refreshAutostartState();
    } finally {
      setAutostartBusy(false);
    }
  }

  async function handleAccessibilityButtonClick() {
    try {
      if (hasAccessibilityPermission) {
        await invoke("open_accessibility_settings");
        toast.info("Opened Accessibility settings.");
        return;
      }

      const granted = await ensureAccessibilityPermission();
      if (granted) {
        toast.success("Accessibility permission granted.");
      } else {
        toast.error("Accessibility permission was not granted.");
      }
    } catch (error) {
      toast.error(`Accessibility action failed: ${getErrorMessage(error)}`);
    }
  }

  async function handleInputMonitoringButtonClick() {
    try {
      if (hasInputMonitoringPermission) {
        await invoke("open_input_monitoring_settings");
        toast.info("Opened Input Monitoring settings.");
        return;
      }

      const granted = await ensureInputMonitoringPermission();
      if (granted) {
        toast.success("Input Monitoring permission granted.");
      } else {
        toast.error("Input Monitoring permission was not granted.");
      }
    } catch (error) {
      toast.error(`Input Monitoring action failed: ${getErrorMessage(error)}`);
    }
  }

  async function handleAddRepo(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();

    if (!repoInput.trim()) {
      toast.error("Enter a GitHub repo like owner/name or https://github.com/owner/name.");
      return;
    }

    setAddingRepo(true);
    const toastId = toast.loading(`Adding ${repoInput.trim()}...`);

    try {
      const addedRepo = await invoke<GitRepoInfo>("add_git_repo_command", { repo: repoInput });
      setRepoInput("");
      await Promise.all([
        refreshRepos({ showErrorToast: false }),
        refreshActiveChords({ showErrorToast: false }),
      ]);
      toast.success(`Added ${addedRepo.slug}.`, { id: toastId });
    } catch (error) {
      toast.error(`Failed to add repo: ${getErrorMessage(error)}`, { id: toastId });
    } finally {
      setAddingRepo(false);
    }
  }

  async function handleSyncRepo(repoSlug: string) {
    setSyncingRepo(repoSlug);
    const toastId = toast.loading(`Syncing ${repoSlug}...`);

    try {
      const syncedRepo = await invoke<GitRepoInfo>("sync_git_repo_command", { repo: repoSlug });
      await Promise.all([
        refreshRepos({ showErrorToast: false }),
        refreshActiveChords({ showErrorToast: false }),
      ]);
      const revisionLabel = syncedRepo.headShortSha ? ` @ ${syncedRepo.headShortSha}` : "";
      toast.success(`Synced ${syncedRepo.slug}${revisionLabel}.`, { id: toastId });
    } catch (error) {
      toast.error(`Failed to sync ${repoSlug}: ${getErrorMessage(error)}`, { id: toastId });
    } finally {
      setSyncingRepo(null);
    }
  }

  async function handleOpenRepoInFinder(repo: GitRepoInfo) {
    try {
      await openPath(repo.localPath);
      toast.info(`Opened ${repo.slug} in Finder.`);
    } catch (error) {
      toast.error(`Failed to open ${repo.slug}: ${getErrorMessage(error)}`);
    }
  }

  useEffect(() => {
    let cancelled = false;

    async function configureWindow() {
      try {
        await Promise.all([
          refreshAccessibilityPermissionState(),
          refreshInputMonitoringPermissionState(),
          refreshAutostartState(),
          refreshRepos({ showErrorToast: true }),
          refreshActiveChords({ showErrorToast: true }),
        ]);
      } catch (error) {
        if (!cancelled) {
          toast.error(`Failed to finish loading settings: ${getErrorMessage(error)}`);
        }
      }
    }

    void configureWindow();

    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    setOpenChordGroups((current) => {
      const next = { ...current };

      for (const chord of activeChords) {
        const groupKey = `${chord.scopeKind}:${chord.scope}`;
        if (next[groupKey] === undefined) {
          next[groupKey] = chord.scopeKind === "global";
        }
      }

      return next;
    });
  }, [activeChords]);

  const normalizedChordSearch = chordSearch.trim().toLowerCase();
  const filteredActiveChords = normalizedChordSearch
    ? activeChords.filter((chord) =>
        [chord.scope, chord.sequence, chord.name, chord.action].some((value) =>
          value.toLowerCase().includes(normalizedChordSearch),
        ),
      )
    : activeChords;

  const chordGroups: Array<{
    key: string;
    scope: string;
    scopeKind: "global" | "app";
    chords: ActiveChordInfo[];
  }> = [];
  const chordGroupMap = new Map<string, (typeof chordGroups)[number]>();
  for (const chord of filteredActiveChords) {
    const key = `${chord.scopeKind}:${chord.scope}`;
    let group = chordGroupMap.get(key);
    if (!group) {
      group = { key, scope: chord.scope, scopeKind: chord.scopeKind, chords: [] };
      chordGroupMap.set(key, group);
      chordGroups.push(group);
    }
    group.chords.push(chord);
  }

  chordGroups.sort((left, right) =>
    left.scopeKind
      .localeCompare(right.scopeKind)
      || left.scope.localeCompare(right.scope),
  );

  for (const group of chordGroups) {
    group.chords.sort(
      (left, right) =>
        left.sequence.localeCompare(right.sequence)
        || left.name.localeCompare(right.name)
        || left.action.localeCompare(right.action),
    );
  }

  return (
    <div className="min-h-full bg-muted/30 px-5 py-4 text-sm text-foreground">
      <div className="mx-auto flex max-w-[720px] flex-col gap-4">
        <div className="flex items-start justify-between gap-3">
          <div>
            <h1 className="text-[20px] font-semibold">Chords</h1>
            <p className="mt-1 text-muted-foreground">
              Configure the tray app, manage chord repos, and inspect the active chord registry.
            </p>
          </div>
          <div className="flex items-center gap-2">
            <Badge variant="outline">{repos.length} repos</Badge>
            <Badge variant="outline">{activeChords.length} chords</Badge>
          </div>
        </div>

        <Tabs defaultValue="settings" className="gap-4">
          <TabsList>
            <TabsTrigger value="settings">Settings</TabsTrigger>
            <TabsTrigger value="active-chords">Active Chords</TabsTrigger>
          </TabsList>

          <TabsContent value="settings" className="space-y-4">
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
                      void refreshRepos({ showSuccessToast: true });
                    }}
                    disabled={reposBusy || addingRepo || syncingRepo !== null}
                  >
                    {reposBusy ? "Refreshing..." : "Refresh"}
                  </Button>
                </div>
              </CardHeader>
              <CardContent className="space-y-4 pt-0">
                <form className="flex flex-col gap-3 sm:flex-row" onSubmit={handleAddRepo}>
                  <Input
                    value={repoInput}
                    onChange={(event) => {
                      setRepoInput(event.target.value);
                    }}
                    placeholder="owner/name or https://github.com/owner/name"
                    disabled={addingRepo}
                  />
                  <Button type="submit" disabled={addingRepo}>
                    {addingRepo ? "Adding..." : "Add Repo"}
                  </Button>
                </form>

                <div className="space-y-3">
                  {reposBusy ? (
                    <p className="text-sm text-muted-foreground">Loading cached repos...</p>
                  ) : repos.length === 0 ? (
                    <p className="text-sm text-muted-foreground">
                      No external repos added yet. Bundled chords still load by default.
                    </p>
                  ) : (
                    repos.map((repo) => (
                      <div
                        key={repo.slug}
                        className="flex flex-col gap-3 rounded-lg border bg-background/80 px-3 py-3 sm:flex-row sm:items-center sm:justify-between"
                      >
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
                          <p className="truncate text-sm text-muted-foreground">{repo.url}</p>
                        </div>
                        <div className="flex items-center gap-2 self-end sm:self-center">
                          <Button
                            type="button"
                            variant="ghost"
                            size="icon-sm"
                            aria-label={`Open ${repo.slug} in Finder`}
                            title="Open in Finder"
                            onClick={() => {
                              void handleOpenRepoInFinder(repo);
                            }}
                          >
                            <FolderOpen />
                          </Button>
                          <Button
                            type="button"
                            variant="outline"
                            size="sm"
                            onClick={() => {
                              void handleSyncRepo(repo.slug);
                            }}
                            disabled={addingRepo || syncingRepo === repo.slug}
                          >
                            {syncingRepo === repo.slug ? "Syncing..." : "Sync Latest"}
                          </Button>
                        </div>
                      </div>
                    ))
                  )}
                </div>
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
                    <p className="truncate text-xs text-muted-foreground">
                      Needed for automated clicking.
                    </p>
                  </div>
                  {hasAccessibilityPermission ? (
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon-sm"
                      aria-label="Open Accessibility settings"
                      title="Open Accessibility settings"
                      onClick={() => {
                        void handleAccessibilityButtonClick();
                      }}
                      disabled={accessibilityBusy}
                    >
                      <Check className="text-emerald-600" />
                    </Button>
                  ) : (
                    <Button
                      type="button"
                      variant="outline"
                      size="sm"
                      onClick={() => {
                        void handleAccessibilityButtonClick();
                      }}
                      disabled={accessibilityBusy}
                    >
                      {accessibilityBusy ? "Requesting..." : "Grant"}
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
                  {hasInputMonitoringPermission ? (
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon-sm"
                      aria-label="Open Input Monitoring settings"
                      title="Open Input Monitoring settings"
                      onClick={() => {
                        void handleInputMonitoringButtonClick();
                      }}
                      disabled={inputMonitoringBusy}
                    >
                      <Check className="text-emerald-600" />
                    </Button>
                  ) : (
                    <Button
                      type="button"
                      variant="outline"
                      size="sm"
                      onClick={() => {
                        void handleInputMonitoringButtonClick();
                      }}
                      disabled={inputMonitoringBusy}
                    >
                      {inputMonitoringBusy ? "Opening..." : "Grant"}
                    </Button>
                  )}
                </div>
              </CardContent>
            </Card>

            <Card size="sm">
              <CardHeader>
                <CardTitle>Launch on Login</CardTitle>
                <CardDescription>{autostartStatus}</CardDescription>
              </CardHeader>
              <CardContent className="space-y-3 pt-0">
                <div className="flex items-start gap-3">
                  <Checkbox
                    id="launch-on-login"
                    checked={autostartEnabled}
                    disabled={autostartBusy}
                    onCheckedChange={(checked) => {
                      void handleAutostartChange(checked === true);
                    }}
                  />
                  <div className="space-y-1">
                    <Label htmlFor="launch-on-login">Launch Chords on login</Label>
                    <p className="text-sm text-muted-foreground">
                      The app stays in the tray, reuses a single instance, and launches hidden on
                      login.
                    </p>
                  </div>
                </div>
              </CardContent>
            </Card>
          </TabsContent>

          <TabsContent value="active-chords">
            <Card size="sm">
              <CardHeader>
                <div className="flex items-center justify-between gap-3">
                  <div>
                    <CardTitle>Registered Chords</CardTitle>
                    <CardDescription>
                      Live view of the chord registry loaded in `context.loaded_app_chords`.
                    </CardDescription>
                  </div>
                  <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    onClick={() => {
                      void refreshActiveChords({ showSuccessToast: true });
                    }}
                    disabled={activeChordsBusy}
                  >
                    {activeChordsBusy ? "Refreshing..." : "Refresh"}
                  </Button>
                </div>
              </CardHeader>
              <CardContent className="space-y-3 pt-0">
                <div className="flex flex-col gap-3 sm:flex-row sm:items-center">
                  <Input
                    value={chordSearch}
                    onChange={(event) => {
                      setChordSearch(event.target.value);
                    }}
                    placeholder="Filter by app, trigger, name, or action"
                  />
                  <Badge variant="outline" className="self-start sm:self-center">
                    {filteredActiveChords.length} matches
                  </Badge>
                </div>

                {activeChordsBusy ? (
                  <p className="text-sm text-muted-foreground">Loading active chords...</p>
                ) : activeChords.length === 0 ? (
                  <p className="text-sm text-muted-foreground">No chords are currently loaded.</p>
                ) : filteredActiveChords.length === 0 ? (
                  <p className="text-sm text-muted-foreground">No chords match that filter.</p>
                ) : (
                  <div className="space-y-2">
                    {chordGroups.map((group) => {
                      const forcedOpen = normalizedChordSearch.length > 0;
                      const isOpen = forcedOpen || openChordGroups[group.key] === true;

                      return (
                        <Collapsible
                          key={group.key}
                          open={isOpen}
                          onOpenChange={(open) => {
                            setOpenChordGroups((current) => ({
                              ...current,
                              [group.key]: open,
                            }));
                          }}
                        >
                          <CollapsibleTrigger asChild>
                            <button
                              type="button"
                              className="flex w-full items-center gap-2 rounded-md border bg-background/80 px-2.5 py-1.5 text-left hover:bg-muted/70"
                            >
                              <ChevronRight
                                className={`size-3.5 shrink-0 transition-transform ${isOpen ? "rotate-90" : ""}`}
                              />
                              <Badge
                                variant={group.scopeKind === "global" ? "secondary" : "outline"}
                              >
                                {group.scopeKind === "global" ? "Global" : "App"}
                              </Badge>
                              <span className="min-w-0 flex-1 truncate text-sm font-medium">
                                {group.scope}
                              </span>
                              <span className="text-xs text-muted-foreground">
                                {group.chords.length}
                              </span>
                            </button>
                          </CollapsibleTrigger>

                          <CollapsibleContent className="pt-1">
                            <div className="overflow-hidden rounded-md border bg-background/80">
                              {group.chords.map((chord) => (
                                <div
                                  key={`${chord.scopeKind}:${chord.scope}:${chord.sequence}:${chord.name}`}
                                  className="grid grid-cols-[86px_minmax(0,1fr)] gap-x-3 border-b px-2.5 py-1.5 text-xs last:border-b-0"
                                >
                                  <div className="truncate font-mono text-[11px] text-foreground/85">
                                    {chord.sequence}
                                  </div>
                                  <div className="min-w-0">
                                    <div className="flex items-baseline gap-2">
                                      <span className="truncate font-medium">{chord.name}</span>
                                      <span className="truncate text-muted-foreground">
                                        {chord.action}
                                      </span>
                                    </div>
                                  </div>
                                </div>
                              ))}
                            </div>
                          </CollapsibleContent>
                        </Collapsible>
                      );
                    })}
                  </div>
                )}
              </CardContent>
            </Card>
          </TabsContent>
        </Tabs>
      </div>
    </div>
  );
}
