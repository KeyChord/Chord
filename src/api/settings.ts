import {
  createTauRPCProxy,
  type ActiveChordInfo,
  type AppNeedsRelaunchInfo,
  type GitRepoInfo,
  type GlobalShortcutMappingInfo,
  type LocalChordFolderInfo,
} from "#/api/bindings.ts";

const taurpc = createTauRPCProxy();

export type {
  ActiveChordInfo,
  AppNeedsRelaunchInfo,
  GitRepoInfo,
  GlobalShortcutMappingInfo,
  LocalChordFolderInfo,
};

export function listGitRepos() {
  return taurpc.list_git_repos();
}

export function listLocalChordFolders() {
  return taurpc.list_local_chord_folders();
}

export function listActiveChords() {
  return taurpc.list_active_chords();
}

export function listGlobalShortcutMappings() {
  return taurpc.list_global_shortcut_mappings();
}

export function listAppsNeedingRelaunch() {
  return taurpc.list_apps_needing_relaunch();
}

export function listRepoChords(repo: string) {
  return taurpc.list_repo_chords(repo);
}

export function listLocalChordFolderChords(path: string) {
  return taurpc.list_local_chord_folder_chords(path);
}

export function openAccessibilitySettings() {
  return taurpc.open_accessibility_settings();
}

export function openInputMonitoringSettings() {
  return taurpc.open_input_monitoring_settings();
}

export function addGitRepo(repo: string) {
  return taurpc.add_git_repo(repo);
}

export function pickLocalChordFolder() {
  return taurpc.pick_local_chord_folder();
}

export function addLocalChordFolder(path: string) {
  return taurpc.add_local_chord_folder(path);
}

export function syncGitRepo(repo: string) {
  return taurpc.sync_git_repo(repo);
}

export function removeGlobalShortcutMapping(shortcut: string) {
  return taurpc.remove_global_shortcut_mapping(shortcut);
}

export function relaunchApp(bundleId: string) {
  return taurpc.relaunch_app(bundleId);
}
