import { Button } from '#/components/ui/button.tsx';
import { useMutation } from '@tanstack/react-query';
import { taurpc } from '../../api/taurpc.ts';

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
