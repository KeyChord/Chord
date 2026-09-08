import { useMutation, useQueryClient } from '@chord/com.npmjs.tanstack__react-query';
import { taurpc } from '@chord/dev.improve.chord.api.taurpc';
import { Button } from '@chord/dev.improve.chord.components.ui.button';

export function SyncRepoButton({ repo }: { repo: { slug: string } }) {
	const queryClient = useQueryClient();
	const syncGitRepoMutation = useMutation({
		mutationFn: taurpc.syncGitRepo,
		onSuccess: () => queryClient.invalidateQueries({ queryKey: ["monorepo-packages", repo.slug] }),
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
