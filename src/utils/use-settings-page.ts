import { useEffect, useState, type FormEvent } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { toast } from "sonner";
import {
  type ActiveChordInfo,
  type GlobalShortcutMappingInfo,
  type LocalChordFolderInfo,
  taurpc,
} from "#/api/taurpc.ts";
import {
  buildChordGroups,
  getErrorMessage,
  validateLocalChordFolder,
} from "#/utils/settings.ts";
import { useMutation } from '@tanstack/react-query'

type RefreshOptions = {
  showSuccessToast?: boolean;
  showErrorToast?: boolean;
};

export function useSettingsPage() {
  const currentWindow = getCurrentWindow();
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

  async function refreshLocalChordFolders(options?: RefreshOptions) {
    const { showSuccessToast = false, showErrorToast = true } = options ?? {};
    setLocalChordFoldersBusy(true);

    try {
      const nextFolders = await taurpc.listLocalChordFolders();
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
      const nextChords = await taurpc.listActiveChords();
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
      const nextMappings = await taurpc.listGlobalShortcutMappings();
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

  async function refreshRepoChords(repoSlug: string, options?: RefreshOptions) {
    const { showSuccessToast = false, showErrorToast = true } = options ?? {};
    setRepoChordsBusy((current) => ({ ...current, [repoSlug]: true }));

    try {
      const nextChords = await taurpc.listRepoChords(repoSlug);
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
      const nextChords = await taurpc.listLocalChordFolderChords(folderPath);
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

  const openAccessibilitySettingsMutation = useMutation({
    mutationFn: taurpc.openAccessibilitySettings,
  })
  const openInputMonitoringSettingsMutation = useMutation({
    mutationFn: taurpc.openInputMonitoringSettings,
  })


  async function handleAddLocalChordFolder() {
    let selectedPath: string | null = null;

    try {
      selectedPath = await taurpc.pickLocalChordFolder();
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
      const addedFolder = await taurpc.addLocalChordFolder(selectedPath);
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

  return {
    settingsTab: {
      repoChordsByRepo,
      repoChordsBusy,
      openRepoChords,
      openRepoChordGroups,
      handleRepoChordsToggle,
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
      relaunchingApp,
    },
    activeChordsTab: {
      activeChords,
      activeChordsBusy,
      chordSearch,
      setChordSearch,
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
      removingGlobalShortcut,
      refreshGlobalShortcutMappings,
      handleRemoveGlobalShortcutMapping,
    },
  };
}

export type SettingsPageData = ReturnType<typeof useSettingsPage>;
