import { getCurrentWindow } from '@tauri-apps/api/window';

function handleTitleBarMouseDown(event: React.MouseEvent<HTMLElement>) {
	if (event.buttons === 1) void getCurrentWindow().startDragging();
}

/** One screen of setup: what this step is for, the step itself, and the way forward. */
export function StepLayout({
	title,
	description,
	stepNumber,
	stepCount,
	children,
	actions,
	secondary,
}: {
	title: string;
	description?: React.ReactNode;
	stepNumber?: number;
	stepCount?: number;
	children?: React.ReactNode;
	actions: React.ReactNode;
	/** A quieter way out, such as skipping. */
	secondary?: React.ReactNode;
}) {
	return (
		<div className="settings-content-surface flex h-full w-full flex-col bg-background">
			<header className="h-[46px] shrink-0" onMouseDown={handleTitleBarMouseDown} />
			<main className="min-h-0 flex-1 overflow-y-auto px-10">
				<div className="mx-auto flex min-h-full w-full max-w-[600px] flex-col justify-center gap-5 pb-4">
					<div className="space-y-2">
						{stepNumber !== undefined && stepCount !== undefined
							? (
									<p className="text-[11.5px] font-medium text-muted-foreground">
										Step {stepNumber} of {stepCount}
									</p>
								)
							: null}
						<h1 className="text-[24px] font-semibold leading-[30px] tracking-[-0.01em] text-foreground">
							{title}
						</h1>
						{description
							? <div className="text-[14px] leading-[21px] text-muted-foreground">{description}</div>
							: null}
					</div>
					{children}
				</div>
			</main>
			<footer className="flex shrink-0 items-center justify-between gap-3 border-t border-black/[0.06] px-6 py-3.5 dark:border-white/10">
				<div>{secondary}</div>
				<div className="flex items-center gap-2">{actions}</div>
			</footer>
		</div>
	);
}

/** A numbered instruction, in the order someone should follow them. */
export function Instruction({
	number,
	state = 'todo',
	children,
}: {
	number: number;
	state?: 'todo' | 'current' | 'done';
	children: React.ReactNode;
}) {
	return (
		<li data-state={state} className="flex items-start gap-3">
			<span
				aria-hidden="true"
				className={
					state === 'done'
						? 'flex size-[22px] shrink-0 items-center justify-center rounded-full bg-emerald-500 text-[12px] font-semibold text-white'
						: state === 'current'
							? 'flex size-[22px] shrink-0 items-center justify-center rounded-full bg-primary text-[11.5px] font-semibold text-primary-foreground'
							: 'flex size-[22px] shrink-0 items-center justify-center rounded-full bg-black/[0.07] text-[11.5px] font-semibold text-muted-foreground dark:bg-white/10'
				}
			>
				{state === 'done' ? '✓' : number}
			</span>
			<div
				className={
					state === 'todo'
						? 'min-w-0 pt-px text-[13.5px] leading-[21px] text-muted-foreground'
						: 'min-w-0 pt-px text-[13.5px] leading-[21px] text-foreground'
				}
			>
				{children}
			</div>
		</li>
	);
}

/** Text for what to click, in the same words as the button itself. */
export function Click({ children }: { children: React.ReactNode }) {
	return <strong className="font-semibold text-foreground">{children}</strong>;
}
