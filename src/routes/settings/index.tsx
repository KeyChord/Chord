import { taurpc } from '#/api/taurpc.ts';
import { BrowseTab } from '#/components/settings/browse-tab.tsx';
import { ChordsTab } from '#/components/settings/chords-tab.tsx';
import { ConfigureTab } from '#/components/settings/configure-tab.tsx';
import { DangerTab } from '#/components/settings/danger-tab.tsx';
import { FirstRunOnboarding } from '#/components/settings/first-run-onboarding.tsx';
import { GlobalShortcutsTab } from '#/components/settings/global-shortcuts-tab.tsx';
import { SettingsTab } from '#/components/settings/settings-tab.tsx';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '#/components/ui/tabs.tsx';
import { TanStackDevtools } from '@tanstack/react-devtools';
import { useQuery } from '@tanstack/react-query';
import { createFileRoute } from '@tanstack/react-router';
import { TanStackRouterDevtoolsPanel } from '@tanstack/react-router-devtools';
import { useState } from 'react';
import { useChordPackageManagerState } from '../../utils/state.ts';

export const Route = createFileRoute('/settings/')({
	component: Settings,
});

function Settings() {
	const [dismissedOnboarding, setDismissedOnboarding] = useState(false);
	const startupStatusQuery = useQuery({
		queryKey: ['startup-status'],
		queryFn: taurpc.getStartupStatus,
	});
	const shouldShowOnboarding
		= startupStatusQuery.data?.shouldShowOnboarding === true && !dismissedOnboarding;
    console.log(useChordPackageManagerState())

	return (
		<>
			{startupStatusQuery.isLoading
				? (
						<div className="flex min-h-full items-center justify-center bg-muted/30 px-5 py-4 text-sm text-muted-foreground">
							Loading settings...
						</div>
					)
				: shouldShowOnboarding
					? (
							<FirstRunOnboarding
								onSkip={() => {
									setDismissedOnboarding(true);
								}}
								onComplete={() => {
									setDismissedOnboarding(true);
									void startupStatusQuery.refetch();
								}}
							/>
						)
					: (
							<div className="min-h-full bg-muted/30 px-5 py-4 text-sm text-foreground">
								<div className="mx-auto flex max-w-[720px] flex-col gap-4">
									<div className="flex items-start justify-between gap-3">
										<div>
											<h1 className="text-[20px] font-semibold">Settings</h1>
											<p className="mt-1 text-muted-foreground">
												Configure permissions, manage chord sources, assign placeholder chords, and review the app's shortcuts.
											</p>
										</div>
									</div>

									<Tabs defaultValue="general" className="gap-4">
										<TabsList className="h-auto w-full flex-wrap justify-start gap-2 rounded-2xl bg-transparent p-0">
											<TabsTrigger
												value="general"
												className="h-auto flex-none rounded-2xl border border-border bg-background px-4 py-2.5 text-sm data-active:border-foreground/15 data-active:bg-background data-active:shadow-sm"
											>
												General
											</TabsTrigger>
											<TabsTrigger
												value="chords"
												className="h-auto flex-none rounded-2xl border border-border bg-background px-4 py-2.5 text-sm data-active:border-foreground/15 data-active:bg-background data-active:shadow-sm"
											>
												Chords
											</TabsTrigger>
											<TabsTrigger
												value="browse"
												className="h-auto flex-none rounded-2xl border border-border bg-background px-4 py-2.5 text-sm data-active:border-foreground/15 data-active:bg-background data-active:shadow-sm"
											>
												Browse
											</TabsTrigger>
											<TabsTrigger
												value="configure"
												className="h-auto flex-none rounded-2xl border border-border bg-background px-4 py-2.5 text-sm data-active:border-foreground/15 data-active:bg-background data-active:shadow-sm"
											>
												Configure
											</TabsTrigger>
											<TabsTrigger
												value="shortcuts"
												className="h-auto flex-none rounded-2xl border border-border bg-background px-4 py-2.5 text-sm data-active:border-foreground/15 data-active:bg-background data-active:shadow-sm"
											>
												Shortcuts
											</TabsTrigger>
											<TabsTrigger
												value="danger"
												className="h-auto flex-none rounded-2xl border border-border bg-background px-4 py-2.5 text-sm data-active:border-foreground/15 data-active:bg-background data-active:shadow-sm"
											>
												Danger
											</TabsTrigger>
										</TabsList>

										<TabsContent value="general">
											<SettingsTab />
										</TabsContent>

										<TabsContent value="chords">
											<ChordsTab />
										</TabsContent>

										<TabsContent value="browse">
											<BrowseTab />
										</TabsContent>

										<TabsContent value="configure">
											<ConfigureTab />
										</TabsContent>

										<TabsContent value="shortcuts">
											<GlobalShortcutsTab />
										</TabsContent>

										<TabsContent value="danger">
											<DangerTab />
										</TabsContent>
									</Tabs>
								</div>
							</div>
						)}
			{import.meta.env.DEV
				? (
						<TanStackDevtools
							config={{
								position: 'bottom-right',
							}}
							plugins={[
								{
									name: 'TanStack Router',
									render: <TanStackRouterDevtoolsPanel />,
								},
							]}
						/>
					)
				: null}
		</>
	);
}
