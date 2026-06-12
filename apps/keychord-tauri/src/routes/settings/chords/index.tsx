import { createFileRoute } from '@tanstack/react-router'
import { SettingsChordsPage } from '@keychord/dev.improve.keychord.routes.settings.chords.index';

export const Route = createFileRoute('/settings/chords/')({
	component: SettingsChordsPage,
});
