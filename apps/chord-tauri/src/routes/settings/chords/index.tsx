import { SettingsChordsPage } from '@chord/dev.improve.chord.routes.settings.chords.index';
import { createFileRoute } from '@tanstack/react-router';

export const Route = createFileRoute('/settings/chords/')({
	validateSearch: (search: Record<string, unknown>): { package?: string } =>
		typeof search.package === 'string' ? { package: search.package } : {},
	component: ChordsRoute,
});

function ChordsRoute() {
	const { package: configurePackage } = Route.useSearch();
	return <SettingsChordsPage key={configurePackage} configurePackage={configurePackage} />;
}
