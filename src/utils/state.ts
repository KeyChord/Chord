import { useEffect, useState } from "react";
import type { ChorderState } from "../types/generated.ts";
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

  return [state]
}