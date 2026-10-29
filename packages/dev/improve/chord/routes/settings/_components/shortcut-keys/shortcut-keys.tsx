import {
	sfArrowDown,
	sfArrowLeft,
	sfArrowRight,
	sfArrowRightToLine,
	sfArrowUp,
	sfCapslock,
	sfCommand,
	sfControl,
	sfDeleteLeft,
	sfDeleteRight,
	sfGlobe,
	sfOption,
	sfReturn,
	sfShift,
} from '@bradleyhodges/sfsymbols';
import { SFIcon } from '@bradleyhodges/sfsymbols-react';
import { Kbd, KbdGroup } from '@chord/dev.improve.chord.components.ui.kbd';
import getPrettyKey from 'pretty-key';

// Draws symbols at the size and medium weight SF Pro gives the same glyphs in 10px key labels.
const SYMBOL_SCALE = 0.43;
const SYMBOL_STROKE = 0.8;

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
	const symbol = getKeySymbol(shortcutKey);
	if (symbol) {
		const [, , width = 0, height = 0] = symbol.icon.viewBox.split(' ').map(Number);
		return (
			<Kbd className="shortcut-key" data-symbol="">
				<SFIcon
					icon={symbol.icon}
					width={width * SYMBOL_SCALE}
					height={height * SYMBOL_SCALE}
					weight={SYMBOL_STROKE}
					aria-label={symbol.label}
				/>
			</Kbd>
		);
	}

	switch (shortcutKey.toLowerCase()) {
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
		default:
			return <Kbd className="shortcut-key">{getPrettyKey(shortcutKey)}</Kbd>;
	}
}

function getKeySymbol(key: string): { icon: typeof sfOption, label: string } | null {
	switch (key.toLowerCase()) {
		case 'cmd':
		case 'command':
		case 'meta':
			return { icon: sfCommand, label: 'Command' };
		case 'ctrl':
		case 'control':
			return { icon: sfControl, label: 'Control' };
		case 'shift':
			return { icon: sfShift, label: 'Shift' };
		case 'alt':
		case 'option':
			return { icon: sfOption, label: 'Option' };
		case 'capslock':
		case 'caps_lock':
			return { icon: sfCapslock, label: 'Caps Lock' };
		case 'fn':
		case 'globe':
			return { icon: sfGlobe, label: 'Fn' };
		case 'enter':
		case 'return':
			return { icon: sfReturn, label: 'Return' };
		case 'tab':
			return { icon: sfArrowRightToLine, label: 'Tab' };
		case 'backspace':
			return { icon: sfDeleteLeft, label: 'Delete' };
		case 'delete':
			return { icon: sfDeleteRight, label: 'Forward Delete' };
		case 'up':
			return { icon: sfArrowUp, label: 'Up Arrow' };
		case 'down':
			return { icon: sfArrowDown, label: 'Down Arrow' };
		case 'left':
			return { icon: sfArrowLeft, label: 'Left Arrow' };
		case 'right':
			return { icon: sfArrowRight, label: 'Right Arrow' };
		default:
			return null;
	}
}
