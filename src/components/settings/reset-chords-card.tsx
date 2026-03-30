import { taurpc } from '#/api/taurpc.ts';
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
} from '#/components/ui/alert-dialog.tsx';
import { Button } from '#/components/ui/button.tsx';
import {
	Card,
	CardContent,
	CardDescription,
	CardFooter,
	CardHeader,
	CardTitle,
} from '#/components/ui/card.tsx';
import { useMutation } from '@tanstack/react-query';
import { toast } from 'sonner';

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
								This removes the current managed git repo cache and replaces it with the
								default pinned chordpack.
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
