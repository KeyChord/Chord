import { createFileRoute, redirect } from '@tanstack/react-router';
import { getCurrentWindow } from '@tauri-apps/api/window';

export const Route = createFileRoute('/settings/')({
	loader: () => {
		const label = getCurrentWindow().label;
    throw redirect({ to: '/settings/general' });
	},
});
