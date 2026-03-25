import { AppIcon } from '#/components/settings/app-icon.tsx';
import { Badge } from '#/components/ui/badge.tsx';
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from '#/components/ui/card.tsx';
import { useSettingsState } from '../../utils/state.ts';
import { RelaunchAppButton } from './relaunch-app-button.tsx';

export function AppsNeedingRelaunchCard() {
	const { bundleIdsNeedingRelaunch } = useSettingsState();

	return (
		<Card size="sm">
			<CardHeader className="flex items-center justify-between gap-3">
				<CardTitle>Apps Needing Relaunch</CardTitle>
				<CardDescription>
					Scripts can flag apps that should be restarted after they change app state.
				</CardDescription>
			</CardHeader>
			<CardContent className="space-y-2 pt-0">
				{bundleIdsNeedingRelaunch.length === 0
					? (
							<p className="text-sm text-muted-foreground">
								No apps are currently marked as needing a relaunch.
							</p>
						)
					: (
							bundleIdsNeedingRelaunch.map((bundleId) => {
								return <AppNeedingRelaunchRow key={bundleId} app={{ bundleId }} />;
							})
						)}
			</CardContent>
		</Card>
	);
}

function AppNeedingRelaunchRow({ app }: { app: { bundleId: string } }) {
	const appLabel = app.bundleId;

	return (
		<div className="flex items-center justify-between gap-3 rounded-lg border bg-background/80 px-3 py-2">
			<div className="flex min-w-0 items-center gap-2">
				<AppIcon label={appLabel} />
				<div className="min-w-0">
					<div className="flex items-center gap-2">
						<p className="truncate font-medium">{appLabel}</p>
						<Badge variant="secondary">Needs relaunch</Badge>
					</div>
				</div>
			</div>

			<RelaunchAppButton app={app} />
		</div>
	);
}
