import { createFileRoute } from '@tanstack/react-router'
import { SettingsGeneralPage } from '@keychord/dev.improve.keychord.routes.settings.general.index';

export const Route = createFileRoute('/settings/general/')({
	component: SettingsGeneralPage,
});
