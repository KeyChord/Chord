import { createFileRoute } from '@tanstack/react-router'
import { SettingsGlobalShortcutsPage } from '@keychord/dev.improve.keychord.routes.settings.global-shortcuts.index';

export const Route = createFileRoute('/settings/global-shortcuts/')({
	component: SettingsGlobalShortcutsPage,
});
