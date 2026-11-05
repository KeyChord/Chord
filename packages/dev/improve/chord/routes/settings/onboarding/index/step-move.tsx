import { useMutation } from '@chord/com.npmjs.tanstack__react-query';
import { taurpc } from '@chord/dev.improve.chord.api.taurpc';
import { Button } from '@chord/dev.improve.chord.components.ui.button';
import { Click, StepLayout } from '#step-layout';

function errorMessage(error: unknown) {
	if (error instanceof Error) return error.message;
	if (error && typeof error === 'object' && 'Message' in error) return String(error.Message);
	return String(error);
}

/** Shown when Chord is running from the disk image it was downloaded on. */
export function MoveStep({ onSkip }: { onSkip: () => void }) {
	const move = useMutation({ mutationFn: () => taurpc.moveToApplications() });
	return (
		<StepLayout
			title="Let's finish installing Chord"
			description="Chord is running from the installer, and it would stop working as soon as the installer closes. Moving it to your Applications folder only takes a click, and Chord will reopen by itself."
			actions={(
				<Button type="button" size="lg" onClick={() => move.mutate()} disabled={move.isPending}>
					{move.isPending ? 'Moving…' : 'Move to Applications'}
				</Button>
			)}
			secondary={(
				<Button type="button" variant="ghost" onClick={onSkip} disabled={move.isPending}>
					Not Now
				</Button>
			)}
		>
			{move.isError
				? (
						<div role="alert" className="space-y-1 rounded-[12px] border border-red-500/30 bg-red-500/[0.06] p-4 text-[13.5px] leading-[21px]">
							<p className="font-semibold">Chord couldn't move itself.</p>
							<p className="text-muted-foreground">
								In the installer window, drag <Click>Chord</Click> onto the <Click>Applications</Click> folder,
								then open Chord from your Applications folder.
							</p>
							<p className="text-[12px] text-muted-foreground">{errorMessage(move.error)}</p>
						</div>
					)
				: null}
		</StepLayout>
	);
}
