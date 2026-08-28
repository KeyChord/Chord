import { useMutation } from '@chord/com.npmjs.tanstack__react-query';
import { taurpc } from '@chord/dev.improve.chord.api.taurpc';
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from '@chord/dev.improve.chord.components.ui.card';
import { Label } from '@chord/dev.improve.chord.components.ui.label';
import { RadioGroup, RadioGroupItem } from '@chord/dev.improve.chord.components.ui.radio-group';
import { useSettingsState } from '@chord/dev.improve.chord.lib.state';

const ENGINES = [
	{
		id: 'bun',
		label: 'Bun',
		description:
			'The default engine: JavaScriptCore with Bun\'s event loop, module loader and Bun/Node APIs (via rbun). Packages that load native code with `bun:ffi` need it.',
	},
	{
		id: 'quickjs',
		label: 'QuickJS (LLRT)',
		description: 'rquickjs with the LLRT Node-compatible runtime; no `bun:ffi`.',
	},
] as const;

export function JsEngineCard() {
	const settings = useSettingsState();
	const setJsEngineMutation = useMutation({
		mutationFn: (engine: string) => taurpc.setJsEngine(engine),
	});
	const needsRestart = settings.jsEngine !== settings.activeJsEngine;

	return (
		<Card size="sm">
			<CardHeader>
				<CardTitle>JavaScript Engine</CardTitle>
				<CardDescription>
					Choose which engine runs JS handlers. Changes apply after Chord restarts.
				</CardDescription>
			</CardHeader>
			<CardContent className="pt-0">
				<RadioGroup
					value={settings.jsEngine}
					disabled={setJsEngineMutation.isPending}
					onValueChange={(value) => {
						if (typeof value === 'string' && value !== settings.jsEngine) {
							setJsEngineMutation.mutate(value);
						}
					}}
					className="overflow-hidden rounded-lg border bg-background/80"
				>
					{ENGINES.map((engine) => {
						const unavailable = engine.id === 'bun' && !settings.isBunEngineAvailable;
						return (
							<div
								key={engine.id}
								className="flex items-start gap-3 border-b px-3 py-3 last:border-b-0"
							>
								<RadioGroupItem
									id={`js-engine-${engine.id}`}
									value={engine.id}
									disabled={unavailable}
								/>
								<div className="space-y-1">
									<Label htmlFor={`js-engine-${engine.id}`}>
										{engine.label}
										{engine.id === settings.activeJsEngine ? ' — running' : ''}
									</Label>
									<p className="text-sm text-muted-foreground">
										{engine.description}
										{unavailable
											? ' This build was compiled without Bun support (cargo feature `bun`).'
											: ''}
									</p>
								</div>
							</div>
						);
					})}
				</RadioGroup>
				{needsRestart
					? (
						<p className="pt-3 text-sm text-muted-foreground">
							Restart Chord to switch to the selected engine.
						</p>
					)
					: null}
			</CardContent>
		</Card>
	);
}
