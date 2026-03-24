import { useChordFilesState } from "./state.ts";
import mapObject from 'map-obj'

type RawChord = {
  name: string;
  shell?: string
  js?: any
  args: any
}

interface RawChordFile {
  config?: { name: string, extends: string };
  chords?: Record<string, RawChord | Array<RawChord>>;
}

// The reason why we do one chord file at a time is because resolving is expensive
export function useChordFile(bundleId: string | undefined): Record<string, RawChord> {
  const { rawFilesAsJsonStrings } = useChordFilesState()
  const rawFilesAsJson = mapObject(rawFilesAsJsonStrings, (key, value) => [key, JSON.parse(value)])
  if (bundleId === undefined) {
    return getGlobalChords(rawFilesAsJson);
  }

  return resolveChords(rawFilesAsJson, bundleId)
}

function getGlobalChords(rawFilesAsJson: Record<string, RawChordFile>) {
  const chords: Record<string, RawChord> = {}
  for (const file of Object.values(rawFilesAsJson)) {
    for (const [sequence, chord] of Object.entries(file.chords ?? {})) {
      if (sequence[0].toUpperCase() === sequence[0].toUpperCase()) {
        chords[sequence] = Array.isArray(chord) ? chord[0] : chord;
      }
    }
  }

  return chords;
}

function resolveChords(rawFilesAsJsonStrings: Record<string, RawChordFile>, bundleId: string): Record<string, RawChord> {
  const file = rawFilesAsJsonStrings[bundleId] ?? {};
  const chords: Record<string, RawChord> = mapObject(file.chords ?? {}, (key, value) => {
    if (Array.isArray(value)) {
      return [key, value[0]];
    } else {
      return [key, value];
    }
  });

  if (file.config?.extends) {
    let extendedChords = resolveChords(rawFilesAsJsonStrings, file.config.extends)
    for (const [sequence, value] of Object.entries(extendedChords)) {
      chords[sequence] = value
    }
  }

  return chords;
}