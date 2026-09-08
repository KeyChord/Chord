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
import { StateQueries, useSettingsState } from '@chord/dev.improve.chord.lib.state';
import { useEffect } from 'react';

export function CommandLineCard() {
	return (
		<StateQueries queries={[useSettingsState()]}>
			{(settingsData) => (
				<CommandLineCardContent settingsData={settingsData} />
			)}
		</StateQueries>
	);
}

function CommandLineCardContent({
	settingsData,
}: {
	settingsData: NonNullable<ReturnType<typeof useSettingsState>['data']>
}) {
	const { cliCommand, isCliInstalled } = settingsData;
	const install = useMutation({ mutationFn: taurpc.installCli });
	const refresh = useMutation({ mutationFn: taurpc.refreshCliInstallation });
	const { mutate: refreshStatus } = refresh;
	useEffect(() => {
		refreshStatus();
		const onFocus = () => refreshStatus();
		window.addEventListener('focus', onFocus);
		return () => window.removeEventListener('focus', onFocus);
	}, [refreshStatus]);
	const error = install.error ?? refresh.error;

	return (
		<Card size="sm">
			<CardHeader>
				<CardTitle>Command Line</CardTitle>
				<CardDescription>
					Add <code>{cliCommand}</code> to your PATH to run chords, shell commands, and scripts
					from your terminal.
				</CardDescription>
			</CardHeader>
			<CardContent className="space-y-2 pt-0 text-sm text-muted-foreground">
				<p>
					Installs a link at <code>/usr/local/bin/{cliCommand}</code>. macOS may ask for your
					administrator password.
				</p>
				{cliCommand === 'chordd' && (
					<p>Development builds use <code>chordd</code> (Chord Dev) so your production <code>chord</code> command stays separate.</p>
				)}
				{isCliInstalled && (
					<p role="status">
						Installed. Run <code>{cliCommand} --help</code> in your terminal to get started.
						If the command is not found, make sure <code>/usr/local/bin</code> is in your shell’s PATH.
					</p>
				)}
				{error && <p role="alert" className="text-destructive">{errorMessage(error)}</p>}
			</CardContent>
			<CardFooter className="justify-end">
				<Button
					disabled={isCliInstalled || install.isPending || refresh.isPending}
					onClick={() => install.mutate()}
				>
					{install.isPending ? 'Installing…' : isCliInstalled ? 'Installed' : `Add ${cliCommand} to PATH`}
				</Button>
			</CardFooter>
		</Card>
	);
}

function errorMessage(error: unknown): string {
	if (error && typeof error === 'object' && 'Message' in error && typeof error.Message === 'string') {
		return error.Message;
	}
	return error instanceof Error ? error.message : String(error);
}
