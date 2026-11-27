import { getCurrentWindow } from '@tauri-apps/api/window';

/** Light, Dark, or the macOS setting (which may itself be Auto). */
export type Appearance = 'system' | 'light' | 'dark';

const STORAGE_KEY = 'chord-appearance';
const darkScheme = window.matchMedia('(prefers-color-scheme: dark)');

/** The appearance chosen for the Settings window. */
export function getAppearance(): Appearance {
	try {
		const stored = window.localStorage.getItem(STORAGE_KEY);
		return stored === 'light' || stored === 'dark' ? stored : 'system';
	} catch {
		return 'system';
	}
}

function applyClasses() {
	const appearance = getAppearance();
	const isDark = appearance === 'system' ? darkScheme.matches : appearance === 'dark';
	const root = document.documentElement;
	root.classList.toggle('dark', isDark);
	root.dataset.theme = isDark ? 'dark' : 'light';
	root.style.colorScheme = isDark ? 'dark' : 'light';
}

/**
 * Gives this window the appearance natively too, so its title bar and sidebar material
 * match. Only this window: the Hint Panel keeps following macOS. With 'system' the window
 * follows macOS again.
 */
async function applyNativeTheme(appearance: Appearance) {
	try {
		await getCurrentWindow().setTheme(appearance === 'system' ? null : appearance);
	} catch (error) {
		console.warn('Could not apply the appearance to the window', error);
	}
}

export async function setAppearance(appearance: Appearance) {
	try {
		window.localStorage.setItem(STORAGE_KEY, appearance);
	} catch {
		// Without storage the choice still applies until Chord quits.
	}
	applyClasses();
	await applyNativeTheme(appearance);
}

/** Applies the appearance before the first render and keeps it current. */
export function followAppearance() {
	applyClasses();
	darkScheme.addEventListener('change', applyClasses);
	// Changed in another Settings window.
	window.addEventListener('storage', event => {
		if (event.key === STORAGE_KEY) {
			applyClasses();
		}
	});
	void applyNativeTheme(getAppearance());
}
