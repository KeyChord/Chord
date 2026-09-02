import { toast } from '@chord/com.npmjs.sonner';
import { useMutation } from '@chord/com.npmjs.tanstack__react-query';
import { taurpc } from '@chord/dev.improve.chord.api.taurpc';
import {
	AlertDialog,
	AlertDialogAction,
	AlertDialogCancel,
	AlertDialogContent,
	AlertDialogDescription,
	AlertDialogFooter,
	AlertDialogHeader,
	AlertDialogTitle,
	AlertDialogTrigger,
} from '@chord/dev.improve.chord.components.ui.alert-dialog';
import { Button } from '@chord/dev.improve.chord.components.ui.button';
import {
	Card,
	CardContent,
	CardDescription,
	CardFooter,
	CardHeader,
	CardTitle,
} from '@chord/dev.improve.chord.components.ui.card';

export function ResetChordsCard() {
	const resetDefaultChordsMutation = useMutation({
		mutationFn: taurpc.resetDefaultChords,
		onSuccess: () => {
			toast.success('Reset managed chord repos to the default chordpack.');
		},
	});

	return (
		<Card size="sm">
			<CardHeader>
				<CardTitle>Reset Chords</CardTitle>
				<CardDescription>
					Replaces the managed git repo set with the bundled default chordpack. Local chord
					folders stay configured.
				</CardDescription>
			</CardHeader>
			<CardContent className="pt-0">
				<p className="text-sm text-muted-foreground">
					Use this to get back to the pinned default repos from
					{' '}
					<code>data/chordpack.toml</code>
					.
				</p>
			</CardContent>
			<CardFooter className="justify-end">
				<AlertDialog>
					<AlertDialogTrigger asChild>
						<Button variant="destructive" disabled={resetDefaultChordsMutation.isPending}>
							{resetDefaultChordsMutation.isPending ? 'Resetting...' : 'Reset Chords'}
						</Button>
					</AlertDialogTrigger>
					<AlertDialogContent size="sm">
						<AlertDialogHeader>
							<AlertDialogTitle>Reset managed chord repos?</AlertDialogTitle>
							<AlertDialogDescription>
								This replaces the active managed repo set with the default pinned chordpack.
								Cached revisions are retained for reuse.
							</AlertDialogDescription>
						</AlertDialogHeader>
						<AlertDialogFooter>
							<AlertDialogCancel disabled={resetDefaultChordsMutation.isPending}>
								Cancel
							</AlertDialogCancel>
							<AlertDialogAction
								variant="destructive"
								disabled={resetDefaultChordsMutation.isPending}
								onClick={() => {
									resetDefaultChordsMutation.mutate();
								}}
							>
								Reset Chords
							</AlertDialogAction>
						</AlertDialogFooter>
					</AlertDialogContent>
				</AlertDialog>
			</CardFooter>
		</Card>
	);
}
