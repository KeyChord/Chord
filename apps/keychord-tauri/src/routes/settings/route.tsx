import { createFileRoute } from '@tanstack/react-router'
import { SettingsPage } from '@keychord/dev.improve.keychord.routes.settings';

export const Route = createFileRoute('/settings')({
	component: SettingsPage,
});
