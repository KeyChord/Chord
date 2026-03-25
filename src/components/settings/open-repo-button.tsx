import { Button } from '#/components/ui/button.tsx';
import { openUrl } from '@tauri-apps/plugin-opener';
import { ExternalLink } from 'lucide-react';

export function OpenRepoButton({ repo }: { repo: { url: string, slug: string } }) {
	return (
		<Button
			type="button"
			variant="ghost"
			size="icon-sm"
			aria-label={`Open ${repo.slug} on GitHub`}
			title="Open on GitHub"
			onClick={async () => {
				await openUrl(repo.url);
			}}
		>
			<ExternalLink />
		</Button>
	);
}
