import { createFileRoute } from '@tanstack/react-router'
import { SettingsDangerPage } from '@keychord/dev.improve.keychord.routes.settings.danger.index';

export const Route = createFileRoute('/settings/danger/')({
	component: SettingsDangerPage,
});
