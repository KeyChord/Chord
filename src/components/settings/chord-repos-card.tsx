import { Badge } from '#/components/ui/badge.tsx';
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from '#/components/ui/card.tsx';
import { useGitRepoStoreState } from '../../utils/state.ts';
import { AddRepoButton } from './add-repo-button.tsx';
import { OpenRepoButton } from './open-repo-button.tsx';
import { SyncRepoButton } from './sync-repo-button.tsx';

export function ChordReposCard() {
	const { repos } = useGitRepoStoreState();
	return (
		<Card size="sm">
			<CardHeader>
				<div className="flex items-center justify-between gap-3">
					<div>
						<CardTitle>Chord Repos</CardTitle>
						<CardDescription>
							Added GitHub repos are cloned into the app cache and merged with bundled chords.
						</CardDescription>
					</div>
				</div>
			</CardHeader>
			<CardContent className="space-y-4 pt-0">
				<AddRepoButton />
				<div className="space-y-3">
					{Object.values(repos).length === 0
						? (
								<p className="text-sm text-muted-foreground">
									No external repos added yet. Bundled chords still load by default.
								</p>
							)
						: (
								Object.values(repos).map(repo => <GitRepoRow key={repo.slug} repo={repo} />)
							)}
				</div>
			</CardContent>
		</Card>
	);
}

function GitRepoRow({ repo }: { repo: { slug: string, headShortSha?: string, url: string } }) {
	return (
		<div key={repo.slug} className="rounded-lg border bg-background/80 px-3 py-3">
			<div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
				<div className="min-w-0 space-y-1">
					<div className="flex items-center gap-2">
						<p className="truncate font-medium">{repo.slug}</p>
						<Badge variant="secondary">GitHub</Badge>
						{repo.headShortSha
							? (
									<Badge variant="outline" className="font-mono text-[11px]">
										{repo.headShortSha}
									</Badge>
								)
							: null}
					</div>
				</div>
				<div className="flex flex-wrap items-center gap-2 self-end sm:self-center">
					<OpenRepoButton repo={repo} />
					<SyncRepoButton repo={repo} />
				</div>
			</div>
		</div>
	);
}
