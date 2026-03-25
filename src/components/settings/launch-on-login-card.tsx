import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from '#/components/ui/card.tsx';
import { Checkbox } from '#/components/ui/checkbox.tsx';
import { Label } from '#/components/ui/label.tsx';
import { useMutation } from '@tanstack/react-query';
import { taurpc } from '../../api/taurpc.ts';
import { usePermissionsState, useSettingsState } from '../../utils/state.ts';

export function LaunchOnLoginCard() {
	const permissions = usePermissionsState();
	const settings = useSettingsState();
	const toggleAutostartMutation = useMutation({
		mutationFn: taurpc.toggleAutostart,
	});
	const toggleMenuBarIconMutation = useMutation({
		mutationFn: taurpc.toggleMenuBarIcon,
	});
	const toggleDockIconMutation = useMutation({
		mutationFn: taurpc.toggleDockIcon,
	});
	const toggleHideGuideByDefaultMutation = useMutation({
		mutationFn: taurpc.toggleHideGuideByDefault,
	});

	return (
		<Card size="sm">
			<CardHeader>
				<CardTitle>Startup & Visibility</CardTitle>
				<CardDescription>
					Control how Chord starts, where its icon appears, and whether the guide is shown by
					default.
				</CardDescription>
			</CardHeader>
			<CardContent className="pt-0">
				<div className="overflow-hidden rounded-lg border bg-background/80">
					<SettingCheckboxRow
						id="launch-on-login"
						label="Launch on Login"
						description="Start Chord automatically after you sign in. The app launches hidden and reuses a single instance."
						checked={permissions.isAutostartEnabled === true}
						disabled={toggleAutostartMutation.isPending}
						onCheckedChange={() => {
							toggleAutostartMutation.mutate();
						}}
					/>
					<SettingCheckboxRow
						id="menu-bar-show-icon"
						label="Menu bar: Show Icon"
						description="Show the menu bar icon so settings and reload actions stay available from the tray."
						checked={settings.showMenuBarIcon}
						disabled={toggleMenuBarIconMutation.isPending}
						onCheckedChange={() => {
							toggleMenuBarIconMutation.mutate();
						}}
					/>
					<SettingCheckboxRow
						id="show-dock-icon"
						label="Show Dock Icon"
						description="Keep Chord visible in the Dock while it is running."
						checked={settings.showDockIcon}
						disabled={toggleDockIconMutation.isPending}
						onCheckedChange={() => {
							toggleDockIconMutation.mutate();
						}}
					/>
					<SettingCheckboxRow
						id="hide-guide-by-default"
						label="Hide Guide by default"
						description="Start chord mode with the guide hidden. Press Tab to reveal it when needed."
						checked={settings.hideGuideByDefault}
						disabled={toggleHideGuideByDefaultMutation.isPending}
						onCheckedChange={() => {
							toggleHideGuideByDefaultMutation.mutate();
						}}
					/>
				</div>
			</CardContent>
		</Card>
	);
}

function SettingCheckboxRow({
	id,
	label,
	description,
	checked,
	disabled,
	onCheckedChange,
}: {
	id: string
	label: string
	description: string
	checked: boolean
	disabled?: boolean
	onCheckedChange: () => void
}) {
	return (
		<div className="flex items-start gap-3 border-b px-3 py-3 last:border-b-0">
			<Checkbox
				id={id}
				checked={checked}
				disabled={disabled}
				onCheckedChange={() => {
					onCheckedChange();
				}}
			/>
			<div className="space-y-1">
				<Label htmlFor={id}>{label}</Label>
				<p className="text-sm text-muted-foreground">{description}</p>
			</div>
		</div>
	);
}
