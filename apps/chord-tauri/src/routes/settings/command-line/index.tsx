import { SettingsCommandLinePage } from '@chord/dev.improve.chord.routes.settings.command-line.index';
import { createFileRoute } from '@tanstack/react-router';

export const Route = createFileRoute('/settings/command-line/')({
	component: SettingsCommandLinePage,
});
