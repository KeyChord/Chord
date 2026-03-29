import type { ChordPackage, PlaceholderChordInfo } from '#/types/generated.ts';
import { useChordPackageManagerState } from './state.ts';

interface RawChord {
	index: number
	name: string
	shell?: string
}

export function useChordFile(bundleId: string | undefined): Record<string, RawChord> {
  const { packages } = useChordPackageManagerState();
	const chords = getGlobalChords(packages);

  if (bundleId !== undefined) {
    for (const pkg of packages) {
      const chordsFile = pkg.appChordsFiles[bundleId];
      if (chordsFile) {
        Object.assign(chords, chordsFile.chords)
      }
    }
  }

	return chords;
}

function getGlobalChords(packages: ChordPackage[]) {
	const globalChords: Record<string, RawChord> = {};
  for (const pkg of packages) {
    Object.assign(globalChords, pkg.globalChords);
  }

	return globalChords;
}

function supportedChordFileName(fileName: string) {
	return fileName === 'macos.toml' || fileName.endsWith('.macos.toml');
}
