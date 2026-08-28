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
		badge: 'Recommended',
		description:
			'JavaScriptCore with Bun\'s event loop, module loader and Bun/Node APIs (via rbun). Required by packages that load native code with `bun:ffi`, and faster at everything else.',
	},
	{
		id: 'quickjs',
		label: 'QuickJS (LLRT)',
		badge: 'Legacy',
		description:
			'rquickjs with the LLRT Node-compatible runtime. Kept for compatibility with older packages; it cannot load native code (`bun:ffi`).',
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
									<Label
										htmlFor={`js-engine-${engine.id}`}
										className="flex flex-wrap items-center gap-2"
									>
										{engine.label}
										<span
											className={engine.badge === 'Recommended'
												? 'rounded-full border border-primary/30 bg-primary/10 px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wide text-primary'
												: 'rounded-full border px-1.5 py-0.5 text-[10px] font-medium uppercase tracking-wide text-muted-foreground'}
										>
											{engine.badge}
										</span>
										{engine.id === settings.activeJsEngine
											? <span className="text-xs font-normal text-muted-foreground">running</span>
											: null}
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
