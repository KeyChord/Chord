import { createFileRoute } from '@tanstack/react-router'
import { SettingsBrowsePage } from '@keychord/dev.improve.keychord.routes.settings.browse.index';

export const Route = createFileRoute('/settings/browse/')({
	component: SettingsBrowsePage,
});
