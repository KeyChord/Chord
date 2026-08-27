import { useMutation } from '@chord/com.npmjs.tanstack__react-query';
import { taurpc } from '@chord/dev.improve.chord.api.taurpc';
import { Button } from '@chord/dev.improve.chord.components.ui.button';

export function SyncRepoButton({ repo }: { repo: { slug: string } }) {
	const syncGitRepoMutation = useMutation({
		mutationFn: taurpc.syncGitRepo,
	});

	return (
		<Button
			type="button"
			variant="outline"
			size="sm"
			onClick={() => {
				syncGitRepoMutation.mutate(repo.slug);
			}}
			disabled={syncGitRepoMutation.isPending}
		>
			{syncGitRepoMutation.isPending ? 'Syncing...' : 'Sync Latest'}
		</Button>
	);
}
