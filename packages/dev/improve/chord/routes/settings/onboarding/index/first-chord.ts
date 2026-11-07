import type { ChordPackageManagerState, ChordReference } from '@chord/dev.improve.chord.lib.typeshare';
import { keysToText } from '#practice';

/** Chords that start with this work in every app, which is what a first chord should do. */
export const GLOBAL_PREFIX = '/';

/** The personal package new chords are saved to; `DEFAULT_PERSONAL_PACKAGE` in the backend. */
const CHORDS_FOLDER = 'default';

const MAX_KEYS = 3;

/** What a person can type after the prefix: letters and digits, up to a short limit. */
export function sanitizeChordKeys(text: string) {
	return text
		.toLowerCase()
		.replaceAll(/[^a-z0-9]/g, '')
		.slice(0, MAX_KEYS);
}

export function chordSequence(keys: string) {
	return GLOBAL_PREFIX + keys;
}

/**
 * The chord already on these keys, highest priority first. Chords with no action never run, so
 * they leave the keys free. Patterns are left out: only the backend can evaluate them, and a
 * personal chord takes priority over one anyway.
 */
export function findChordConflict(sequence: string, chords: readonly ChordReference[]) {
	return chords.find(({ chord }) =>
		chord.actions.length > 0 && 'keys' in chord.trigger && keysToText(chord.trigger.keys) === sequence);
}

/**
 * The first letter of the app's name that no other chord uses. Falling back to the plain first
 * letter leaves the conflict for the person to see and resolve.
 */
export function suggestChordKeys(appName: string, chords: readonly ChordReference[] = []) {
	const letters = [...appName.toLowerCase().replaceAll(/[^a-z0-9]/g, '')];
	return letters.find(letter => !findChordConflict(chordSequence(letter), chords)) ?? letters[0] ?? '';
}

/**
 * The folder new chords are saved to, when Chord isn't allowed to load it. Chord only approves
 * this folder by itself when it creates it, so one left from an earlier install needs a person
 * to say so. Until then, a chord saved there would never run.
 */
export function findBlockedChordsFolder({
	personalPackages,
	thirdPartyPackages,
}: Pick<ChordPackageManagerState, 'personalPackages' | 'thirdPartyPackages'>) {
	const folder = personalPackages.find(({ dirname }) => dirname === CHORDS_FOLDER);
	return thirdPartyPackages.find(pkg =>
		!pkg.approved && pkg.name === folder?.name && pkg.path.endsWith(`/${CHORDS_FOLDER}`));
}
