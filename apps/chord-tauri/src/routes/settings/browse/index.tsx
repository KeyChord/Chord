import { SettingsBrowsePage } from '@chord/dev.improve.chord.routes.settings.browse.index';
import { createFileRoute } from '@tanstack/react-router';

export const Route = createFileRoute('/settings/browse/')({
	component: SettingsBrowsePage,
});
