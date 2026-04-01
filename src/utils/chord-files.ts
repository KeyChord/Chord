import type { ChordPackage } from '#/types/generated.ts';

interface RawChord {
	index: number
	name: string
	shell?: string
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
