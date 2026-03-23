import { useEffect, useState } from "react";
import type { AppSettingsState, ChorderState } from "../types/generated.ts";
import { listen } from "@tauri-apps/api/event";

export function useChorderState() {
  const [state, setState] = useState<ChorderState>({
    keyBuffer: [],
    activeChord: undefined,
    pressedChord: undefined
  });

  useEffect(() => {
    const unlistenPromise = listen<ChorderState>(
      "chorder-state-changed",
      (event) => {
        setState(event.payload)
      },
    );

    return () => {
      void unlistenPromise.then((unlisten) => unlisten?.());
    };
  }, []);

  return state
}

export function useAppSettingsState() {
  const [state, setState] = useState<AppSettingsState>({
    bundleIdsNeedingRelaunch: [],
    gitRepos: [],
    permissions: {
      isAccessibilityEnabled: false,
      isInputMonitoringEnabled: false,
      isAutostartEnabled: false
    }
  });

  useEffect(() => {
    const unlistenPromise = listen<AppSettingsState>(
      "app-settings-state-changed",
      (event) => {
        setState(event.payload)
      },
    );

    return () => {
      void unlistenPromise.then((unlisten) => unlisten?.());
    };
  }, []);

  return state
}