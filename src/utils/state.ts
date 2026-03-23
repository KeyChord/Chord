import { useEffect, useState } from "react";
import type { AppPermissionsState, AppSettingsState, ChorderState } from "../types/generated.ts";
import { listen } from "@tauri-apps/api/event";
import renameFunction from "rename-fn";

function createUseTauriState<T>(stateName: string) {
  const useTauriState = () => {
    const [state, setState] = useState<T>((window as any).__INITIAL_STATE__ as T);

    useEffect(() => {
      const unlistenPromise = listen<T>(`state:${stateName}`, (event) => {
        setState(event.payload);
      });

      return () => {
        void unlistenPromise.then((unlisten) => unlisten?.());
      };
    }, []);

    return state;
  };

  return renameFunction(useTauriState, stateName);
}

export const useChorderState = createUseTauriState<ChorderState>("chorder");
export const useSettingsState = createUseTauriState<AppSettingsState>("settings");
export const usePermissionsState = createUseTauriState<AppPermissionsState>("permissions");
