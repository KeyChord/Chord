import { Kbd, KbdGroup } from '#/components/ui/kbd.tsx';
import {
	sfCapslock,
	sfCommand,
	sfControl,
	sfGlobe,
	sfOption,
	sfShift,
	sfSpace,
} from '@bradleyhodges/sfsymbols';
import { SFIcon } from '@bradleyhodges/sfsymbols-react';
import getPrettyKey from 'pretty-key';

export function ShortcutKeys({ shortcut }: { shortcut: string }) {
	const chords = shortcut.split(' ').reduce<Array<{ id: string, keys: string[] }>>((items, chord) => {
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
						{chord.keys.map((key) => {
							const modifierIcon = getModifierIcon(key);
							const label = getPrettyKey(key);

							return (
								<Kbd key={key} className={modifierIcon ? 'px-1.5' : 'font-mono text-[11px]'}>
									{modifierIcon
										? (
												<>
													<SFIcon icon={modifierIcon} size={11} aria-label={label} />
													<span className="sr-only">{label}</span>
												</>
											)
										: (
												label
											)}
								</Kbd>
							);
						})}
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

function getModifierIcon(key: string) {
	switch (key.toLowerCase()) {
		case 'cmd':
		case 'command':
		case 'meta':
			return sfCommand;
		case 'ctrl':
		case 'control':
			return sfControl;
		case 'shift':
			return sfShift;
		case 'alt':
		case 'option':
			return sfOption;
		case 'capslock':
		case 'caps_lock':
			return sfCapslock;
		case 'fn':
		case 'globe':
			return sfGlobe;
		case 'space':
		case 'spacebar':
			return sfSpace;
		default:
			return null;
	}
}
