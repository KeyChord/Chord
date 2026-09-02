import { useMutation, useQuery, useQueryClient } from '@chord/com.npmjs.tanstack__react-query';
import { taurpc } from '@chord/dev.improve.chord.api.taurpc';
import { Badge } from '@chord/dev.improve.chord.components.ui.badge';
import { Button } from '@chord/dev.improve.chord.components.ui.button';
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from '@chord/dev.improve.chord.components.ui.card';

export function LocalFoldersCard() {
	const queryClient = useQueryClient();
	const pickLocalChordFolderMutation = useMutation({
		mutationFn: taurpc.pickLocalChordFolder,
		onSuccess: async () => {
			await queryClient.invalidateQueries({
				queryKey: ['local-chord-folders'],
			});
		},
	});
	const localFoldersQuery = useQuery({
		queryKey: ['local-chord-folders'],
		queryFn: taurpc.listLocalChordFolders,
	});
	const folders = localFoldersQuery.data ?? [];

	return (
		<Card size="sm">
			<CardHeader className="flex items-center justify-between gap-3">
				<CardTitle>Local Folders</CardTitle>
				<CardDescription>
					Local folders are loaded in place. Use the tray reload action after editing files to
					rebuild the JS runtime.
				</CardDescription>
			</CardHeader>
			<CardContent className="space-y-4 pt-0">
				<div className="flex justify-end">
					<Button
						type="button"
						onClick={() => {
							pickLocalChordFolderMutation.mutate();
						}}
						disabled={pickLocalChordFolderMutation.isPending}
					>
						{pickLocalChordFolderMutation.isPending ? 'Adding...' : 'Add Folder'}
					</Button>
				</div>

				{localFoldersQuery.isLoading
					? (
							<p className="text-sm text-muted-foreground">Loading local folders...</p>
						)
					: folders.length === 0
						? (
								<p className="text-sm text-muted-foreground">
									No local folders added yet. Add one to load chords directly from disk.
								</p>
							)
						: (
								<div className="space-y-2">
									{folders.map(folder => (
										<div
											key={folder}
											className="flex items-center justify-between gap-3 rounded-lg border bg-background/80 px-3 py-3"
										>
											<div className="min-w-0">
												<p className="font-medium">Local Chord Folder</p>
												<p className="truncate text-xs text-muted-foreground">{folder}</p>
											</div>
											<Badge variant="secondary">Local</Badge>
										</div>
									))}
								</div>
							)}
			</CardContent>
		</Card>
	);
}
