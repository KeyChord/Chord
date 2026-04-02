import { createFileRoute } from '@tanstack/react-router';
import { ActivationTriggerCard } from '#/components/settings/activation-trigger-card.tsx';
import { LaunchOnLoginCard } from '#/components/settings/launch-on-login-card.tsx';
import { PermissionsCard } from '#/components/settings/permissions-card.tsx';
import { QuitChordCard } from '#/components/settings/quit-chord-card.tsx';

export const Route = createFileRoute('/settings/general/')({
  component: SettingsGeneralPage,
});

function SettingsGeneralPage() {
	return (
		<div className="space-y-4">
			<PermissionsCard />
			<ActivationTriggerCard />
			<LaunchOnLoginCard />
			<QuitChordCard />
		</div>
	);
}
