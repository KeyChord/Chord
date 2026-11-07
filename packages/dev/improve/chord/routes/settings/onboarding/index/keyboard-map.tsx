import { cn } from '@chord/com.npmjs.utils-cn';

interface MapKey {
	/** What the key is called on a Mac keyboard. */
	label: string;
	/** The character it types, if a chord can use it. */
	id?: string;
	/** Width in units of a letter key. */
	width?: number;
}

const letters = (keys: string): MapKey[] => [...keys].map(key => ({ label: key, id: key }));

const ROWS: MapKey[][] = [
	[{ label: 'tab', width: 1.5 }, ...letters('qwertyuiop[]\\')],
	[{ label: 'caps lock', id: 'caps', width: 1.8 }, ...letters('asdfghjkl;\''), { label: 'return', width: 1.8 }],
	[{ label: 'shift', width: 2.3 }, ...letters('zxcvbnm,./'), { label: 'shift', width: 2.3 }],
	[
		{ label: 'fn' },
		{ label: 'control' },
		{ label: 'option' },
		{ label: 'command', width: 1.3 },
		{ label: 'space', id: 'space', width: 5.4 },
		{ label: 'command', width: 1.3 },
		{ label: 'option' },
	],
];

/**
 * A small Mac keyboard that shows where the keys are. Most people have never needed to find the
 * Caps Lock key by name, and a symbol alone is easy to confuse with Shift.
 */
export function KeyboardMap({
	highlighted,
	pressed = [],
}: {
	/** Keys to find, by the character they type, or `caps` and `space`. */
	highlighted: readonly string[];
	/** Keys that are down right now. */
	pressed?: readonly string[];
}) {
	return (
		<div
			role="img"
			aria-label={`Keyboard with ${highlighted.join(', ')} highlighted`}
			className="flex w-full max-w-[560px] select-none flex-col gap-[3px] rounded-[12px] border border-black/[0.08] bg-black/[0.03] p-2 dark:border-white/10 dark:bg-white/[0.04]"
		>
			{ROWS.map((row, index) => (
				<div key={index} className="flex gap-[3px]">
					{row.map((key, keyIndex) => {
						const isPressed = key.id !== undefined && pressed.includes(key.id);
						const isHighlighted = key.id !== undefined && highlighted.includes(key.id);
						return (
							<div
								key={keyIndex}
								data-key={key.id}
								data-pressed={isPressed || undefined}
								style={{ flex: key.width ?? 1 }}
								className={cn(
									'flex h-[27px] min-w-0 items-center justify-center overflow-hidden whitespace-nowrap rounded-[5px] border font-mono text-[9.5px]',
									isPressed
										? 'border-emerald-600 bg-emerald-500 font-semibold text-white'
										: isHighlighted
											? 'border-primary bg-primary font-semibold text-primary-foreground'
											: 'border-black/[0.08] bg-background text-muted-foreground/70 dark:border-white/10',
								)}
							>
								{key.label}
							</div>
						);
					})}
				</div>
			))}
		</div>
	);
}

/** A key named in instructions, drawn like the key it refers to. */
export function Key({ children }: { children: React.ReactNode }) {
	return (
		<kbd className="mx-[1px] inline-flex h-[20px] min-w-[20px] items-center justify-center rounded-[5px] border border-black/15 bg-background px-1.5 align-[1px] font-mono text-[11.5px] font-medium text-foreground shadow-[0_1px_0_rgba(0,0,0,0.12)] dark:border-white/20">
			{children}
		</kbd>
	);
}
