import { Button } from '#/components/ui/button.tsx';
import { openUrl } from '@tauri-apps/plugin-opener';
import { ExternalLink } from 'lucide-react';

export function OpenRepoButton({ repo }: { repo: { url: string, slug: string } }) {
	return (
		<Button
			type="button"
			variant="ghost"
			size="icon-sm"
			aria-label={`Open ${repo.slug}`}
			title="Open Repo"
			onClick={async () => {
				await openUrl(repo.url);
			}}
		>
			<ExternalLink />
		</Button>
	);
}
