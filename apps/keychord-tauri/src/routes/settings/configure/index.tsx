import { createFileRoute } from '@tanstack/react-router'
import { SettingsConfigurePage } from '@keychord/dev.improve.keychord.routes.settings.configure.index';

export const Route = createFileRoute('/settings/configure/')({
	component: SettingsConfigurePage,
});
