const COLLAPSE_EARLIER_KEYS_KEY = 'chord-collapse-earlier-keys';

/**
 * Whether the Hint Panel folds the earliest keys of a long chord into a trail ("; > ›")
 * beside its columns. Settings and the panel share this storage; the panel reads it each
 * time it draws, so a change applies at the next chord.
 */
export function getCollapseEarlierKeys(): boolean {
	try {
		return window.localStorage.getItem(COLLAPSE_EARLIER_KEYS_KEY) !== 'false';
	} catch {
		return true;
	}
}

export function setCollapseEarlierKeys(collapse: boolean) {
	try {
		window.localStorage.setItem(COLLAPSE_EARLIER_KEYS_KEY, String(collapse));
	} catch {
		// Without storage the panel keeps the default.
	}
}
