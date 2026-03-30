import { Badge } from '#/components/ui/badge.tsx';
import { Button } from '#/components/ui/button.tsx';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '#/components/ui/card.tsx';
import { Checkbox } from '#/components/ui/checkbox.tsx';
import { Empty, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle } from '#/components/ui/empty.tsx';
import { Input } from '#/components/ui/input.tsx';
import { useMutation } from '@tanstack/react-query';
import { CheckCircle2, Plus, Search, Trash2 } from 'lucide-react';
import { useState } from 'react';
import { toast } from 'sonner';
import officialChordReposData from '../../../data/official-chord-repos.json';
import { taurpc } from '../../api/taurpc.ts';
import { useGitRepoStoreState } from '../../utils/state.ts';

interface OfficialChordRepo {
	name: string
	url: string
}

const githubDotGitSuffixRegex = /\.git$/i;
const officialChordRepos: OfficialChordRepo[] = officialChordReposData;
const defaultSelectedRepoUrls = officialChordRepos.map(repo => repo.url);

export function BrowseTab() {
	const { repos } = useGitRepoStoreState();
	const [searchInput, setSearchInput] = useState('');
	const [selectedRepoUrls, setSelectedRepoUrls] = useState<string[]>([]);

	const installedRepoSlugs = new Set(Object.values(repos).map(repo => repo.slug.toLowerCase()));
	const addGitRepoMutation = useMutation({
		mutationFn: taurpc.addGitRepo,
	});
	const removeGitRepoMutation = useMutation({
		mutationFn: (slug: string) => taurpc.removeGitRepo(slug),
	});

	const filteredRepos = officialChordRepos.filter((repo) => {
		const normalizedSearch = searchInput.trim().toLowerCase();
		if (!normalizedSearch) {
			return true;
		}

		const slug = getGitHubSlug(repo.url) ?? repo.url;
		return `${repo.name} ${slug}`.toLowerCase().includes(normalizedSearch);
	});

	const selectedMissingRepos = officialChordRepos.filter((repo) => {
		const slug = getGitHubSlug(repo.url);
		const isInstalled = slug ? installedRepoSlugs.has(slug.toLowerCase()) : false;
		return selectedRepoUrls.includes(repo.url) && !isInstalled;
	});
	const missingRepos = officialChordRepos.filter((repo) => {
		const slug = getGitHubSlug(repo.url);
		return slug ? !installedRepoSlugs.has(slug.toLowerCase()) : true;
	});

	async function addRepo(repo: OfficialChordRepo) {
		await addGitRepoMutation.mutateAsync(repo.url);
		toast.success(`Added ${repo.name}.`);
	}

	async function removeRepo(slug: string, repoName: string) {
		await removeGitRepoMutation.mutateAsync(slug);
		toast.success(`Removed ${repoName}.`);
	}

	async function addSelectedRepos() {
		await addRepos(selectedMissingRepos);
	}

	async function addMissingRepos() {
		await addRepos(missingRepos);
	}

	async function addRepos(reposToAdd: OfficialChordRepo[]) {
		if (reposToAdd.length === 0) {
			return;
		}

		let addedCount = 0;
		for (const repo of reposToAdd) {
			try {
				await addGitRepoMutation.mutateAsync(repo.url);
				addedCount += 1;
			}
			catch (error) {
				if (addedCount > 0) {
					toast.success(`Added ${addedCount} repo${addedCount === 1 ? '' : 's'} before the error.`);
				}
				throw error;
			}
		}

		toast.success(
			`Added ${addedCount} curated repo${addedCount === 1 ? '' : 's'}.`,
		);
	}

	return (
		<Card size="sm">
			<CardHeader>
				<div className="flex flex-col gap-3">
					<div className="space-y-1">
						<CardTitle>Browse Official Repos</CardTitle>
						<CardDescription>
							Curated chord repos from the official catalog. Missing repos are preselected so you can add a starter set quickly.
						</CardDescription>
					</div>

					<div className="flex flex-col gap-3 sm:flex-row sm:items-center">
						<div className="relative flex-1">
							<Search className="pointer-events-none absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground" />
							<Input
								value={searchInput}
								onChange={(event) => {
									setSearchInput(event.target.value);
								}}
								placeholder="Filter by repo name or slug"
								className="pl-9"
								disabled={addGitRepoMutation.isPending}
							/>
						</div>

						<div className="flex flex-wrap items-center gap-2">
							<Badge variant="outline">
								{missingRepos.length}
								{' '}
								available
							</Badge>
							<Button
								type="button"
								variant="outline"
								size="sm"
								onClick={() => {
									setSelectedRepoUrls(missingRepos.map(repo => repo.url));
								}}
								disabled={addGitRepoMutation.isPending || missingRepos.length === 0}
							>
								Select Missing
							</Button>
							<Button
								type="button"
								variant="outline"
								size="sm"
								onClick={addMissingRepos}
								disabled={addGitRepoMutation.isPending || missingRepos.length === 0}
							>
								{addGitRepoMutation.isPending ? 'Adding...' : 'Add All Missing'}
							</Button>
							<Button
								type="button"
								size="sm"
								onClick={addSelectedRepos}
								disabled={addGitRepoMutation.isPending || selectedMissingRepos.length === 0}
							>
								{addGitRepoMutation.isPending
									? 'Adding...'
									: `Add Selected (${selectedMissingRepos.length})`}
							</Button>
						</div>
					</div>
				</div>
			</CardHeader>

			<CardContent className="space-y-3 pt-0">
				{filteredRepos.length === 0
					? (
							<Empty className="rounded-lg border bg-muted/20 py-10">
								<EmptyHeader>
									<EmptyMedia variant="icon">
										<Search />
									</EmptyMedia>
									<EmptyTitle>No matching repos</EmptyTitle>
									<EmptyDescription>
										Try a different filter or clear the search field.
									</EmptyDescription>
								</EmptyHeader>
							</Empty>
						)
					: (
							filteredRepos.map((repo) => {
								const slug = getGitHubSlug(repo.url);
								const isInstalled = slug ? installedRepoSlugs.has(slug.toLowerCase()) : false;
								const isSelected = selectedRepoUrls.includes(repo.url);

								return (
									<div
										key={repo.url}
										className="rounded-lg border bg-background/80 px-3 py-3"
									>
										<div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
											<label
												htmlFor={repo.url}
												className="flex min-w-0 flex-1 cursor-pointer items-start gap-3"
											>
												<Checkbox
													id={repo.url}
													checked={isInstalled ? true : isSelected}
													disabled={isInstalled || addGitRepoMutation.isPending}
													onCheckedChange={(checked) => {
														setSelectedRepoUrls((currentSelection) => {
															if (checked) {
																return currentSelection.includes(repo.url)
																	? currentSelection
																	: [...currentSelection, repo.url];
															}

															return currentSelection.filter(url => url !== repo.url);
														});
													}}
													className="mt-0.5"
												/>

												<div className="min-w-0 space-y-1">
													<div className="flex flex-wrap items-center gap-2">
														<p className="font-medium">{repo.name}</p>
														<Badge variant="secondary">Official</Badge>
														{isInstalled
															? (
																	<Badge variant="outline" className="gap-1">
																		<CheckCircle2 className="size-3.5" />
																		Added
																	</Badge>
																)
															: null}
													</div>
													<p className="text-sm text-muted-foreground">
														{slug ?? repo.url}
													</p>
												</div>
											</label>

											{isInstalled && slug
												? (
														<Button
															type="button"
															variant="ghost"
															size="sm"
															className="text-destructive hover:bg-destructive/10 hover:text-destructive"
															onClick={() => {
																void removeRepo(slug, repo.name);
															}}
															disabled={removeGitRepoMutation.isPending}
														>
															<Trash2 />
															Uninstall
														</Button>
													)
												: (
														<Button
															type="button"
															variant="outline"
															size="sm"
															onClick={() => {
																void addRepo(repo);
															}}
															disabled={addGitRepoMutation.isPending}
														>
															{addGitRepoMutation.isPending && addGitRepoMutation.variables === repo.url
																? 'Adding...'
																: (
																		<>
																			<Plus />
																			Add Repo
																		</>
																	)}
														</Button>
													)}
										</div>
									</div>
								);
							})
						)}
			</CardContent>
		</Card>
	);
}

function getGitHubSlug(url: string) {
	try {
		const parsedUrl = new URL(url);
		if (!parsedUrl.hostname.endsWith('github.com')) {
			return undefined;
		}

		const [owner, name] = parsedUrl.pathname
			.split('/')
			.filter(Boolean)
			.map(segment => segment.replace(githubDotGitSuffixRegex, ''));

		if (!owner || !name) {
			return undefined;
		}

		return `${owner}/${name}`;
	}
	catch {
		return undefined;
	}
}
