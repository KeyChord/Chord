import { useEffect, useState, type FormEvent } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  disable as disableAutostart,
  enable as enableAutostart,
  isEnabled as isAutostartEnabled,
} from "@tauri-apps/plugin-autostart";
import { openUrl } from "@tauri-apps/plugin-opener";
import { toast } from "sonner";
import {
  checkAccessibilityPermission,
  checkInputMonitoringPermission,
  requestAccessibilityPermission,
  requestInputMonitoringPermission,
} from "tauri-plugin-macos-permissions-api";
import {
  addGitRepo,
  addLocalChordFolder,
  listActiveChords,
  listAppsNeedingRelaunch,
  listGitRepos,
  listGlobalShortcutMappings,
  listLocalChordFolderChords,
  listLocalChordFolders,
  listRepoChords,
  openAccessibilitySettings,
  openInputMonitoringSettings,
  pickLocalChordFolder,
  relaunchApp,
  removeGlobalShortcutMapping,
  syncGitRepo,
  type ActiveChordInfo,
  type AppNeedsRelaunchInfo,
  type GitRepoInfo,
  type GlobalShortcutMappingInfo,
  type LocalChordFolderInfo,
} from "#/api/settings.ts";
import {
  buildChordGroups,
  getAppLabel,
  getErrorMessage,
  type AppMetadataByBundleId,
  validateLocalChordFolder,
} from "#/utils/settings.ts";
import { useAppMetadata } from "#/utils/use-app-metadata.ts";

type RefreshOptions = {
  showSuccessToast?: boolean;
  showErrorToast?: boolean;
};

export function useSettingsPage() {
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
  const [localChordFolders, setLocalChordFolders] = useState<LocalChordFolderInfo[]>([]);
  const [localChordFoldersBusy, setLocalChordFoldersBusy] = useState(true);
  const [addingLocalChordFolder, setAddingLocalChordFolder] = useState(false);
  const [activeChords, setActiveChords] = useState<ActiveChordInfo[]>([]);
  const [activeChordsBusy, setActiveChordsBusy] = useState(true);
  const [chordSearch, setChordSearch] = useState("");
  const [globalShortcutMappings, setGlobalShortcutMappings] = useState<GlobalShortcutMappingInfo[]>([]);
  const [globalShortcutMappingsBusy, setGlobalShortcutMappingsBusy] = useState(true);
  const [globalShortcutSearch, setGlobalShortcutSearch] = useState("");
  const [removingGlobalShortcut, setRemovingGlobalShortcut] = useState<string | null>(null);
  const [appsNeedingRelaunch, setAppsNeedingRelaunch] = useState<AppNeedsRelaunchInfo[]>([]);
  const [appsNeedingRelaunchBusy, setAppsNeedingRelaunchBusy] = useState(true);
  const [relaunchingApp, setRelaunchingApp] = useState<string | null>(null);
  const [openChordGroups, setOpenChordGroups] = useState<Record<string, boolean>>({});
  const [repoChordsByRepo, setRepoChordsByRepo] = useState<Record<string, ActiveChordInfo[]>>({});
  const [repoChordsBusy, setRepoChordsBusy] = useState<Record<string, boolean>>({});
  const [openRepoChords, setOpenRepoChords] = useState<Record<string, boolean>>({});
  const [openRepoChordGroups, setOpenRepoChordGroups] = useState<Record<string, Record<string, boolean>>>({});
  const [localFolderChordsByPath, setLocalFolderChordsByPath] = useState<Record<string, ActiveChordInfo[]>>({});
  const [localFolderChordsBusy, setLocalFolderChordsBusy] = useState<Record<string, boolean>>({});
  const [openLocalFolderChords, setOpenLocalFolderChords] = useState<Record<string, boolean>>({});
  const [openLocalFolderChordGroups, setOpenLocalFolderChordGroups] = useState<
    Record<string, Record<string, boolean>>
  >({});

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

  useEffect(() => {
    let unlisten: (() => void) | undefined;

    void listen<AppNeedsRelaunchInfo[]>("apps-needing-relaunch-changed", (event) => {
      setAppsNeedingRelaunch(event.payload);
      setAppsNeedingRelaunchBusy(false);
      setRelaunchingApp((current) =>
        current && event.payload.some((app) => app.bundleId === current) ? current : null,
      );
    })
      .then((callback) => {
        unlisten = callback;
      })
      .catch((error) => {
        console.error("Failed to listen for relaunch updates", error);
      });

    return () => {
      unlisten?.();
    };
  }, []);

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

  async function refreshRepos(options?: RefreshOptions) {
    const { showSuccessToast = false, showErrorToast = true } = options ?? {};
    setReposBusy(true);

    try {
      const nextRepos = await listGitRepos();
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

  async function refreshLocalChordFolders(options?: RefreshOptions) {
    const { showSuccessToast = false, showErrorToast = true } = options ?? {};
    setLocalChordFoldersBusy(true);

    try {
      const nextFolders = await listLocalChordFolders();
      setLocalChordFolders(nextFolders);

      if (showSuccessToast) {
        toast.success("Local folder list refreshed.");
      }

      return nextFolders;
    } catch (error) {
      const message = `Failed to load local folders: ${getErrorMessage(error)}`;
      if (showErrorToast) {
        toast.error(message);
      }
      return [];
    } finally {
      setLocalChordFoldersBusy(false);
    }
  }

  async function refreshActiveChords(options?: RefreshOptions) {
    const { showSuccessToast = false, showErrorToast = true } = options ?? {};
    setActiveChordsBusy(true);

    try {
      const nextChords = await listActiveChords();
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

  async function refreshGlobalShortcutMappings(options?: RefreshOptions) {
    const { showSuccessToast = false, showErrorToast = true } = options ?? {};
    setGlobalShortcutMappingsBusy(true);

    try {
      const nextMappings = await listGlobalShortcutMappings();
      setGlobalShortcutMappings(nextMappings);

      if (showSuccessToast) {
        toast.success("Global shortcut mappings refreshed.");
      }

      return nextMappings;
    } catch (error) {
      const message = `Failed to load global shortcut mappings: ${getErrorMessage(error)}`;
      if (showErrorToast) {
        toast.error(message);
      }
      return [];
    } finally {
      setGlobalShortcutMappingsBusy(false);
    }
  }

  async function refreshAppsNeedingRelaunch(options?: RefreshOptions) {
    const { showSuccessToast = false, showErrorToast = true } = options ?? {};
    setAppsNeedingRelaunchBusy(true);

    try {
      const nextApps = await listAppsNeedingRelaunch();
      setAppsNeedingRelaunch(nextApps);

      if (showSuccessToast) {
        toast.success("Relaunch list refreshed.");
      }

      return nextApps;
    } catch (error) {
      const message = `Failed to load relaunch list: ${getErrorMessage(error)}`;
      if (showErrorToast) {
        toast.error(message);
      }
      return [];
    } finally {
      setAppsNeedingRelaunchBusy(false);
    }
  }

  async function refreshRepoChords(repoSlug: string, options?: RefreshOptions) {
    const { showSuccessToast = false, showErrorToast = true } = options ?? {};
    setRepoChordsBusy((current) => ({ ...current, [repoSlug]: true }));

    try {
      const nextChords = await listRepoChords(repoSlug);
      setRepoChordsByRepo((current) => ({ ...current, [repoSlug]: nextChords }));
      setOpenRepoChordGroups((current) => {
        const next = { ...(current[repoSlug] ?? {}) };

        for (const chord of nextChords) {
          const groupKey = `${chord.scopeKind}:${chord.scope}`;
          if (next[groupKey] === undefined) {
            next[groupKey] = chord.scopeKind === "global";
          }
        }

        return { ...current, [repoSlug]: next };
      });

      if (showSuccessToast) {
        toast.success(`Loaded chords from ${repoSlug}.`);
      }

      return nextChords;
    } catch (error) {
      const message = `Failed to load chords from ${repoSlug}: ${getErrorMessage(error)}`;
      if (showErrorToast) {
        toast.error(message);
      }
      return [];
    } finally {
      setRepoChordsBusy((current) => ({ ...current, [repoSlug]: false }));
    }
  }

  async function refreshLocalFolderChords(folderPath: string, options?: RefreshOptions) {
    const { showSuccessToast = false, showErrorToast = true } = options ?? {};
    setLocalFolderChordsBusy((current) => ({ ...current, [folderPath]: true }));

    try {
      const nextChords = await listLocalChordFolderChords(folderPath);
      setLocalFolderChordsByPath((current) => ({ ...current, [folderPath]: nextChords }));
      setOpenLocalFolderChordGroups((current) => {
        const next = { ...(current[folderPath] ?? {}) };

        for (const chord of nextChords) {
          const groupKey = `${chord.scopeKind}:${chord.scope}`;
          if (next[groupKey] === undefined) {
            next[groupKey] = chord.scopeKind === "global";
          }
        }

        return { ...current, [folderPath]: next };
      });

      if (showSuccessToast) {
        toast.success("Loaded chords from local folder.");
      }

      return nextChords;
    } catch (error) {
      const message = `Failed to load local folder chords: ${getErrorMessage(error)}`;
      if (showErrorToast) {
        toast.error(message);
      }
      return [];
    } finally {
      setLocalFolderChordsBusy((current) => ({ ...current, [folderPath]: false }));
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
        await openAccessibilitySettings();
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
        await openInputMonitoringSettings();
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
      const addedRepo = await addGitRepo(repoInput);
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

  async function handleAddLocalChordFolder() {
    let selectedPath: string | null = null;

    try {
      selectedPath = await pickLocalChordFolder();
      if (!selectedPath) {
        return;
      }
    } catch (error) {
      toast.error(`Failed to choose folder: ${getErrorMessage(error)}`);
      return;
    }

    setAddingLocalChordFolder(true);
    const toastId = toast.loading("Adding local folder...");

    try {
      await validateLocalChordFolder(selectedPath);
      const addedFolder = await addLocalChordFolder(selectedPath);
      await Promise.all([
        refreshLocalChordFolders({ showErrorToast: false }),
        refreshActiveChords({ showErrorToast: false }),
      ]);
      toast.success(`Added ${addedFolder.name}.`, { id: toastId });
    } catch (error) {
      toast.error(`Failed to add local folder: ${getErrorMessage(error)}`, { id: toastId });
    } finally {
      setAddingLocalChordFolder(false);
    }
  }

  async function handleSyncRepo(repoSlug: string) {
    setSyncingRepo(repoSlug);
    const toastId = toast.loading(`Syncing ${repoSlug}...`);

    try {
      const syncedRepo = await syncGitRepo(repoSlug);
      setRepoChordsByRepo((current) => {
        const next = { ...current };
        delete next[repoSlug];
        return next;
      });
      setOpenRepoChordGroups((current) => {
        const next = { ...current };
        delete next[repoSlug];
        return next;
      });
      await Promise.all([
        refreshRepos({ showErrorToast: false }),
        refreshActiveChords({ showErrorToast: false }),
        openRepoChords[repoSlug]
          ? refreshRepoChords(repoSlug, { showErrorToast: false })
          : Promise.resolve([]),
      ]);
      const revisionLabel = syncedRepo.headShortSha ? ` @ ${syncedRepo.headShortSha}` : "";
      toast.success(`Synced ${syncedRepo.slug}${revisionLabel}.`, { id: toastId });
    } catch (error) {
      toast.error(`Failed to sync ${repoSlug}: ${getErrorMessage(error)}`, { id: toastId });
    } finally {
      setSyncingRepo(null);
    }
  }

  async function handleOpenRepoUrl(repo: GitRepoInfo) {
    try {
      await openUrl(repo.url);
      toast.info(`Opened ${repo.slug} on GitHub.`);
    } catch (error) {
      toast.error(`Failed to open ${repo.slug}: ${getErrorMessage(error)}`);
    }
  }

  async function handleRepoChordsToggle(repoSlug: string, nextOpen: boolean) {
    setOpenRepoChords((current) => ({ ...current, [repoSlug]: nextOpen }));

    if (!nextOpen || repoChordsByRepo[repoSlug] || repoChordsBusy[repoSlug]) {
      return;
    }

    await refreshRepoChords(repoSlug);
  }

  async function handleLocalFolderChordsToggle(folderPath: string, nextOpen: boolean) {
    setOpenLocalFolderChords((current) => ({ ...current, [folderPath]: nextOpen }));

    if (!nextOpen || localFolderChordsByPath[folderPath] || localFolderChordsBusy[folderPath]) {
      return;
    }

    await refreshLocalFolderChords(folderPath);
  }

  async function handleRemoveGlobalShortcutMapping(shortcut: string) {
    setRemovingGlobalShortcut(shortcut);

    try {
      await removeGlobalShortcutMapping(shortcut);
      setGlobalShortcutMappings((current) =>
        current.filter((mapping) => mapping.shortcut !== shortcut),
      );
      toast.success(`Removed ${shortcut}.`);
    } catch (error) {
      toast.error(`Failed to remove ${shortcut}: ${getErrorMessage(error)}`);
    } finally {
      setRemovingGlobalShortcut(null);
    }
  }

  async function handleRelaunchApp(bundleId: string) {
    const app = appsNeedingRelaunch.find((item) => item.bundleId === bundleId);
    const appLabel = getAppLabel(bundleId, appMetadataByBundleId[bundleId], app?.displayName);
    setRelaunchingApp(bundleId);

    try {
      await relaunchApp(bundleId);
      toast.success(`Requested relaunch for ${appLabel}.`);
    } catch (error) {
      toast.error(`Failed to relaunch ${appLabel}: ${getErrorMessage(error)}`);
    } finally {
      setRelaunchingApp((current) => (current === bundleId ? null : current));
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
          refreshLocalChordFolders({ showErrorToast: true }),
          refreshActiveChords({ showErrorToast: true }),
          refreshGlobalShortcutMappings({ showErrorToast: true }),
          refreshAppsNeedingRelaunch({ showErrorToast: true }),
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

  const appMetadataBundleIds = [
    ...new Set([
      ...activeChords
        .filter((chord) => chord.scopeKind === "app")
        .map((chord) => chord.scope),
      ...globalShortcutMappings.map((mapping) => mapping.bundleId),
      ...appsNeedingRelaunch.map((app) => app.bundleId),
    ]),
  ].sort();
  const appMetadataByBundleId: AppMetadataByBundleId = useAppMetadata(appMetadataBundleIds);

  const normalizedChordSearch = chordSearch.trim().toLowerCase();
  const filteredActiveChords = normalizedChordSearch
    ? activeChords.filter((chord) =>
        [
          chord.scope,
          appMetadataByBundleId[chord.scope]?.displayName ?? "",
          chord.sequence,
          chord.name,
          chord.action,
        ].some((value) => value.toLowerCase().includes(normalizedChordSearch))
      )
    : activeChords;
  const chordGroups = buildChordGroups(filteredActiveChords);

  const normalizedGlobalShortcutSearch = globalShortcutSearch.trim().toLowerCase();
  const filteredGlobalShortcutMappings = normalizedGlobalShortcutSearch
    ? globalShortcutMappings.filter((mapping) =>
        [
          mapping.shortcut,
          mapping.bundleId,
          appMetadataByBundleId[mapping.bundleId]?.displayName ?? "",
          mapping.hotkeyId,
        ].some((value) => value.toLowerCase().includes(normalizedGlobalShortcutSearch))
      )
    : globalShortcutMappings;

  return {
    summary: {
      sourceCount: repos.length + localChordFolders.length,
      chordCount: activeChords.length,
      shortcutCount: globalShortcutMappings.length,
    },
    appMetadataByBundleId,
    settingsTab: {
      repos,
      reposBusy,
      repoInput,
      setRepoInput,
      addingRepo,
      syncingRepo,
      repoChordsByRepo,
      repoChordsBusy,
      openRepoChords,
      openRepoChordGroups,
      refreshRepos,
      handleAddRepo,
      handleOpenRepoUrl,
      handleRepoChordsToggle,
      handleSyncRepo,
      setRepoChordGroupOpen: (repoSlug: string, groupKey: string, open: boolean) => {
        setOpenRepoChordGroups((current) => ({
          ...current,
          [repoSlug]: {
            ...(current[repoSlug] ?? {}),
            [groupKey]: open,
          },
        }));
      },
      localChordFolders,
      localChordFoldersBusy,
      addingLocalChordFolder,
      localFolderChordsByPath,
      localFolderChordsBusy,
      openLocalFolderChords,
      openLocalFolderChordGroups,
      refreshLocalChordFolders,
      handleAddLocalChordFolder,
      handleLocalFolderChordsToggle,
      setLocalFolderChordGroupOpen: (folderPath: string, groupKey: string, open: boolean) => {
        setOpenLocalFolderChordGroups((current) => ({
          ...current,
          [folderPath]: {
            ...(current[folderPath] ?? {}),
            [groupKey]: open,
          },
        }));
      },
      appsNeedingRelaunch,
      appsNeedingRelaunchBusy,
      relaunchingApp,
      refreshAppsNeedingRelaunch,
      handleRelaunchApp,
      hasAccessibilityPermission,
      hasInputMonitoringPermission,
      accessibilityBusy,
      inputMonitoringBusy,
      handleAccessibilityButtonClick,
      handleInputMonitoringButtonClick,
      autostartEnabled,
      autostartBusy,
      autostartStatus,
      handleAutostartChange,
    },
    activeChordsTab: {
      activeChords,
      activeChordsBusy,
      chordSearch,
      setChordSearch,
      filteredActiveChords,
      chordGroups,
      normalizedChordSearch,
      openChordGroups,
      refreshActiveChords,
      setChordGroupOpen: (groupKey: string, open: boolean) => {
        setOpenChordGroups((current) => ({
          ...current,
          [groupKey]: open,
        }));
      },
    },
    globalShortcutsTab: {
      globalShortcutMappings,
      globalShortcutMappingsBusy,
      globalShortcutSearch,
      setGlobalShortcutSearch,
      filteredGlobalShortcutMappings,
      removingGlobalShortcut,
      refreshGlobalShortcutMappings,
      handleRemoveGlobalShortcutMapping,
    },
  };
}

export type SettingsPageData = ReturnType<typeof useSettingsPage>;
