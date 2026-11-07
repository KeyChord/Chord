import { Button } from '@chord/dev.improve.chord.components.ui.button';
import { AppModeState, type ChordInputState } from '@chord/dev.improve.chord.lib.typeshare';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { PartyPopper } from 'lucide-react';
import { useEffect, useReducer } from 'react';
import { Key, KeyboardMap } from '#keyboard-map';
import {
	advancePractice,
	currentTask,
	initialPractice,
	type PracticeEvent,
	type PracticeProgress,
	type PracticeTask,
} from '#practice';
import { Click, Instruction, StepLayout } from '#step-layout';

/** Long enough to see the app open before setup comes back for the next step. */
const SHOW_RESULT_MS = 1_600;

function usePractice(target: string) {
	const [progress, dispatch] = useReducer(
		(state: PracticeProgress, event: PracticeEvent) => advancePractice(state, event, target),
		initialPractice,
	);
	useEffect(() => {
		const subscriptions = [
			listen<AppModeState>('state:app-mode', ({ payload }) => {
				dispatch({ type: 'mode', chord: payload === AppModeState.Chord });
			}),
			listen<ChordInputState>('state:chord-input', ({ payload }) => {
				dispatch({
					type: 'input',
					input: payload.input,
					executed: payload.selectedInputEvent?.input ?? null,
					error: payload.error ?? null,
				});
			}),
		];
		return () => {
			for (const subscription of subscriptions) void subscription.then(unlisten => unlisten());
		};
	}, []);
	useEffect(() => {
		if (!progress.ran) return;
		const timer = setTimeout(() => void getCurrentWindow().setFocus(), SHOW_RESULT_MS);
		return () => clearTimeout(timer);
	}, [progress.ran]);
	return progress;
}

function highlightedKeys(progress: PracticeProgress, task: PracticeTask, keys: string[]) {
	if (progress.ran) return [];
	if (task === 'hold') return ['caps', 'space'];
	return task === 'type' ? [...keys, 'space'] : ['space'];
}

function pressedKeys({ inChordMode, typed }: PracticeProgress) {
	return inChordMode ? ['space', ...typed] : [];
}

export function PracticeView({
	progress,
	sequence,
	appName,
}: {
	progress: PracticeProgress;
	sequence: string;
	appName: string;
}) {
	const task = currentTask(progress, sequence);
	const keys = [...sequence];
	const stateOf = (step: 'hold' | 'type' | 'release') => {
		if (progress.ran) return 'done';
		const order = ['hold', 'type', 'release'];
		const [current, own] = [order.indexOf(task), order.indexOf(step)];
		return own < current ? 'done' : own === current ? 'current' : 'todo';
	};
	return (
		<>
			<KeyboardMap
				highlighted={highlightedKeys(progress, task, keys)}
				pressed={pressedKeys(progress)}
			/>
			<ol className="space-y-3">
				<Instruction number={1} state={stateOf('hold')}>
					Press <Key>caps lock</Key> and <Key>space</Key> together. Let go of caps lock but <Click>keep holding space</Click>.
				</Instruction>
				<Instruction number={2} state={stateOf('type')}>
					Still holding space, type {keys.map(key => <Key key={key}>{key}</Key>)}
					{progress.typed && !progress.ran
						? <span className="ml-2 text-muted-foreground">You've typed: <span className="font-medium text-foreground">{progress.typed}</span></span>
						: null}
				</Instruction>
				<Instruction number={3} state={stateOf('release')}>
					Let go of <Key>space</Key> to open {appName}.
				</Instruction>
			</ol>
			<div aria-live="polite" className="min-h-[58px] text-[13px] leading-[19px]">
				{progress.ran
					? (
							<p className="flex items-center gap-2 rounded-[10px] bg-emerald-500/10 p-3 font-semibold text-emerald-700 dark:text-emerald-400">
								<PartyPopper className="size-4 shrink-0" />
								That's a chord! You just opened {appName} without touching your mouse.
							</p>
						)
					: progress.hint === 'released-too-soon'
						? (
								<p className="rounded-[10px] bg-amber-500/10 p-3 text-amber-800 dark:text-amber-300">
									You let go of space before typing. Press caps lock and space again, and keep holding space until you've typed the letters.
								</p>
							)
						: progress.hint === 'did-not-run'
							? (
									<p className="rounded-[10px] bg-amber-500/10 p-3 text-amber-800 dark:text-amber-300">
										You typed <span className="font-semibold">{sequence}</span> correctly, but Chord couldn't run it{progress.error ? <>: {progress.error}</> : null}. You can skip this for now and check the chord in Settings.
									</p>
								)
							: progress.hint === 'wrong-keys'
								? (
										<p className="rounded-[10px] bg-amber-500/10 p-3 text-amber-800 dark:text-amber-300">
											That was <span className="font-semibold">{progress.typed}</span>, not <span className="font-semibold">{sequence}</span>. No harm done, so give it another try.
										</p>
									)
								: progress.inChordMode
									? <p className="p-3 text-muted-foreground">While you hold space, a list of your chords shows up on the screen, so you never have to memorize them.</p>
									: <p className="p-3 text-muted-foreground">Don't worry, caps lock still works. Tap it by itself to turn capital letters on or off.</p>}
			</div>
		</>
	);
}

export function PracticeStep({
	sequence,
	appName,
	stepNumber,
	stepCount,
	onContinue,
}: {
	sequence: string;
	appName: string;
	stepNumber: number;
	stepCount: number;
	onContinue: () => void;
}) {
	const progress = usePractice(sequence);
	return (
		<StepLayout
			stepNumber={stepNumber}
			stepCount={stepCount}
			title={progress.ran ? 'You did it!' : 'Now try it'}
			description={progress.ran ? undefined : `Your chord for ${appName} is ready. Follow these steps, and we'll watch for it.`}
			actions={(
				<Button type="button" size="lg" variant={progress.ran ? 'default' : 'outline'} onClick={onContinue}>
					{progress.ran ? 'Continue' : 'Skip for Now'}
				</Button>
			)}
		>
			<PracticeView progress={progress} sequence={sequence} appName={appName} />
		</StepLayout>
	);
}
