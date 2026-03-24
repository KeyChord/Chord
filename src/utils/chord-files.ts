import mapObject from "map-obj";
import { useChordFilesState } from "./state.ts";
import type { PlaceholderChordInfo } from "#/types/generated.ts";

type RawChord = {
  name: string;
  shell?: string;
  js?: any;
  args?: any;
};

interface RawChordFile {
  config?: { name?: string; extends?: string };
  chords?: Record<string, RawChord | Array<RawChord>>;
}

function materializePlaceholderChords(
  file: RawChordFile,
  placeholders: PlaceholderChordInfo[],
): RawChordFile {
  const nextChords: Record<string, RawChord | Array<RawChord>> = { ...(file.chords ?? {}) };

  for (const placeholder of placeholders) {
    const chord = nextChords[placeholder.sequenceTemplate];
    delete nextChords[placeholder.sequenceTemplate];

    if (!chord || !placeholder.assignedSequence) {
      continue;
    }

    const resolvedSequence = `${placeholder.sequencePrefix}${placeholder.assignedSequence}${placeholder.sequenceSuffix}`;
    nextChords[resolvedSequence] = chord;
  }

  return {
    ...file,
    chords: nextChords,
  };
}

function resolveRawFiles(
  rawFilesAsJsonStrings: Record<string, string>,
  placeholderChords: PlaceholderChordInfo[],
) {
  const placeholdersByFile = placeholderChords.reduce<Record<string, PlaceholderChordInfo[]>>(
    (result, placeholder) => {
      result[placeholder.filePath] ??= [];
      result[placeholder.filePath].push(placeholder);
      return result;
    },
    {},
  );

  return mapObject(rawFilesAsJsonStrings, (filePath, value) => {
    const file = JSON.parse(value) as RawChordFile;
    return [filePath, materializePlaceholderChords(file, placeholdersByFile[filePath] ?? [])];
  });
}

// The reason why we do one chord file at a time is because resolving is expensive
export function useChordFile(bundleId: string | undefined): Record<string, RawChord> {
  const { rawFilesAsJsonStrings, placeholderChords } = useChordFilesState();
  const rawFilesAsJson = resolveRawFiles(rawFilesAsJsonStrings, placeholderChords);
  const chords = getGlobalChords(rawFilesAsJson);

  if (bundleId !== undefined) {
    for (const [sequence, chord] of Object.entries(resolveChords(rawFilesAsJson, bundleId))) {
      chords[sequence] = chord;
    }
  }

  return chords;
}

function getGlobalChords(rawFilesAsJson: Record<string, RawChordFile>) {
  const chords: Record<string, RawChord> = {};

  for (const file of Object.values(rawFilesAsJson)) {
    for (const [sequence, chord] of Object.entries(file.chords ?? {})) {
      if (sequence[0]?.toUpperCase() === sequence[0]) {
        chords[sequence] = Array.isArray(chord) ? chord[0] : chord;
      }
    }
  }

  return chords;
}

const bundleIdToFilepath = (bundleId: string) =>
  `chords/${bundleId.replaceAll(".", "/")}/macos.toml`;

function resolveChords(
  rawFilesAsJson: Record<string, RawChordFile>,
  bundleId: string,
): Record<string, RawChord> {
  const filepath = bundleIdToFilepath(bundleId);
  const file = rawFilesAsJson[filepath] ?? {};
  const chords: Record<string, RawChord> = mapObject(file.chords ?? {}, (key, value) => {
    if (Array.isArray(value)) {
      return [key, value[0]];
    }

    return [key, value];
  });

  if (file.config?.extends) {
    const extendedChords = resolveChords(rawFilesAsJson, file.config.extends);
    for (const [sequence, value] of Object.entries(extendedChords)) {
      chords[sequence] = value;
    }
  }

  return chords;
}
