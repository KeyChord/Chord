import { useEffect, useState } from "react";
import type { AppPermissionsState } from "../types/generated.ts";
import { listen } from "@tauri-apps/api/event";

export function useAppPermissionsState() {
  const [state, setState] = useState<AppPermissionsState>({
    isAccessibilityEnabled: false,
    isInputMonitoringEnabled: false,
    isAutostartEnabled: false,
  });

  useEffect(() => {
    const unlistenPromise = listen<AppPermissionsState>(
      "app-permissions-state-changed",
      (event) => {
        setState(event.payload);
      },
    );

    return () => {
      void unlistenPromise.then((unlisten) => unlisten?.());
    };
  }, []);

  return [state];
}
