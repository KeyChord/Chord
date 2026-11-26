// Settings and the Hint Panel follow the macOS appearance (Light, Dark, or Auto as it
// changes through the day). The palette in theme.css, the settings styles in global.css
// and Tailwind's `dark:` variant all key off the `dark` class and `data-theme` set here,
// so they switch together with the native vibrancy behind the panel.
const darkScheme = window.matchMedia('(prefers-color-scheme: dark)');

function applyAppearance(isDark: boolean) {
	const root = document.documentElement;
	root.classList.toggle('dark', isDark);
	root.dataset.theme = isDark ? 'dark' : 'light';
	root.style.colorScheme = isDark ? 'dark' : 'light';
}

/** Applies the current appearance now, before the first render, and on every change. */
export function followSystemAppearance() {
	applyAppearance(darkScheme.matches);
	darkScheme.addEventListener('change', event => {
		applyAppearance(event.matches);
	});
}
