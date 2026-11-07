import { useMutation, useQuery } from '@chord/com.npmjs.tanstack__react-query';
import { useNavigate } from '@chord/com.npmjs.tanstack__react-router';
import { taurpc } from '@chord/dev.improve.chord.api.taurpc';
import { usePermissionsState } from '@chord/dev.improve.chord.lib.state';
import { useEffect, useState } from 'react';
import { DoneStep, useFinishOnboarding } from '#step-done';
import { type CreatedChord, FirstChordStep } from '#step-first-chord';
import { MoveStep } from '#step-move';
import { PacksStep } from '#step-packs';
import { PermissionStep } from '#step-permission';
import { PracticeStep } from '#step-practice';
import { WelcomeStep } from '#step-welcome';

type Step = 'welcome' | 'permission' | 'first-chord' | 'practice' | 'packs' | 'done';

/** The numbered steps after the welcome screen. */
const NUMBERED_STEPS: Step[] = ['permission', 'first-chord', 'practice', 'packs'];

export function OnboardingPage() {
	const navigate = useNavigate();
	const info = useQuery({
		queryKey: ['onboarding-info'],
		queryFn: () => taurpc.getOnboardingInfo(),
		staleTime: Number.POSITIVE_INFINITY,
	});
	const { data: permissions } = usePermissionsState();
	const isGranted = permissions?.isAccessibilityEnabled === true;
	const [step, setStep] = useState<Step>('welcome');
	const [movePostponed, setMovePostponed] = useState(false);
	const [chord, setChord] = useState<CreatedChord>();

	// The grant happens in System Settings. Chord's state hears about it, and this moves along.
	useEffect(() => {
		if (step === 'permission' && isGranted) setStep('first-chord');
	}, [step, isGranted]);

	const finish = useFinishOnboarding(() => void navigate({ to: '/settings/chords' }));
	const skip = useMutation({
		mutationFn: () => taurpc.setOnboardingComplete(true),
		onSettled: () => void navigate({ to: '/settings/general' }),
	});
	const skipSetup = () => {
		skip.mutate();
	};
	const numbers = (current: Step) => ({
		stepNumber: NUMBERED_STEPS.indexOf(current) + 1,
		stepCount: NUMBERED_STEPS.length,
	});

	if (info.isPending) return <div className="settings-content-surface h-full w-full bg-background" />;
	if (info.data?.needsMoveToApplications && !movePostponed) {
		return <MoveStep onSkip={() => setMovePostponed(true)} />;
	}
	switch (step) {
		case 'welcome':
			return (
				<WelcomeStep
					onContinue={() => setStep(isGranted ? 'first-chord' : 'permission')}
					onSkip={skipSetup}
				/>
			);
		case 'permission':
			return (
				<PermissionStep
					{...numbers('permission')}
					paneName={info.data?.accessibilityPaneName ?? 'Accessibility'}
					onSkip={() => setStep('packs')}
				/>
			);
		case 'first-chord':
			return (
				<FirstChordStep
					{...numbers('first-chord')}
					onCreated={(created) => {
						setChord(created);
						setStep('practice');
					}}
					onSkip={() => setStep('packs')}
				/>
			);
		case 'practice':
			return (
				<PracticeStep
					{...numbers('practice')}
					sequence={chord?.sequence ?? ''}
					appName={chord?.appName ?? ''}
					onContinue={() => setStep('packs')}
				/>
			);
		case 'packs':
			return <PacksStep {...numbers('packs')} onContinue={() => setStep('done')} />;
		case 'done':
			return <DoneStep isPermissionGranted={isGranted} onFinish={startAtLogin => finish.mutate(startAtLogin)} />;
	}
}
