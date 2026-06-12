import { Button } from '#/components/ui/button.tsx';
import { useMutation } from "@keychord/com.npmjs.tanstack__react-query";
import { taurpc } from '../../api/taurpc.ts';

export function RelaunchAppButton({ app }: { app: { bundleId: string } }) {
	const relaunchAppMutation = useMutation({
		mutationFn: taurpc.relaunchApp,
	});

	return (
		<Button
			type="button"
			variant="outline"
			size="sm"
			onClick={() => {
				relaunchAppMutation.mutate(app.bundleId);
			}}
			disabled={relaunchAppMutation.isPending}
		>
			{relaunchAppMutation.isPending ? 'Relaunching...' : 'Relaunch'}
		</Button>
	);
}
