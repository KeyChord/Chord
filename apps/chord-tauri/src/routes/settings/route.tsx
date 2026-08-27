import { SettingsPage } from '@chord/dev.improve.chord.routes.settings';
import { createFileRoute } from '@tanstack/react-router';

export const Route = createFileRoute('/settings')({
	component: SettingsPage,
});
