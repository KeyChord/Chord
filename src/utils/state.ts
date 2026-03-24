import { useEffect, useState } from "react";
import type { AppPermissionsState, AppSettingsState, ChorderState, ChordRegistryState, FrontmostState } from "../types/generated.ts";
import { listen } from "@tauri-apps/api/event";
import renameFunction from "rename-fn";

function createUseTauriState<T>(stateId: string) {
  const useTauriState = () => {
    const [state, setState] = useState<T>((window as any).__INITIAL_STATES__[stateId] as T);

    useEffect(() => {
      const unlistenPromise = listen<T>(`state:${stateId}`, (event) => {
        setState(event.payload);
      });

      return () => {
        void unlistenPromise.then((unlisten) => unlisten?.());
      };
    }, []);

    return state;
  };

  return renameFunction(useTauriState, stateId);
}

export const useChorderState = createUseTauriState<ChorderState>("chorder");
export const useSettingsState = createUseTauriState<AppSettingsState>("settings");
export const usePermissionsState = createUseTauriState<AppPermissionsState>("permissions");
export const useGitRepoStoreState = createUseTauriState<any>("git-repos");
export const useChordRegistryState = createUseTauriState<ChordRegistryState>("chord-registry");
export const useFrontmostState = createUseTauriState<FrontmostState>("frontmost");
