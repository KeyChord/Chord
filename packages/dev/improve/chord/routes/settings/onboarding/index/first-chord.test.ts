import { expect, test } from 'bun:test';
import { chordSequence, sanitizeChordKeys, suggestChordKeys } from '#first-chord';

test('suggests the first letter of the app', () => {
	expect(suggestChordKeys('Safari')).toBe('s');
	expect(suggestChordKeys('App Store')).toBe('a');
});

test('skips letters already used by other chords', () => {
	expect(suggestChordKeys('Safari', ['/s'])).toBe('a');
	expect(suggestChordKeys('Safari', ['/s', '/a'])).toBe('f');
});

test('skips characters a chord cannot use', () => {
	expect(suggestChordKeys('1Password')).toBe('1');
	expect(suggestChordKeys('.NET Tools')).toBe('n');
});

test('falls back to the first letter when every letter is taken', () => {
	expect(suggestChordKeys('Go', ['/g', '/o'])).toBe('g');
});

test('keeps only keys a chord can use, and only a few of them', () => {
	expect(sanitizeChordKeys('S/a F!')).toBe('saf');
	expect(sanitizeChordKeys('abcdef')).toBe('abc');
});

test('builds a chord that works in every app', () => {
	expect(chordSequence('s')).toBe('/s');
});
