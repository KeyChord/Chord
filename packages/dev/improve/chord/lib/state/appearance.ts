import { emitTo } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';

/** Light, Dark, or the macOS setting (which may itself be Auto). */
export type Appearance = 'system' | 'light' | 'dark';

const STORAGE_KEY = 'chord-appearance';
/** Sent to the Hint Panel when Settings changes the appearance. */
const CHANGED_EVENT = 'chord-appearance-changed';
const darkScheme = window.matchMedia('(prefers-color-scheme: dark)');
let windowAppearance: Appearance | undefined;

function isAppearance(value: unknown): value is Appearance {
	return value === 'system' || value === 'light' || value === 'dark';
}

/** The appearance chosen in Settings, for Settings and the Hint Panel. */
export function getAppearance(): Appearance {
	if (windowAppearance !== undefined) return windowAppearance;
	try {
		const stored = window.localStorage.getItem(STORAGE_KEY);
		return isAppearance(stored) ? stored : 'system';
	} catch {
		return 'system';
	}
}

function store(appearance: Appearance) {
	try {
		window.localStorage.setItem(STORAGE_KEY, appearance);
	} catch {
		// Without storage the choice still applies until Chord quits.
	}
}

function applyClasses(isDark: boolean) {
	const root = document.documentElement;
	root.classList.toggle('dark', isDark);
	root.dataset.theme = isDark ? 'dark' : 'light';
	root.style.colorScheme = isDark ? 'dark' : 'light';
}

/**
 * Whether this window is dark. With Auto, the window's own theme decides: the Hint Panel's
 * page cannot rely on prefers-color-scheme, and drew its light palette over dark glass.
 */
async function isSystemDark() {
	try {
		return (await getCurrentWindow().theme()) === 'dark';
	} catch {
		return darkScheme.matches;
	}
}

let requestedNativeTheme: Appearance | undefined;
let pendingNativeTheme = Promise.resolve();
let refreshVersion = 0;

/**
 * Gives this window the appearance natively too, so its title bar, sidebar material and
 * the Hint Panel's frosted glass match its content. With 'system' the window follows macOS
 * again.
 */
function applyNativeTheme(appearance: Appearance) {
	if (appearance !== requestedNativeTheme) {
		requestedNativeTheme = appearance;
		const update = pendingNativeTheme
			.then(() => getCurrentWindow().setTheme(appearance === 'system' ? null : appearance))
			.catch(error => {
				if (pendingNativeTheme === update) requestedNativeTheme = undefined;
				console.warn('Could not apply the appearance to the window', error);
			});
		pendingNativeTheme = update;
	}
	return pendingNativeTheme;
}

/**
 * Applies the stored appearance to this window. Cheap when nothing changed, so the Hint
 * Panel calls it before every show.
 */
export async function refreshAppearance() {
	const version = ++refreshVersion;
	const appearance = getAppearance();
	// The classes first, from what the page can tell at once, so nothing waits on the window.
	applyClasses(appearance === 'system' ? darkScheme.matches : appearance === 'dark');
	await applyNativeTheme(appearance);
	if (appearance === 'system' && version === refreshVersion) {
		const dark = await isSystemDark();
		if (version === refreshVersion) applyClasses(dark);
	}
}

export async function setAppearance(appearance: Appearance) {
	windowAppearance = appearance;
	store(appearance);
	await refreshAppearance();
	pendingBroadcast = pendingBroadcast.then(async () => {
		if (getAppearance() === appearance) {
			await emitTo('chords', CHANGED_EVENT, appearance).catch(() => {});
		}
	});
	await pendingBroadcast;
}

let pendingBroadcast = Promise.resolve();

/** Applies the appearance before the first render and keeps it current. */
export function followAppearance() {
	void refreshAppearance();
	darkScheme.addEventListener('change', () => void refreshAppearance());
	const currentWindow = getCurrentWindow();
	void currentWindow
		.onThemeChanged(() => void refreshAppearance())
		.catch(() => {});
	void currentWindow.listen<unknown>(CHANGED_EVENT, event => {
		if (isAppearance(event.payload)) {
			// Settings owns the stored value. A delayed event must not write it back.
			windowAppearance = event.payload;
			void refreshAppearance();
		}
	}).catch(() => {});
	window.addEventListener('storage', event => {
		if (event.key === STORAGE_KEY) {
			windowAppearance = undefined;
			void refreshAppearance();
		}
	});
}
