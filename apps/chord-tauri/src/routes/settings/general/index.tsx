import { SettingsGeneralPage } from '@chord/dev.improve.chord.routes.settings.general.index';
import { createFileRoute } from '@tanstack/react-router';

export const Route = createFileRoute('/settings/general/')({
	component: SettingsGeneralPage,
});
