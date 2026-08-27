import { useMutation } from '@chord/com.npmjs.tanstack__react-query';
import { taurpc } from '@chord/dev.improve.chord.api.taurpc';
import { Button } from '@chord/dev.improve.chord.components.ui.button';
import {
	Card,
	CardContent,
	CardDescription,
	CardFooter,
	CardHeader,
	CardTitle,
} from '@chord/dev.improve.chord.components.ui.card';

export function QuitChordCard() {
	const quitAppMutation = useMutation({
		mutationFn: taurpc.quitApp,
	});

	return (
		<Card size="sm">
			<CardHeader>
				<CardTitle>App Control</CardTitle>
				<CardDescription>
					Exit Chord immediately. You can still relaunch it later from Applications or your login
					items.
				</CardDescription>
			</CardHeader>
			<CardContent className="pt-0">
				<p className="text-sm text-muted-foreground">
					Use this when you want to fully stop the background app instead of just hiding the
					settings window.
				</p>
			</CardContent>
			<CardFooter className="justify-end">
				<Button
					variant="destructive"
					onClick={() => {
						quitAppMutation.mutate();
					}}
					disabled={quitAppMutation.isPending}
				>
					{quitAppMutation.isPending ? 'Quitting...' : 'Quit Chord'}
				</Button>
			</CardFooter>
		</Card>
	);
}
