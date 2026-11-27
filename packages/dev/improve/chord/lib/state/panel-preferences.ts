// Hint Panel preferences chosen in Settings. Settings and the panel share this storage;
// the panel reads it each time it draws, so a change applies at the next chord.

const COLLAPSE_EARLIER_KEYS_KEY = 'chord-collapse-earlier-keys';
const TEXT_FONT_KEY = 'chord-panel-text-font';
const KEY_FONT_KEY = 'chord-panel-key-font';

export const PANEL_TEXT_FONTS = {
	geist: { label: 'Geist', family: '"Geist Variable", sans-serif' },
	system: { label: 'SF Pro', family: '-apple-system, BlinkMacSystemFont, system-ui, sans-serif' },
	rounded: { label: 'Rounded', family: 'ui-rounded, -apple-system, system-ui, sans-serif' },
	serif: { label: 'New York', family: 'ui-serif, "New York", Georgia, serif' },
} as const;

export const PANEL_KEY_FONTS = {
	geist: { label: 'Geist Mono', family: '"Geist Mono Variable", ui-monospace, monospace' },
	system: { label: 'SF Mono', family: 'ui-monospace, "SF Mono", monospace' },
	text: { label: 'Same as text', family: '' },
} as const;

export type PanelTextFont = keyof typeof PANEL_TEXT_FONTS;
export type PanelKeyFont = keyof typeof PANEL_KEY_FONTS;

function read(key: string) {
	try {
		return window.localStorage.getItem(key);
	} catch {
		return null;
	}
}

function write(key: string, value: string) {
	try {
		window.localStorage.setItem(key, value);
	} catch {
		// Without storage the panel keeps the defaults.
	}
}

/** Whether a long chord's earliest keys fold into a trail beside the sidebar's columns. */
export function getCollapseEarlierKeys(): boolean {
	return read(COLLAPSE_EARLIER_KEYS_KEY) !== 'false';
}

export function setCollapseEarlierKeys(collapse: boolean) {
	write(COLLAPSE_EARLIER_KEYS_KEY, String(collapse));
}

export function getPanelTextFont(): PanelTextFont {
	const stored = read(TEXT_FONT_KEY);
	return stored && Object.hasOwn(PANEL_TEXT_FONTS, stored) ? stored as PanelTextFont : 'geist';
}

export function setPanelTextFont(font: PanelTextFont) {
	write(TEXT_FONT_KEY, font);
}

export function getPanelKeyFont(): PanelKeyFont {
	const stored = read(KEY_FONT_KEY);
	return stored && Object.hasOwn(PANEL_KEY_FONTS, stored) ? stored as PanelKeyFont : 'geist';
}

export function setPanelKeyFont(font: PanelKeyFont) {
	write(KEY_FONT_KEY, font);
}

/** The CSS that gives the panel the chosen fonts; empty with the defaults. */
export function panelFontStyles() {
	const text = getPanelTextFont();
	const key = getPanelKeyFont();
	const rules = [];
	if (text !== 'geist') {
		rules.push(`html { font-family: ${PANEL_TEXT_FONTS[text].family}; }`);
	}
	if (key !== 'geist') {
		const family = key === 'text' ? PANEL_TEXT_FONTS[text].family : PANEL_KEY_FONTS[key].family;
		rules.push(`kbd, .font-mono { font-family: ${family}; }`);
	}
	return rules.join('\n');
}
