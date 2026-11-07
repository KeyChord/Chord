import { hasKeySymbol, Kbd, KbdGroup, KeyLabel } from '@chord/dev.improve.chord.components.ui.kbd';
import getPrettyKey from 'pretty-key';

// Key names used in shortcuts that pretty-key doesn't recognize.
const KEY_ALIASES = new Map([
	['meta', '⌘'],
	['ctrl', '⌃'],
	['control', '⌃'],
	['alt', '⌥'],
	['caps_lock', '⇪'],
	['globe', '🌐'],
]);

export function ShortcutKeys({ shortcut }: { shortcut: string }) {
	const chords = shortcut.split(' ').reduce<{ id: string, keys: string[] }[]>((items, chord) => {
		const prefix = items.length > 0 ? `${items.at(-1)?.id ?? ''} ${chord}` : chord;
		items.push({
			id: prefix,
			keys: chord.split('+'),
		});
		return items;
	}, []);

	return (
		<div className="flex flex-wrap items-center gap-1.5">
			{chords.map(chord => (
				<div
					key={`${shortcut}:${chord.id}`}
					className="flex items-center gap-1.5"
				>
					<KbdGroup>
						{chord.keys.map(key => (
							<ShortcutKey key={key} shortcutKey={key} />
						))}
					</KbdGroup>
					{chord.id !== chords.at(-1)?.id
						? (
								<span className="text-xs text-muted-foreground">then</span>
							)
						: null}
				</div>
			))}
		</div>
	);
}

function ShortcutKey({ shortcutKey }: { shortcutKey: string }) {
	const name = shortcutKey.toLowerCase();
	switch (name) {
		case 'esc':
		case 'escape':
			return (
				<Kbd className="shortcut-key" data-symbol="" data-compact="">
					esc
				</Kbd>
			);
		case 'space':
		case 'spacebar':
			return <Kbd className="shortcut-key">Space</Kbd>;
		default: {
			const key = KEY_ALIASES.get(name) ?? getPrettyKey(shortcutKey);
			return (
				<Kbd className="shortcut-key" data-symbol={hasKeySymbol(key) ? '' : undefined}>
					<KeyLabel label={key} />
				</Kbd>
			);
		}
	}
}
