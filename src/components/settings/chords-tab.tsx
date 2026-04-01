import { taurpc } from '#/api/taurpc.ts';
import { Badge } from '#/components/ui/badge.tsx';
import { Button } from '#/components/ui/button.tsx';
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from '#/components/ui/card.tsx';
import {
	Collapsible,
	CollapsibleContent,
	CollapsibleTrigger,
} from '#/components/ui/collapsible.tsx';
import {
	Empty,
	EmptyDescription,
	EmptyHeader,
	EmptyMedia,
	EmptyTitle,
} from '#/components/ui/empty.tsx';
import { useMutation } from '@tanstack/react-query';
import { ChevronDownIcon, FolderPlus, Package } from 'lucide-react';
import { toast } from 'sonner';
import { useChordPackageManagerState } from '../../utils/state.ts';

export function ChordsTab() {
	const { packages } = useChordPackageManagerState();
	const addLocalChordFolderMutation = useMutation({
		mutationFn: taurpc.addLocalChordFolder,
	});
	const pickLocalChordFolderMutation = useMutation({
		mutationFn: taurpc.pickLocalChordFolder,
	});
	const isAddingLocalChordPackage
		= addLocalChordFolderMutation.isPending || pickLocalChordFolderMutation.isPending;

	async function handleAddLocalChordPackage() {
		const path = await pickLocalChordFolderMutation.mutateAsync();
		if (!path) {
			return;
		}

		await addLocalChordFolderMutation.mutateAsync(path);
		toast.success('Added local chord package.');
	}

	return (
		<Card size="sm">
			<CardHeader>
				<div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
					<div className="space-y-1">
						<CardTitle>Loaded Chord Packages</CardTitle>
						<CardDescription>
							All chord packages currently loaded into the registry, including local packages.
						</CardDescription>
					</div>
					<div className="flex items-center gap-2">
						<Badge variant="outline">
							{packages.length}
							{' '}
							loaded
						</Badge>
						<Button
							type="button"
							size="sm"
							onClick={() => {
								void handleAddLocalChordPackage();
							}}
							disabled={isAddingLocalChordPackage}
						>
							<FolderPlus />
							{isAddingLocalChordPackage ? 'Adding...' : 'Add'}
						</Button>
					</div>
				</div>
			</CardHeader>
			<CardContent className="space-y-3 pt-0">
				{packages.length === 0
					? (
							<Empty className="rounded-lg border bg-muted/20 py-10">
								<EmptyHeader>
									<EmptyMedia variant="icon">
										<Package />
									</EmptyMedia>
									<EmptyTitle>No loaded chord packages</EmptyTitle>
									<EmptyDescription>
										Add a local chord package or load one from your configured git repos.
									</EmptyDescription>
								</EmptyHeader>
							</Empty>
						)
					: (
							<div className="space-y-2">
								{packages.map(pkg => (
									<Collapsible key={pkg.name} className="group/collapsible rounded-lg border bg-background/80 overflow-hidden">
										<CollapsibleTrigger asChild>
											<Button
												variant="ghost"
												className="flex w-full items-center justify-between rounded-none border-0 p-3 h-auto hover:bg-muted/50"
											>
												<span className="font-medium">{pkg.name}</span>
												<ChevronDownIcon className="size-4 text-muted-foreground transition-transform duration-200 group-data-[state=open]/collapsible:rotate-180" />
											</Button>
										</CollapsibleTrigger>
										<CollapsibleContent className="px-3 pb-3">
											{pkg.compiledChordsFiles && Object.values(pkg.compiledChordsFiles).length > 0
												? (
														<div className="ml-4 space-y-1">
															{Object.values(pkg.compiledChordsFiles).flatMap(file => (
																file.chords.map(chord => (
																	<p key={chord.rawTrigger} className="text-sm text-muted-foreground">
																		-
																		{' '}
																		{chord.name}
																	</p>
																))
															))}
														</div>
													)
												: (
														<div className="ml-4">
															<p className="text-sm text-muted-foreground italic">No chords loaded for this package.</p>
														</div>
													)}
										</CollapsibleContent>
									</Collapsible>
								))}
							</div>
						)}
			</CardContent>
		</Card>
	);
}
