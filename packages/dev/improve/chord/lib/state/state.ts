import type {
  AppModeState,
  AppPermissionsState,
  AppSettingsState,
  ChordInputState,
  ChordExecutionState,
  ChordMacrosState,
  ChordPackageManagerState,
  ChordPackageStoreState,
  ChordPanelState,
  DesktopAppManagerState,
  FrontmostState,
  GitReposState,
  KeyboardState,
} from "@chord/dev.improve.chord.lib.typeshare";
import { useQuery, useQueryClient } from "@chord/com.npmjs.tanstack__react-query";
import { taurpc } from "@chord/dev.improve.chord.api.taurpc";
import { listen } from "@tauri-apps/api/event";
import { createStateQuery } from "#state-query";

export interface TauriStates {
  "app-mode": AppModeState;
  keyboard: KeyboardState;
  "chord-panel": ChordPanelState;
  "chord-input": ChordInputState;
  "chord-execution": ChordExecutionState;
  "chord-macros": ChordMacrosState;
  settings: AppSettingsState;
  permissions: AppPermissionsState;
  "git-repos": GitReposState;
  frontmost: FrontmostState;
  "chord-package-manager": ChordPackageManagerState;
  "desktop-app-manager": DesktopAppManagerState;
  "chord-package-store": ChordPackageStoreState;
}

const stateIds = [
  "app-mode",
  "keyboard",
  "chord-panel",
  "chord-input",
  "chord-execution",
  "chord-macros",
  "settings",
  "permissions",
  "git-repos",
  "frontmost",
  "chord-package-manager",
  "desktop-app-manager",
  "chord-package-store",
] as const;

const stateQuery = createStateQuery<TauriStates>({
  stateIds,
  read: async () => JSON.parse(await taurpc.getCurrentStates()) as TauriStates,
  subscribe: (id, onChange) => listen(`state:${id}`, (event) => onChange(event.payload)),
});

function createUseTauriState<K extends keyof TauriStates>(stateId: K) {
  const select = (states: TauriStates) => states[stateId];
  return function useTauriState() {
    const client = useQueryClient();
    return useQuery({ ...stateQuery(client), select });
  };
}

export const useKeyboardState = createUseTauriState("keyboard");
export const useChordPanelState = createUseTauriState("chord-panel");
export const useChordInputState = createUseTauriState("chord-input");
export const useChordExecutionState = createUseTauriState("chord-execution");
export const useChordMacrosState = createUseTauriState("chord-macros");
export const useSettingsState = createUseTauriState("settings");
export const usePermissionsState = createUseTauriState("permissions");
export const useGitRepoStoreState = createUseTauriState("git-repos");
export const useFrontmostState = createUseTauriState("frontmost");
export const useChordPackageManagerState = createUseTauriState("chord-package-manager");
export const useDesktopAppManagerState = createUseTauriState("desktop-app-manager");
export const useChordPackageStoreState = createUseTauriState("chord-package-store");
