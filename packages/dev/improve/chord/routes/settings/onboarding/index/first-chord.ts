/** Chords that start with this work in every app, which is what a first chord should do. */
export const GLOBAL_PREFIX = '/';

const MAX_KEYS = 3;

/** What a person can type after the prefix: letters and digits, up to a short limit. */
export function sanitizeChordKeys(text: string) {
	return text
		.toLowerCase()
		.replaceAll(/[^a-z0-9]/g, '')
		.slice(0, MAX_KEYS);
}

/**
 * The first letter of the app's name that no other chord uses. Falling back to the plain first
 * letter leaves the conflict for the person to see and resolve.
 */
export function suggestChordKeys(appName: string, taken: readonly string[] = []) {
	const letters = [...appName.toLowerCase().replaceAll(/[^a-z0-9]/g, '')];
	return letters.find(letter => !taken.includes(GLOBAL_PREFIX + letter)) ?? letters[0] ?? '';
}

export function chordSequence(keys: string) {
	return GLOBAL_PREFIX + keys;
}
