import type { PlaceholderChordInfo } from '#/types/generated.ts';
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
	// const { rawFilesAsJsonStrings, placeholderChords } = useChordFilesState();
  const { packages } = useChordPackageManagerState();

	const rawFilesAsJson = resolveRawFiles({}, []);
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

	for (const [filePath, file] of sortedChordFileEntries(rawFilesAsJson)) {
		if (bundleIdFromFilePath(filePath) !== GLOBAL_RUNTIME_ID) {
			continue;
		}

		for (const [sequence, chord] of Object.entries(file.chords ?? {})) {
			if (sequence[0]?.toUpperCase() === sequence[0]) {
				chords[sequence] = Array.isArray(chord) ? chord[0] : chord;
			}
		}
	}

	return chords;
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

function sortedChordFileEntries(rawFilesAsJson: Record<string, RawChordFile>) {
	return Object.entries(rawFilesAsJson).sort(([leftPath], [rightPath]) => {
		const leftIsBase = leftPath.endsWith('/macos.toml') || leftPath === 'chords/macos.toml';
		const rightIsBase = rightPath.endsWith('/macos.toml') || rightPath === 'chords/macos.toml';
		return Number(rightIsBase) - Number(leftIsBase) || leftPath.localeCompare(rightPath);
	});
}

function bundleIdFromFilePath(filePath: string) {
	return runtimeInfoFromFilePath(filePath)?.bundleId;
}

function resolveChordsForFile(
	rawFilesAsJson: Record<string, RawChordFile>,
	filePath: string,
): Record<string, RawChord> {
	const file = rawFilesAsJson[filePath] ?? {};
	return mapObject(file.chords ?? {}, (key, value) => {
		if (Array.isArray(value)) {
			return [key, value[0]];
		}

		return [key, value];
	});
}

function resolveChords(
	rawFilesAsJson: Record<string, RawChordFile>,
	bundleId: string,
): Record<string, RawChord> {
	const chords: Record<string, RawChord> = {};
	const filePaths = sortedChordFileEntries(rawFilesAsJson)
		.map(([filePath]) => filePath)
		.filter(filePath => bundleIdFromFilePath(filePath) === bundleId);

	for (const filePath of filePaths) {
		for (const [sequence, chord] of Object.entries(resolveChordsForFile(rawFilesAsJson, filePath))) {
			chords[sequence] = chord;
		}
	}

	return chords;
}
