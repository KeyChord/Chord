import { SettingsChordsPage } from '@chord/dev.improve.chord.routes.settings.chords.index';
import { createFileRoute } from '@tanstack/react-router';

export const Route = createFileRoute('/settings/chords/')({
	component: SettingsChordsPage,
});
