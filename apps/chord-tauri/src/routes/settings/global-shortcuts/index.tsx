import { SettingsGlobalShortcutsPage } from '@chord/dev.improve.chord.routes.settings.global-shortcuts.index';
import { createFileRoute } from '@tanstack/react-router';

export const Route = createFileRoute('/settings/global-shortcuts/')({
	component: SettingsGlobalShortcutsPage,
});
