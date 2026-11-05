/**
 * Follows someone typing their first chord, from the events Chord itself publishes, so the
 * guidance reacts to what they actually did. Every event matters: a chord that runs is reported
 * and cleared in quick succession, so this works from the event stream and not rendered state.
 */
export type PracticeHint = 'released-too-soon' | 'wrong-keys' | null;

export interface PracticeProgress {
	/** Caps Lock and Space were pressed and Space is still held. */
	inChordMode: boolean;
	/** The keys typed so far, as characters. */
	typed: string;
	/**
	 * The last keys typed since entering Chord mode. Keys that aren't a chord are cleared just
	 * before Chord mode ends, and that should read as a wrong chord, not as typing nothing.
	 */
	attempted: string;
	/** The target chord ran. */
	ran: boolean;
	hint: PracticeHint;
}

export const initialPractice: PracticeProgress = {
	inChordMode: false,
	typed: '',
	attempted: '',
	ran: false,
	hint: null,
};

export type PracticeEvent =
	| { type: 'mode'; chord: boolean }
	| {
		type: 'input';
		input: readonly string[];
		executed: readonly string[] | null;
		failed: boolean;
	};

const NAMED_KEYS: Record<string, string> = {
	Slash: '/',
	Backslash: '\\',
	Semicolon: ';',
	Quote: '\'',
	Comma: ',',
	Period: '.',
	Minus: '-',
	Equal: '=',
	Backquote: '`',
	BracketLeft: '[',
	BracketRight: ']',
};

/** A key as published by Chord, such as `KeyS` or `Slash`, as the character it types. */
export function keyToText(key: string) {
	const letter = /^Key([A-Z])$/.exec(key)?.[1];
	if (letter) return letter.toLowerCase();
	const digit = /^Digit(\d)$/.exec(key)?.[1];
	if (digit) return digit;
	return NAMED_KEYS[key] ?? key;
}

export function keysToText(keys: readonly string[]) {
	return keys.map(keyToText).join('');
}

export function advancePractice(
	state: PracticeProgress,
	event: PracticeEvent,
	target: string,
): PracticeProgress {
	if (state.ran) return { ...state, inChordMode: event.type === 'mode' ? event.chord : state.inChordMode };
	if (event.type === 'mode') {
		if (event.chord) return { ...initialPractice, inChordMode: true };
		// Space came up. A chord that ran has been reported by now, so this ended without one.
		if (!state.inChordMode) return state;
		return {
			...state,
			inChordMode: false,
			typed: state.attempted,
			hint: state.attempted === '' ? 'released-too-soon' : 'wrong-keys',
		};
	}
	if (event.executed) {
		const executed = keysToText(event.executed);
		if (!event.failed && executed === target) {
			return { ...state, typed: executed, attempted: executed, ran: true, hint: null };
		}
		return { ...state, typed: executed, attempted: executed };
	}
	const typed = keysToText(event.input);
	if (!state.inChordMode) return state;
	return { ...state, typed, attempted: typed || state.attempted };
}

export type PracticeTask = 'hold' | 'type' | 'release';

/** The instruction someone should be following right now. */
export function currentTask({ inChordMode, typed }: PracticeProgress, target: string): PracticeTask {
	if (!inChordMode) return 'hold';
	return typed === target ? 'release' : 'type';
}
