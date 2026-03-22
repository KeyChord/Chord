import { createTauRPCProxy as createProxy, type InferCommandOutput } from "taurpc";

export type GitRepoInfo = {
  owner: string;
  name: string;
  slug: string;
  url: string;
  localPath: string;
  headShortSha: string | null;
};

export type LocalChordFolderInfo = {
  name: string;
  localPath: string;
};

export type ActiveChordInfo = {
  scope: string;
  scopeKind: "global" | "app";
  sequence: string;
  name: string;
  action: string;
};

export type GlobalShortcutMappingInfo = {
  shortcut: string;
  bundleId: string;
  hotkeyId: string;
};

export type AppNeedsRelaunchInfo = {
  bundleId: string;
  displayName: string | null;
};

const ARGS_MAP = {
  "": JSON.stringify({
    open_accessibility_settings: [],
    open_input_monitoring_settings: [],
    list_git_repos: [],
    add_git_repo: ["repo"],
    sync_git_repo: ["repo"],
    list_local_chord_folders: [],
    pick_local_chord_folder: [],
    add_local_chord_folder: ["path"],
    list_active_chords: [],
    list_repo_chords: ["repo"],
    list_local_chord_folder_chords: ["path"],
    list_global_shortcut_mappings: [],
    remove_global_shortcut_mapping: ["shortcut"],
    list_apps_needing_relaunch: [],
    relaunch_app: ["bundle_id"],
  }),
};

export type Router = {
  "": {
    open_accessibility_settings: () => Promise<void>;
    open_input_monitoring_settings: () => Promise<void>;
    list_git_repos: () => Promise<GitRepoInfo[]>;
    add_git_repo: (repo: string) => Promise<GitRepoInfo>;
    sync_git_repo: (repo: string) => Promise<GitRepoInfo>;
    list_local_chord_folders: () => Promise<LocalChordFolderInfo[]>;
    pick_local_chord_folder: () => Promise<string | null>;
    add_local_chord_folder: (path: string) => Promise<LocalChordFolderInfo>;
    list_active_chords: () => Promise<ActiveChordInfo[]>;
    list_repo_chords: (repo: string) => Promise<ActiveChordInfo[]>;
    list_local_chord_folder_chords: (path: string) => Promise<ActiveChordInfo[]>;
    list_global_shortcut_mappings: () => Promise<GlobalShortcutMappingInfo[]>;
    remove_global_shortcut_mapping: (shortcut: string) => Promise<void>;
    list_apps_needing_relaunch: () => Promise<AppNeedsRelaunchInfo[]>;
    relaunch_app: (bundleId: string) => Promise<void>;
  };
};

export const createTauRPCProxy = () => createProxy<Router>(ARGS_MAP);
export type { InferCommandOutput };
