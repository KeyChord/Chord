import { taurpc } from '@chord/dev.improve.chord.api.taurpc';
import { createFileRoute, redirect } from '@tanstack/react-router';
import { getCurrentWindow } from '@tauri-apps/api/window';

export const Route = createFileRoute('/')({
	loader: async () => {
		const label = getCurrentWindow().label;

		if (label === 'settings') {
			const states = JSON.parse(await taurpc.getCurrentStates());
			throw redirect({ to: states.settings.isOnboardingComplete ? '/settings' : '/onboarding' });
		}

		throw redirect({ to: '/chords' });
	},
});
