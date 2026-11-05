import { taurpc } from '@chord/dev.improve.chord.api.taurpc';
import { Button } from '@chord/dev.improve.chord.components.ui.button';
import { Loader2, ShieldCheck } from 'lucide-react';
import { useState } from 'react';
import { Click, Instruction, StepLayout } from '#step-layout';

/**
 * Tells someone exactly what macOS is about to show them, in macOS's own words, before they see
 * it. The rest of setup continues by itself when the permission is granted, which arrives as
 * an update to Chord's state. This screen only explains and starts the process.
 */
export function PermissionStep({
	paneName,
	stepNumber,
	stepCount,
	onSkip,
}: {
	paneName: string;
	stepNumber: number;
	stepCount: number;
	onSkip: () => void;
}) {
	const [hasOpened, setHasOpened] = useState(false);
	function open() {
		// macOS's prompt opens the right page by itself the first time. After that it may not
		// come back, so go straight to the page.
		if (hasOpened) void taurpc.openAccessibilitySettings();
		else void taurpc.requestAccessibilityAccess();
		setHasOpened(true);
	}
	return (
		<StepLayout
			stepNumber={stepNumber}
			stepCount={stepCount}
			title="Allow Chord to use your keyboard"
			description="Chord needs to notice when you start a chord, and press keys for you. Only you can allow that, in your Mac's settings."
			actions={(
				<Button type="button" size="lg" onClick={open}>
					{hasOpened ? 'Open System Settings Again' : 'Open System Settings'}
				</Button>
			)}
			secondary={(
				<Button type="button" variant="ghost" onClick={onSkip}>
					Skip for Now
				</Button>
			)}
		>
			<ol className="space-y-3">
				<Instruction number={1} state={hasOpened ? 'done' : 'current'}>
					Click <Click>Open System Settings</Click> below.
				</Instruction>
				<Instruction number={2} state={hasOpened ? 'current' : 'todo'}>
					macOS asks if Chord can control this Mac. Click <Click>Open System Settings</Click> in that
					message too.
				</Instruction>
				<Instruction number={3} state={hasOpened ? 'current' : 'todo'}>
					On the <Click>{paneName}</Click> page, turn on the switch next to <Click>Chord</Click>.
				</Instruction>
				<Instruction number={4} state={hasOpened ? 'current' : 'todo'}>
					If asked, type the password you use to log in to your Mac and click <Click>Modify Settings</Click>.
				</Instruction>
			</ol>
			{hasOpened
				? (
						<p className="flex items-center gap-2 text-[13.5px] font-medium text-foreground" role="status">
							<Loader2 className="size-4 animate-spin" />
							Waiting for you to turn on Chord. This window comes back by itself.
						</p>
					)
				: null}
			<div className="flex gap-3 rounded-[12px] border border-black/[0.08] bg-black/[0.02] p-4 dark:border-white/10 dark:bg-white/[0.03]">
				<ShieldCheck className="mt-0.5 size-5 shrink-0 text-muted-foreground" strokeWidth={1.75} />
				<div className="space-y-1 text-[12.5px] leading-[19px] text-muted-foreground">
					<p>
						<span className="font-semibold text-foreground">The wording sounds serious, and that's normal.</span>{' '}
						macOS says the same thing about every keyboard tool: that it could monitor your keyboard and
						control your Mac.
					</p>
					<p>
						Chord only responds to keys while you hold its shortcut. It doesn't record what you type, and
						it never sends it anywhere. You can turn this off again at any time on the same page.
					</p>
				</div>
			</div>
		</StepLayout>
	);
}
