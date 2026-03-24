import { useEffect, useState } from "react";
import type { AppPermissionsState, AppSettingsState, ChorderState, ChordFilesState, FrontmostState } from "../types/generated.ts";
import { listen } from "@tauri-apps/api/event";
import renameFunction from "rename-fn";
import { taurpc } from "../api/taurpc.ts";

async function createUseTauriState<T>(stateId: string) {
  const initialStates = JSON.parse(await taurpc.getCurrentStates());
  const useTauriState = () => {
    const [state, setState] = useState<T>(initialStates[stateId]);
    useEffect(() => {
      const unlistenPromise = listen<T>(`state:${stateId}`, (event) => {
        console.log(event.payload)
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

export const useChorderState = await createUseTauriState<ChorderState>("chorder");
export const useSettingsState = await createUseTauriState<AppSettingsState>("settings");
export const usePermissionsState = await createUseTauriState<AppPermissionsState>("permissions");
export const useGitRepoStoreState = await createUseTauriState<any>("git-repos");
export const useChordFilesState = await createUseTauriState<ChordFilesState>("chord-files");
export const useFrontmostState = await createUseTauriState<FrontmostState>("frontmost");
