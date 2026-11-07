import { useMutation } from '@chord/com.npmjs.tanstack__react-query';
import { Button } from '@chord/dev.improve.chord.components.ui.button';
import { openUrl } from '@tauri-apps/plugin-opener';
import { ExternalLink } from 'lucide-react';

export function OpenRepoButton({ repo }: { repo: { url: string, slug: string } }) {
	const openRepo = useMutation({ mutationFn: () => openUrl(repo.url) });
	return (
		<Button
			type="button"
			variant="ghost"
			size="icon-sm"
			aria-label={`Open ${repo.slug}`}
			title="Open Repo"
			onClick={() => {
				openRepo.mutate();
			}}
		>
			<ExternalLink />
		</Button>
	);
}
