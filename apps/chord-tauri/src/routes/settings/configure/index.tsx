import { SettingsConfigurePage } from '@chord/dev.improve.chord.routes.settings.configure.index';
import { createFileRoute } from '@tanstack/react-router';

export const Route = createFileRoute('/settings/configure/')({
	component: SettingsConfigurePage,
});
