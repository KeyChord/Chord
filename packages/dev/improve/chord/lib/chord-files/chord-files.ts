import type { ChordPackage } from '@chord/dev.improve.chord.lib.typeshare';

interface RawChord {
	index: number
	name: string
	shell?: string
}

export function getGlobalChords(packages: ChordPackage[]) {
	const globalChords: Record<string, RawChord> = {};
	for (const pkg of packages) {
		Object.assign(globalChords, pkg.globalChords);
	}

	return globalChords;
}

export function supportedChordFileName(fileName: string) {
	return fileName === 'macos.toml' || fileName.endsWith('.macos.toml');
}
