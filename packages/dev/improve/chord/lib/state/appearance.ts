import { getCurrentWindow } from '@tauri-apps/api/window';

/** Light, Dark, or the macOS setting (which may itself be Auto). */
export type Appearance = 'system' | 'light' | 'dark';

const STORAGE_KEY = 'chord-appearance';
const darkScheme = window.matchMedia('(prefers-color-scheme: dark)');

/** The appearance chosen in Settings, for Settings and the Hint Panel. */
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
 * Gives this window the appearance natively too, so its title bar, sidebar material and
 * the Hint Panel's frosted glass match its content. With 'system' the window follows macOS
 * again.
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

let appliedNativeTheme: Appearance | undefined;

/**
 * Applies the stored appearance to this window. Cheap when nothing changed, so the Hint
 * Panel calls it before every show: storage events don't reliably reach its page.
 */
export function refreshAppearance() {
	applyClasses();
	const appearance = getAppearance();
	if (appearance !== appliedNativeTheme) {
		appliedNativeTheme = appearance;
		void applyNativeTheme(appearance);
	}
}

/** Applies the appearance before the first render and keeps it current. */
export function followAppearance() {
	refreshAppearance();
	darkScheme.addEventListener('change', applyClasses);
	// Changed in another window.
	window.addEventListener('storage', event => {
		if (event.key === STORAGE_KEY) {
			refreshAppearance();
		}
	});
}
