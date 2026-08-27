import { ActivationTriggerCard } from '@chord/dev.improve.chord.routes.settings._components.activation-trigger-card';
import { LaunchOnLoginCard } from '@chord/dev.improve.chord.routes.settings._components.launch-on-login-card';
import { PermissionsCard } from '@chord/dev.improve.chord.routes.settings._components.permissions-card';
import { QuitChordCard } from '@chord/dev.improve.chord.routes.settings._components.quit-chord-card';

export function SettingsGeneralPage() {
	return (
		<div className="space-y-4">
			<PermissionsCard />
			<ActivationTriggerCard />
			<LaunchOnLoginCard />
			<QuitChordCard />
		</div>
	);
}
