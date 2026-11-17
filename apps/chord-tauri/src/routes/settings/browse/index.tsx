import { SettingsBrowsePage } from '@chord/dev.improve.chord.routes.settings.browse.index';
import { createFileRoute } from '@tanstack/react-router';

export const Route = createFileRoute('/settings/browse/')({
	component: BrowseRoute,
});

function BrowseRoute() {
	const navigate = Route.useNavigate();
	return (
		<SettingsBrowsePage
			onConfigure={(packageName) => {
				void navigate({ to: '/settings/chords', search: { package: packageName } });
			}}
		/>
	);
}
