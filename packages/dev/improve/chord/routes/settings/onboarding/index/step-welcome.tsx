import { Button } from '@chord/dev.improve.chord.components.ui.button';
import { ArrowRight } from 'lucide-react';
import { Key } from '#keyboard-map';
import { StepLayout } from '#step-layout';

export function WelcomeStep({ onContinue, onSkip }: { onContinue: () => void; onSkip: () => void }) {
	return (
		<StepLayout
			title="Welcome to Chord"
			description="Type a couple of letters to open an app or press a shortcut, from anywhere on your Mac."
			actions={(
				<Button type="button" size="lg" onClick={onContinue}>
					Get Started
					<ArrowRight />
				</Button>
			)}
			secondary={(
				<Button type="button" variant="ghost" onClick={onSkip}>
					Skip Setup
				</Button>
			)}
		>
			<div className="rounded-[12px] border border-black/[0.08] bg-black/[0.02] p-4 text-[13.5px] leading-[24px] text-foreground dark:border-white/10 dark:bg-white/[0.03]">
				<p className="mb-1 font-semibold">A chord looks like this:</p>
				<p>
					Hold <Key>caps lock</Key> + <Key>space</Key>, type <Key>/</Key> <Key>s</Key>, and let go.
				</p>
				<p className="text-muted-foreground">Safari opens. That's it.</p>
			</div>
			<p className="text-[13.5px] leading-[21px] text-muted-foreground">
				Setup takes about two minutes. We'll tell you what to click at every step, and by the end
				you'll have made a chord of your own.
			</p>
		</StepLayout>
	);
}
