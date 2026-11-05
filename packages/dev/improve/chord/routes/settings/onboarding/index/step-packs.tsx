import { useMutation } from '@chord/com.npmjs.tanstack__react-query';
import { taurpc } from '@chord/dev.improve.chord.api.taurpc';
import { Button } from '@chord/dev.improve.chord.components.ui.button';
import { officialChordReposData } from '@chord/dev.improve.chord.data.official-chord-repos';
import { getGitHubSlug } from '@chord/dev.improve.chord.lib.github';
import { useGitRepoStoreState } from '@chord/dev.improve.chord.lib.state';
import { Check, Loader2 } from 'lucide-react';
import { StepLayout } from '#step-layout';

export function PacksView({
	installedSlugs,
	pendingUrl,
	failedUrl,
	onAdd,
}: {
	installedSlugs: ReadonlySet<string>;
	pendingUrl?: string;
	failedUrl?: string;
	onAdd: (url: string) => void;
}) {
	return (
		<ul className="divide-y divide-black/[0.06] rounded-[12px] border border-black/[0.08] dark:divide-white/10 dark:border-white/10">
			{officialChordReposData.map((repo) => {
				const slug = getGitHubSlug(repo.url)?.toLowerCase() ?? repo.url;
				const isInstalled = installedSlugs.has(slug);
				const isPending = pendingUrl === repo.url;
				return (
					<li key={repo.url} className="flex items-center justify-between gap-4 px-4 py-2.5">
						<div className="min-w-0">
							<p className="text-[13.5px] font-medium leading-[19px] text-foreground">{repo.displayName}</p>
							<p className="truncate text-[12px] leading-[17px] text-muted-foreground">{repo.description}</p>
							{failedUrl === repo.url
								? <p role="alert" className="text-[12px] leading-[17px] text-red-600 dark:text-red-400">Couldn't add this pack. Check your internet connection and try again.</p>
								: null}
						</div>
						{isInstalled
							? (
									<span className="flex shrink-0 items-center gap-1 text-[12.5px] font-medium text-emerald-700 dark:text-emerald-400">
										<Check className="size-4" /> Added
									</span>
								)
							: (
									<Button
										type="button"
										variant="outline"
										size="sm"
										className="shrink-0"
										disabled={pendingUrl !== undefined}
										aria-label={`Add ${repo.displayName}`}
										onClick={() => onAdd(repo.url)}
									>
										{isPending ? <Loader2 className="animate-spin" /> : null}
										{isPending ? 'Adding…' : 'Add'}
									</Button>
								)}
					</li>
				);
			})}
		</ul>
	);
}

export function PacksStep({
	stepNumber,
	stepCount,
	onContinue,
}: {
	stepNumber: number;
	stepCount: number;
	onContinue: () => void;
}) {
	const { data } = useGitRepoStoreState();
	const add = useMutation({ mutationFn: (url: string) => taurpc.addGitRepo(url), onError: () => {} });
	const installedSlugs = new Set(Object.values(data?.repos ?? {}).map(repo => repo.slug.toLowerCase()));
	return (
		<StepLayout
			stepNumber={stepNumber}
			stepCount={stepCount}
			title="Want chords for the apps you use?"
			description="Chord packs are ready-made sets of chords. Add any that match your apps, or continue without them. You can always add more later under Browse."
			actions={(
				<Button type="button" size="lg" onClick={onContinue} disabled={add.isPending}>
					Continue
				</Button>
			)}
		>
			<PacksView
				installedSlugs={installedSlugs}
				pendingUrl={add.isPending ? add.variables : undefined}
				failedUrl={add.isError ? add.variables : undefined}
				onAdd={url => add.mutate(url)}
			/>
		</StepLayout>
	);
}
