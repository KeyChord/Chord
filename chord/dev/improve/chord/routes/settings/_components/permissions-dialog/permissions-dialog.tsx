import { useMutation, useQuery } from '@chord/com.npmjs.tanstack__react-query';
import { taurpc } from '@chord/dev.improve.chord.api.taurpc';
import { Button } from '@chord/dev.improve.chord.components.ui.button';
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from '@chord/dev.improve.chord.components.ui.dialog';
import { usePermissionsState } from '@chord/dev.improve.chord.lib.state';
import { Keyboard, MousePointer2 } from 'lucide-react';
import { useEffect } from 'react';

type Permission = {
	id: 'accessibility' | 'input-monitoring'
	title: string
	description: string
	icon: typeof MousePointer2
}

const permissions: Permission[] = [
	{
		id: 'accessibility',
		title: 'Chord needs Accessibility access',
		description:
			'Accessibility lets Chord click controls and run actions in other apps. macOS requires you to approve this in System Settings.',
		icon: MousePointer2,
	},
	{
		id: 'input-monitoring',
		title: 'Chord needs Input Monitoring access',
		description:
			'Input Monitoring lets Chord detect your global trigger while you use other apps. macOS requires you to approve this in System Settings.',
		icon: Keyboard,
	},
];

export function PermissionsDialog({ onDismiss }: { onDismiss: () => void }) {
	const initialPermissions = usePermissionsState();
	const { data: permissionState, refetch: refetchPermissions } = useQuery({
		queryKey: ['macos-permissions'],
		queryFn: async () => {
			const [isAccessibilityEnabled, isInputMonitoringEnabled]
				= await taurpc.refreshPermissions();

			return { isAccessibilityEnabled, isInputMonitoringEnabled };
		},
		initialData: {
			isAccessibilityEnabled: initialPermissions.isAccessibilityEnabled ?? false,
			isInputMonitoringEnabled: initialPermissions.isInputMonitoringEnabled ?? false,
		},
		refetchOnWindowFocus: 'always',
	});
	useEffect(() => {
		const handleFocus = () => {
			void refetchPermissions();
		};

		window.addEventListener('focus', handleFocus);
		return () => {
			window.removeEventListener('focus', handleFocus);
		};
	}, [refetchPermissions]);
	const openAccessibilitySettingsMutation = useMutation({
		mutationFn: taurpc.openAccessibilitySettings,
	});
	const openInputMonitoringSettingsMutation = useMutation({
		mutationFn: taurpc.openInputMonitoringSettings,
	});
	const currentPermission = permissions.find((permission) => {
		if (permission.id === 'accessibility') {
			return !permissionState.isAccessibilityEnabled;
		}

		return !permissionState.isInputMonitoringEnabled;
	});
	const currentMutation = currentPermission?.id === 'accessibility'
		? openAccessibilitySettingsMutation
		: openInputMonitoringSettingsMutation;
	const PermissionIcon = currentPermission?.icon ?? MousePointer2;

	return (
		<Dialog
			open={currentPermission !== undefined}
			onOpenChange={(open) => {
				if (!open && currentPermission !== undefined) {
					onDismiss();
				}
			}}
		>
			<DialogContent
				showCloseButton={false}
				className="gap-0 overflow-hidden p-0 shadow-2xl sm:max-w-[430px]"
			>
				<div className="flex gap-4 p-5">
					<div className="flex size-12 shrink-0 items-center justify-center rounded-xl bg-primary/10 text-primary">
						<PermissionIcon className="size-6" strokeWidth={1.75} />
					</div>
					<DialogHeader className="gap-2 pt-0.5">
						<DialogTitle className="text-[17px] leading-5">
							{currentPermission?.title}
						</DialogTitle>
						<DialogDescription className="leading-5">
							{currentPermission?.description}
						</DialogDescription>
					</DialogHeader>
				</div>

				<DialogFooter className="m-0 rounded-none px-4 py-3">
					<Button type="button" variant="ghost" onClick={onDismiss}>
						Not Now
					</Button>
					<Button
						type="button"
						onClick={() => {
							currentMutation.mutate();
						}}
						disabled={currentMutation.isPending}
					>
						{currentMutation.isPending ? 'Opening…' : 'Open System Settings'}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}
