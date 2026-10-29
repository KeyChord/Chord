import { expect, test } from 'bun:test';
import { blockDisplayEnds, getCommandLabel, getCommandPrefix, getRangeHint, isDeletionOf, normalizeChordTokens, withQuoteTexts } from '#chord-tokens';

const tokens = (input: string) => normalizeChordTokens([...input].map(c => (c === '⏎' ? 'Enter' : c)));
const prefixTokens = (input: string) => getCommandPrefix(tokens(input)).tokens;

test('typed escapes keep their keys until Tab replaces them natively', () => {
	expect(tokens('a\\up')).toEqual(['A', '\\', 'U', 'P']);
	expect(tokens('a\\n')).toEqual(['A', '\\', 'N']);
	expect(normalizeChordTokens(['KeyA', 'ArrowUp', 'Enter'])).toEqual(['A', '↑', '↩']);
});

test('completion prefixes preserve meta commands and numeric chord arguments', () => {
	expect(prefixTokens('')).toEqual([]);
	expect(prefixTokens('0h12')).toEqual([]);
	expect(prefixTokens('12ab34')).toEqual(['1', '2']);
	expect(prefixTokens('10')).toEqual(['1', '0']);
	expect(prefixTokens('ab12')).toEqual([]);
});

test('config blocks and Enter extend the prefix to the chord being typed', () => {
	expect(prefixTokens('3`d2,i1')).toEqual(['3']);
	expect(prefixTokens('3`i13`/w')).toEqual(['3', '`', 'I', '1', '3', '`']);
	expect(prefixTokens('3`d2,i13`/w]⏎/w')).toEqual(['3', '`', 'D', '2', ',', 'I', '1', '3', '`', '/', 'W', ']', '↩']);
	expect(prefixTokens('2ab⏎cd⏎')).toEqual(['2', 'A', 'B', '↩', 'C', 'D', '↩']);
	expect(prefixTokens('`i0`ab⏎cd')).toEqual(['`', 'I', '0', '`', 'A', 'B', '↩']);
	expect(prefixTokens('`d5`ab⏎cd')).toEqual(['`', 'D', '5', '`', 'A', 'B', '↩']);
	// Enter is an ordinary key without a count or config block, and a typed \n is not Enter.
	expect(prefixTokens('ab⏎cd')).toEqual([]);
	expect(prefixTokens('2ab\\ncd')).toEqual(['2']);

	const label = (input: string) => getCommandLabel(getCommandPrefix(tokens(input)));
	expect(label('ab')).toBe('');
	expect(label('12ab')).toBe('Repeat 12 times');
	expect(label('1ab')).toBe('Loop');
	expect(label('3`d2,i13`/w]⏎/w')).toBe('Repeat 3 times · `d2,i13` · Step 2');
	expect(label('`d5`/w')).toBe('`d5`');
});

test('Enter inside a quote is typed rather than separating steps', () => {
	expect(prefixTokens("2'a⏎b")).toEqual(['2']);
	expect(prefixTokens("2'a⏎b'⏎/w")).toEqual(['2', "'", 'A', '↩', 'B', "'", '↩']);
	expect(prefixTokens("2'a'`i5`⏎/w")).toEqual(['2', "'", 'A', "'", '`', 'I', '5', '`', '↩']);
	expect(prefixTokens("2'a'`i5⏎")).toEqual(['2']);
	expect(prefixTokens("'a⏎b'⏎/w")).toEqual([]);
});

test('repeat-count ranges only continue an unfinished count', () => {
	const hint = { prefix: [], keys: [...'123456789'], description: 'Repeat count', isRepeatCount: true };
	const range = (input: string) => getRangeHint(hint, getCommandPrefix(tokens(input)));
	expect(range('')!.keys).not.toContain('1');
	expect(range('3')!.keys).toContain('0');
	expect(range('3`i5`')).toBeNull();
	expect(range('3ab⏎')).toBeNull();
});

test('typed quotes show their insert-mode text, including the open one', () => {
	expect(withQuoteTexts(tokens("3''"), ['Hi there'], false)).toEqual(['3', "'", 'Hi there', "'"]);
	expect(withQuoteTexts(tokens("2''⏎/w]⏎'"), ['A', 'B'], true)).toEqual(['2', "'", 'A', "'", '↩', '/', 'W', ']', '↩', "'", 'B']);
	expect(withQuoteTexts(tokens("3'"), [''], true)).toEqual(['3', "'", '']);
	// Text notation keeps its keys, and a `'` inside a chord is an ordinary key.
	expect(withQuoteTexts(tokens("'hi'"), [], false)).toEqual(["'", 'H', 'I', "'"]);
	expect(withQuoteTexts(tokens("ab'"), [], false)).toEqual(['A', 'B', "'"]);
});

test('block ends follow the quote texts shown before them', () => {
	expect(blockDisplayEnds(tokens('gde'), [], [1, 3])).toEqual([1, 3]);
	// The block `''a` shows its quote's text as one more token.
	expect(blockDisplayEnds(tokens("''a⏎''"), ['Hi', 'There'], [3])).toEqual([4]);
	expect(blockDisplayEnds(tokens('g'), [], [])).toEqual([]);
});

test('Backspace deletes keys and quote text, while typing and expansions do not', () => {
	expect(isDeletionOf(tokens('/w'), tokens('/w]'))).toBe(true);
	expect(isDeletionOf([], tokens('a'))).toBe(true);
	expect(isDeletionOf(["'", 'Hi'], ["'", 'Hi!'])).toBe(true);
	expect(isDeletionOf(tokens('/w]'), tokens('/w]'))).toBe(false);
	expect(isDeletionOf(tokens('/w]'), tokens('/w'))).toBe(false);
	expect(isDeletionOf(["'", 'Hi!'], ["'", 'Hi'])).toBe(false);
	// Tab and typed references replace the escape instead of shortening the buffer.
	expect(isDeletionOf(tokens('a/w]'), tokens('a\\1'))).toBe(false);
	expect(isDeletionOf(tokens('a↑'), tokens('a\\up'))).toBe(false);
});
