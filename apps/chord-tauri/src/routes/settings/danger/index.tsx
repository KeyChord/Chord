import { SettingsDangerPage } from '@chord/dev.improve.chord.routes.settings.danger.index';
import { createFileRoute } from '@tanstack/react-router';

export const Route = createFileRoute('/settings/danger/')({
	component: SettingsDangerPage,
});
