import { useMutation } from '@chord/com.npmjs.tanstack__react-query';
import { taurpc } from '@chord/dev.improve.chord.api.taurpc';
import { Button } from '@chord/dev.improve.chord.components.ui.button';
import { Switch } from '@chord/dev.improve.chord.components.ui.switch';
import { usePermissionsState } from '@chord/dev.improve.chord.lib.state';
import { TriangleAlert } from 'lucide-react';
import { useState } from 'react';
import { Key } from '#keyboard-map';
import { StepLayout } from '#step-layout';

function Tip({ title, children }: { title: string; children: React.ReactNode }) {
	return (
		<div className="px-4 py-2.5">
			<p className="text-[13.5px] font-medium leading-[20px] text-foreground">{title}</p>
			<p className="text-[12.5px] leading-[21px] text-muted-foreground">{children}</p>
		</div>
	);
}

export function DoneStep({
	isPermissionGranted,
	onFinish,
}: {
	isPermissionGranted: boolean;
	onFinish: (startAtLogin: boolean) => void;
}) {
	const [startAtLogin, setStartAtLogin] = useState(true);
	return (
		<StepLayout
			title="You're all set"
			description="Here's what you'll want to remember."
			actions={(
				<Button type="button" size="lg" onClick={() => onFinish(startAtLogin)}>
					Finish
				</Button>
			)}
		>
			{isPermissionGranted
				? null
				: (
						<div role="alert" className="flex gap-2 rounded-[10px] bg-amber-500/10 p-3 text-[13px] leading-[19px] text-amber-800 dark:text-amber-300">
							<TriangleAlert className="mt-0.5 size-4 shrink-0" />
							Chords won't work until you allow Chord to use your keyboard. Chord will ask again when you finish.
						</div>
					)}
			<div className="divide-y divide-black/[0.06] rounded-[12px] border border-black/[0.08] dark:divide-white/10 dark:border-white/10">
				<Tip title="Use any chord">
					Press <Key>caps lock</Key> and <Key>space</Key> together. Let go of caps lock but keep holding space, type its letters, then let go of space.
				</Tip>
				<Tip title="Forgot a chord?">
					Press <Key>caps lock</Key> and <Key>space</Key> together, keep holding space, and look at the list on your screen. It shows the chords for whichever app you're using.
				</Tip>
				<Tip title="Open Chord's settings">
					Use the chord <Key>0</Key> <Key>,</Key> or click the Chord icon at the top of your screen, in the menu bar.
				</Tip>
			</div>
			<label className="flex items-center justify-between gap-4 rounded-[12px] border border-black/[0.08] px-4 py-3 dark:border-white/10">
				<span>
					<span className="block text-[13.5px] font-medium leading-[20px] text-foreground">Start Chord when you log in</span>
					<span className="block text-[12.5px] leading-[18px] text-muted-foreground">So your chords are always ready. macOS may show a notice that Chord runs in the background. That's expected.</span>
				</span>
				<Switch aria-label="Start Chord when you log in" checked={startAtLogin} onCheckedChange={setStartAtLogin} />
			</label>
		</StepLayout>
	);
}

/** Saves the choice and hands over to the regular settings. */
export function useFinishOnboarding(onFinished: () => void) {
	const { data: permissions } = usePermissionsState();
	return useMutation({
		mutationFn: async (startAtLogin: boolean) => {
			// Setup is over whatever happens next; a failure below shows up as an error message.
			await taurpc.setOnboardingComplete(true);
			if (startAtLogin !== Boolean(permissions?.isAutostartEnabled)) {
				await taurpc.toggleAutostart();
			}
		},
		onSettled: onFinished,
	});
}
