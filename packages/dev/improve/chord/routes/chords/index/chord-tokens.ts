import getPrettyKey from 'pretty-key';

export function normalizeToken(token: string) {
	const pretty = getPrettyKey(token);
	const normalized = pretty === '_' ? '-' : pretty;
	return normalized.length === 1 ? normalized.toUpperCase() : normalized;
}

/** Typed escapes stay as typed; only Tab replaces them, natively. */
export function normalizeChordTokens(keys: string[]) {
	return keys.map(normalizeToken);
}

const ENTER = normalizeToken('Enter');

/**
 * Display tokens with insert-mode text: each typed quote is an adjacent `''` pair
 * in the keys, and while inserting the final `'` is still open.
 */
export function withQuoteTexts(tokens: string[], quotes: string[], inserting: boolean) {
	const result: string[] = [];
	let quote = 0;
	for (let index = 0; index < tokens.length; index++) {
		const token = tokens[index]!;
		result.push(token);
		if (token !== "'" || quote >= quotes.length) continue;
		if (tokens[index + 1] === "'") {
			result.push(quotes[quote++]!, "'");
			index++;
		} else if (inserting && index === tokens.length - 1) {
			result.push(quotes[quote++]!);
		}
	}
	return result;
}

/** Where each block of `tokens` ends among the display tokens of `withQuoteTexts`. */
export function blockDisplayEnds(tokens: string[], quotes: string[], blocks: number[]) {
	return blocks.map(end => withQuoteTexts(tokens.slice(0, end), quotes, false).length);
}

/** Whether `next` only drops keys or quote text from the end of `previous`, as Backspace does. */
export function isDeletionOf(next: string[], previous: string[]) {
	const nextText = next.join('\0');
	const previousText = previous.join('\0');
	return nextText.length < previousText.length && previousText.startsWith(nextText);
}

export interface CommandPrefix {
	/** Leading repeat-count digits. */
	count: string[];
	/** Options inside a closed `config` block, or null when there is none. */
	config: string[] | null;
	/** Everything before the chord being typed: count, config and earlier chords. */
	tokens: string[];
	/** Zero-based position of the step being typed in an Enter-separated sequence. */
	chordIndex: number;
}

/** Mirrors the native command parser: Enter separates steps after a count or config block. */
export function getCommandPrefix(tokens: string[]): CommandPrefix {
	if (tokens[0] === '0') return { count: [], config: null, tokens: [], chordIndex: 0 };
	const countEnd = tokens.findIndex(token => !/^\d$/.test(token));
	const count = tokens.slice(0, countEnd === -1 ? tokens.length : countEnd);
	let start = count.length;
	let config: string[] | null = null;
	if (tokens[start] === '`') {
		const end = tokens.indexOf('`', start + 1);
		if (end === -1) return { count, config, tokens: count, chordIndex: 0 };
		config = tokens.slice(start + 1, end);
		start = end + 1;
	}
	let chordIndex = 0;
	while (count.length > 0 || config) {
		let end = start;
		if (tokens[end] === "'") {
			// A quote, which may contain Enter, then its optional config block.
			end = tokens.indexOf("'", end + 1);
			if (end === -1) break;
			end++;
			if (tokens[end] === '`') {
				end = tokens.indexOf('`', end + 1);
				if (end === -1) break;
				end++;
			}
			if (tokens[end] !== ENTER) break;
		} else {
			end = tokens.indexOf(ENTER, end);
			if (end === -1) break;
		}
		start = end + 1;
		chordIndex++;
	}
	return { count, config, tokens: tokens.slice(0, start), chordIndex };
}

export function getCommandLabel(prefix: CommandPrefix) {
	const parts: string[] = [];
	if (prefix.count.length > 0) {
		const count = Number(prefix.count.join(''));
		parts.push(count === 1 ? 'Loop' : `Repeat ${count} times`);
	}
	if (prefix.config) parts.push(`\`${prefix.config.join('').toLowerCase()}\``);
	if (prefix.chordIndex > 0) parts.push(`Step ${prefix.chordIndex + 1}`);
	return parts.join(' · ');
}

export function getRangeHint(hint: { prefix: string[], keys: string[], description: string, isRepeatCount: boolean }, commandPrefix: CommandPrefix) {
	const prefixTokens = commandPrefix.tokens;
	// Digits after a config block or separator start a chord, not a count.
	if (hint.isRepeatCount && prefixTokens.length > commandPrefix.count.length) return null;
	const keys = hint.isRepeatCount && prefixTokens.length > 0
		? ['0', ...hint.keys.filter(key => key !== '0')]
		: hint.keys;
	return {
		prefix: [...prefixTokens, ...hint.prefix],
		// Keep the dedicated loop row at the start of a sequence.
		keys: hint.isRepeatCount && prefixTokens.length === 0 ? keys.filter(key => key !== '1') : keys,
		label: keys.length === 1 ? keys[0]! : `${keys[0]}-${keys.at(-1)}`,
		description: hint.description,
	};
}
