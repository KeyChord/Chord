import { expect, test } from 'bun:test';
import type { ChordReference } from '@chord/dev.improve.chord.lib.typeshare';
import {
	chordSequence,
	findBlockedChordsFolder,
	findChordConflict,
	sanitizeChordKeys,
	suggestChordKeys,
} from '#first-chord';

const existing = (name: string, keys: string[], actions: unknown[] = [{ type: 'shell' }]) => ({
	packageName: 'some-package',
	chordsFilePathslug: 'chords/macos.toml',
	chord: { rawTrigger: '', trigger: { keys }, name, index: 0, actions },
}) as unknown as ChordReference;
const slash = (letter: string) => existing(`Chord ${letter}`, ['Slash', `Key${letter.toUpperCase()}`]);

test('suggests the first letter of the app', () => {
	expect(suggestChordKeys('Safari')).toBe('s');
	expect(suggestChordKeys('App Store')).toBe('a');
});

test('skips letters already used by other chords', () => {
	expect(suggestChordKeys('Safari', [slash('s')])).toBe('a');
	expect(suggestChordKeys('Safari', [slash('s'), slash('a')])).toBe('f');
});

test('skips characters a chord cannot use', () => {
	expect(suggestChordKeys('1Password')).toBe('1');
	expect(suggestChordKeys('.NET Tools')).toBe('n');
});

test('falls back to the first letter when every letter is taken', () => {
	expect(suggestChordKeys('Go', [slash('g'), slash('o')])).toBe('g');
});

test('keeps only keys a chord can use, and only a few of them', () => {
	expect(sanitizeChordKeys('S/a F!')).toBe('saf');
	expect(sanitizeChordKeys('abcdef')).toBe('abc');
});

test('builds a chord that works in every app', () => {
	expect(chordSequence('s')).toBe('/s');
});

test('finds the chord already using the same keys', () => {
	const terminal = existing('Terminal', ['Slash', 'KeyT']);
	expect(findChordConflict('/t', [slash('s'), terminal])).toBe(terminal);
	expect(findChordConflict('/x', [slash('s'), terminal])).toBeUndefined();
});

test('a longer chord that starts the same way is not a conflict', () => {
	expect(findChordConflict('/t', [existing('Tabs', ['Slash', 'KeyT', 'KeyA'])])).toBeUndefined();
});

test('a chord that runs nothing leaves its keys free', () => {
	expect(findChordConflict('/t', [existing('Heading', ['Slash', 'KeyT'], [])])).toBeUndefined();
});

test('a pattern is left for the backend to resolve', () => {
	const pattern = { chord: { trigger: { pattern: '\\A/.\\z' }, actions: [{}] } } as unknown as ChordReference;
	expect(findChordConflict('/t', [pattern])).toBeUndefined();
});

const defaultFolder = { name: 'default', path: '/Users/me/.chord/packages/default', identity: 'id', approved: false };
const personalPackages = [{ dirname: 'default', name: 'default' }];

test('notices when the folder for new chords is not allowed to load', () => {
	expect(findBlockedChordsFolder({ personalPackages, thirdPartyPackages: [defaultFolder] })).toBe(defaultFolder);
});

test('a folder that is already allowed needs nothing', () => {
	expect(findBlockedChordsFolder({
		personalPackages,
		thirdPartyPackages: [{ ...defaultFolder, approved: true }],
	})).toBeUndefined();
});

test('other blocked packages do not hold up the first chord', () => {
	expect(findBlockedChordsFolder({
		personalPackages,
		thirdPartyPackages: [{ ...defaultFolder, name: 'downloaded', path: '/Users/me/.chord/packages/downloaded' }],
	})).toBeUndefined();
});
