import { useMutation } from '@chord/com.npmjs.tanstack__react-query';
import { taurpc } from '@chord/dev.improve.chord.api.taurpc';
import { Button } from '@chord/dev.improve.chord.components.ui.button';

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
