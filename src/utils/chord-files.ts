import type { ChordPackage, PlaceholderChordInfo } from '#/types/generated.ts';
import mapObject from 'map-obj';
import { useChordPackageManagerState } from './state.ts';

interface RawChord {
	index: number
	name: string
	shell?: string
}

interface RawChordFile {
	name?: string
	meta?: Record<string, unknown>
	js?: Record<string, string>
	chords?: Record<string, RawChord | Array<RawChord>>
}

const GLOBAL_RUNTIME_ID = '__global__';

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

// The reason why we do one chord file at a time is because resolving is expensive
export function useChordFile(bundleId: string | undefined): Record<string, RawChord> {
	// const { rawFilesAsJsonStrings, placeholderChords } = useChordFilesState();
  const { packages } = useChordPackageManagerState();
	const chords = getGlobalChords(packages);

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

function runtimeInfoFromFilePath(filePath: string) {
	if (!filePath.startsWith('chords/')) {
		return undefined;
	}

	const parts = filePath.split('/');
	const fileName = parts.at(-1);
	if (!fileName || !supportedChordFileName(fileName)) {
		return undefined;
	}

	const bundlePath = parts.slice(1, -1).join('/');
	const bundleId = bundlePath === '' ? GLOBAL_RUNTIME_ID : bundlePath.replaceAll('/', '.');
	const runtimeId = fileName === 'macos.toml'
		? bundleId
		: `${bundleId}#${fileName.slice(0, -'.macos.toml'.length)}`;

	return { bundleId, runtimeId };
}
