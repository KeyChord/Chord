import { expect, test } from 'bun:test';

test('an Auto response cannot repaint a newer explicit appearance', async () => {
	const previousWindow = globalThis.window;
	const previousDocument = globalThis.document;
	const preferences = new Map<string, string>();
	let finishThemeRead!: (theme: string) => void;
	let markThemeRead!: () => void;
	const themeRead = new Promise<void>(resolve => { markThemeRead = resolve; });
	const theme = new Promise<string>(resolve => { finishThemeRead = resolve; });
	const dataset: Record<string, string> = {};
	const broadcasts: unknown[] = [];
	globalThis.document = {
		documentElement: { dataset, style: {}, classList: { toggle() {} } },
	} as unknown as Document;
	globalThis.window = {
		matchMedia: () => ({ matches: false }),
		localStorage: {
			getItem: (key: string) => preferences.get(key) ?? null,
			setItem: (key: string, value: string) => preferences.set(key, value),
		},
		__TAURI_INTERNALS__: {
			metadata: { currentWindow: { label: 'settings' } },
			invoke: async (command: string, args: { payload?: unknown }) => {
				if (command === 'plugin:window|theme') {
					markThemeRead();
					return theme;
				}
				if (command === 'plugin:event|emit_to') broadcasts.push(args.payload);
			},
		},
	} as unknown as Window & typeof globalThis;

	try {
		const { getAppearance, refreshAppearance, setAppearance } = await import('#appearance');
		const auto = refreshAppearance();
		await themeRead;
		await setAppearance('dark');
		finishThemeRead('light');
		await auto;

		expect(getAppearance()).toBe('dark');
		expect(dataset.theme).toBe('dark');
		expect(broadcasts).toEqual(['dark']);
	} finally {
		globalThis.window = previousWindow;
		globalThis.document = previousDocument;
	}
});
