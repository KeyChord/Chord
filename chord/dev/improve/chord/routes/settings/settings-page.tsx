import { TanStackDevtools } from '@chord/com.npmjs.tanstack__react-devtools';
import { Link, Outlet, useRouterState } from '@chord/com.npmjs.tanstack__react-router';
import { TanStackRouterDevtoolsPanel } from '@chord/com.npmjs.tanstack__react-router-devtools';
import {
	Sidebar,
	SidebarContent,
	SidebarGroup,
	SidebarGroupContent,
	SidebarGroupLabel,
	SidebarHeader,
	SidebarInset,
	SidebarMenu,
	SidebarMenuButton,
	SidebarMenuItem,
	SidebarProvider,
} from '@chord/dev.improve.chord.components.ui.sidebar';
import { PermissionsDialog } from '@chord/dev.improve.chord.routes.settings._components.permissions-dialog';
import {
	Command,
	Compass,
	Keyboard,
	Settings2,
	SlidersHorizontal,
	TriangleAlert,
} from 'lucide-react';
import { useEffect, useRef, useState } from 'react';

const navigationGroups = [
	{
		label: 'Configuration',
		items: [
			{
				label: 'General',
				to: '/settings/general',
				icon: Settings2,
			},
			{
				label: 'Chords',
				to: '/settings/chords',
				icon: Keyboard,
			},
			{
				label: 'Browse',
				to: '/settings/browse',
				icon: Compass,
			},
			{
				label: 'Configure',
				to: '/settings/configure',
				icon: SlidersHorizontal,
			},
			{
				label: 'Global Shortcuts',
				to: '/settings/global-shortcuts',
				icon: Command,
			},
		],
	},
	{
		label: 'Maintenance',
		items: [
			{
				label: 'Danger Zone',
				to: '/settings/danger',
				icon: TriangleAlert,
			},
		],
	},
] as const;

function SettingsShell({ children }: { children: React.ReactNode }) {
	const pathname = useRouterState({
		select: state => state.location.pathname,
	});
	const scrollContainerRef = useRef<HTMLDivElement>(null);

	useEffect(() => {
		scrollContainerRef.current?.scrollTo({ top: 0 });
	}, [pathname]);

	return (
		<SidebarProvider className="settings-window-shell min-h-0 h-full" style={{ '--sidebar-width': '15.5rem' } as React.CSSProperties}>
			<Sidebar
				collapsible="none"
				className="settings-window-sidebar shrink-0 border-r border-black/[0.08] bg-transparent text-sidebar-foreground"
			>
				<SidebarHeader
					data-tauri-drag-region
					className="h-[50px] shrink-0 p-0"
				/>

				<SidebarContent className="px-2.5 pb-4 pt-1">
					{navigationGroups.map(group => (
						<SidebarGroup key={group.label} className="px-0 py-2">
							<SidebarGroupLabel className="h-6 px-2.5 text-[11px] font-semibold tracking-[0.01em] text-sidebar-foreground/40">
								{group.label}
							</SidebarGroupLabel>
							<SidebarGroupContent>
								<SidebarMenu className="gap-0.5">
									{group.items.map((item) => {
										const isActive = pathname === item.to || pathname === `${item.to}/`;
										const Icon = item.icon;

										return (
											<SidebarMenuItem key={item.to}>
												<SidebarMenuButton
													asChild
													isActive={isActive}
													className="h-8 rounded-[8px] px-2.5 text-[13px] font-medium text-sidebar-foreground/85 hover:bg-black/[0.05] data-active:bg-[#3478f6] data-active:text-white data-active:shadow-[inset_0_0_0_0.5px_rgba(0,0,0,0.08)] data-active:hover:bg-[#3478f6] data-active:hover:text-white"
												>
													<Link to={item.to}>
														<Icon strokeWidth={1.8} />
														<span>{item.label}</span>
													</Link>
												</SidebarMenuButton>
											</SidebarMenuItem>
										);
									})}
								</SidebarMenu>
							</SidebarGroupContent>
						</SidebarGroup>
					))}
				</SidebarContent>
			</Sidebar>

			<SidebarInset className="settings-content-surface min-h-0 min-w-0 overflow-hidden bg-background">
				<header
					data-tauri-drag-region
					className="flex h-[62px] shrink-0 items-center px-7"
				>
					<h1 className="text-[18px] font-semibold tracking-[-0.018em]">Chord Settings</h1>
				</header>
				<div
					ref={scrollContainerRef}
					className="settings-page-scroll min-h-0 flex-1 overflow-y-auto bg-background"
				>
					<div className="settings-page-content w-full px-7 pb-10 pt-3">
						{children}
					</div>
				</div>
			</SidebarInset>
		</SidebarProvider>
	);
}

export function SettingsPage() {
	const [permissionDialogDismissed, setPermissionDialogDismissed] = useState(false);

	return (
		<>
			<SettingsShell>
				<Outlet />
			</SettingsShell>
			{!permissionDialogDismissed
				? (
						<PermissionsDialog
							onDismiss={() => {
								setPermissionDialogDismissed(true);
							}}
						/>
					)
				: null}
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
