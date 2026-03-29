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
	Empty,
	EmptyDescription,
	EmptyHeader,
	EmptyMedia,
	EmptyTitle,
} from '#/components/ui/empty.tsx';
import { useMutation } from '@tanstack/react-query';
import { FolderPlus, Package } from 'lucide-react';
import { toast } from 'sonner';
import { useChordFilesState } from '../../utils/state.ts';

export function ChordsTab() {
	const { loadedPackages } = useChordFilesState();
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
							{loadedPackages.length}
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
				{loadedPackages.length === 0
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
								{loadedPackages.map(chordPackage => (
									<div
										key={chordPackage.path}
										className="flex items-center justify-between gap-3 rounded-lg border bg-background/80 px-3 py-3"
									>
										<div className="min-w-0">
											<p className="font-medium">{chordPackage.name}</p>
											<p className="truncate text-xs text-muted-foreground">{chordPackage.path}</p>
										</div>
										<Badge variant={chordPackage.kind === 'Local' ? 'secondary' : 'outline'}>
											{chordPackage.kind}
										</Badge>
									</div>
								))}
							</div>
						)}
			</CardContent>
		</Card>
	);
}
