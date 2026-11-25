import type {
  AppSettingsState,
  ChordExecutionState,
  ChordInputState,
  ChordMacrosState,
  ChordPanelState,
  FrontmostState,
  HintPanelState,
} from "@chord/dev.improve.chord.lib.typeshare";
import { useQuery, useQueryClient } from "@chord/com.npmjs.tanstack__react-query";
import { taurpc } from "@chord/dev.improve.chord.api.taurpc";
import { listen } from "@tauri-apps/api/event";
import { createStateQuery } from "#state-query";

/** The states the Hint Panel draws from. It reads and follows only these. */
export interface HintPanelStates {
  "chord-input": ChordInputState;
  "chord-panel": ChordPanelState;
  frontmost: FrontmostState;
  "hint-panel": HintPanelState;
  "chord-execution": ChordExecutionState;
  "chord-macros": ChordMacrosState;
  settings: AppSettingsState;
}

const stateIds = [
  "chord-input",
  "chord-panel",
  "frontmost",
  "hint-panel",
  "chord-execution",
  "chord-macros",
  "settings",
] as const satisfies readonly (keyof HintPanelStates)[];

const stateQuery = createStateQuery<HintPanelStates>({
  key: "hint-panel-states",
  stateIds,
  read: async () => JSON.parse(await taurpc.getStates([...stateIds])) as HintPanelStates,
  subscribe: (id, onChange) => listen(`state:${id}`, (event) => onChange(event.payload)),
});

export function useHintPanelState<K extends keyof HintPanelStates>(stateId: K) {
  const client = useQueryClient();
  return useQuery({
    ...stateQuery(client),
    select: (states: HintPanelStates) => states[stateId],
  });
}
