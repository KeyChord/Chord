import { OnboardingPage } from '@chord/dev.improve.chord.routes.settings.onboarding.index';
import { createFileRoute } from '@tanstack/react-router';

export const Route = createFileRoute('/onboarding/')({
	component: OnboardingPage,
});
