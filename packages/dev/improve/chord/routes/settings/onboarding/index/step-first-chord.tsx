import { useMutation, useQuery } from '@chord/com.npmjs.tanstack__react-query';
import { taurpc, type InstalledApp } from '@chord/dev.improve.chord.api.taurpc';
import { Button } from '@chord/dev.improve.chord.components.ui.button';
import { Input } from '@chord/dev.improve.chord.components.ui.input';
import { useChordPackageManagerState } from '@chord/dev.improve.chord.lib.state';
import type { ChordReference, ThirdPartyPackage } from '@chord/dev.improve.chord.lib.typeshare';
import { cn } from '@chord/com.npmjs.utils-cn';
import { Loader2, Search } from 'lucide-react';
import { useState } from 'react';
import {
	chordSequence,
	findBlockedChordsFolder,
	findChordConflict,
	GLOBAL_PREFIX,
	sanitizeChordKeys,
	suggestChordKeys,
} from '#first-chord';
import { Key } from '#keyboard-map';
import { Click, StepLayout } from '#step-layout';

const SUGGESTED_APPS = 12;

export interface CreatedChord {
	sequence: string;
	appName: string;
}

function errorMessage(error: unknown) {
	if (error && typeof error === 'object' && 'Message' in error) return String(error.Message);
	return error instanceof Error ? error.message : String(error);
}

export function AppTile({
	app,
	selected,
	onSelect,
}: {
	app: InstalledApp;
	selected: boolean;
	onSelect: () => void;
}) {
	return (
		<button
			type="button"
			onClick={onSelect}
			aria-pressed={selected}
			className={cn(
				'flex min-w-0 flex-col items-center gap-1 rounded-[10px] border p-2 outline-none transition-colors focus-visible:ring-2 focus-visible:ring-ring',
				selected
					? 'border-primary bg-primary/10'
					: 'border-transparent hover:bg-black/[0.04] dark:hover:bg-white/[0.06]',
			)}
		>
			{app.iconDataUrl
				? <img src={app.iconDataUrl} alt="" className="size-10 object-contain" />
				: <div className="size-10 rounded-[9px] bg-black/10" />}
			<span className="w-full truncate text-center text-[11.5px] leading-[15px] text-foreground">{app.name}</span>
		</button>
	);
}

/** Where the chord's keys are chosen, and anything that has to change before it can work. */
export function ChordKeysPanel({
	appName,
	keys,
	conflict,
	blockedFolder,
	isAllowing,
	error,
	onKeysChange,
	onAllowFolder,
}: {
	appName: string;
	keys: string;
	conflict?: ChordReference;
	blockedFolder?: ThirdPartyPackage;
	isAllowing: boolean;
	error?: string;
	onKeysChange: (keys: string) => void;
	onAllowFolder: () => void;
}) {
	return (
		<div className="space-y-2 rounded-[12px] border border-black/[0.08] bg-black/[0.02] p-4 dark:border-white/10 dark:bg-white/[0.03]">
			<label className="flex items-center gap-2 text-[13.5px] text-foreground">
				<span>To open <Click>{appName}</Click>, you'll type</span>
				<Key>{GLOBAL_PREFIX}</Key>
				<Input
					aria-label="Letters for this chord"
					value={keys}
					onChange={event => onKeysChange(sanitizeChordKeys(event.target.value))}
					className="w-16 text-center font-medium"
				/>
			</label>
			{conflict
				? (
						<p role="alert" className="text-[12.5px] font-medium leading-[19px] text-amber-800 dark:text-amber-300">
							<span className="font-semibold">{chordSequence(keys)}</span> is already used by <span className="font-semibold">{conflict.chord.name}</span>. Choose a different letter to continue.
						</p>
					)
				: (
						<p className="text-[12.5px] leading-[19px] text-muted-foreground">
							Change the letter to anything easy to remember. Starting with <Key>{GLOBAL_PREFIX}</Key> is what makes it work in every app.
						</p>
					)}
			{blockedFolder
				? (
						<div role="alert" className="space-y-2 rounded-[10px] bg-amber-500/10 p-3 text-[12.5px] leading-[19px] text-amber-800 dark:text-amber-300">
							<p>
								Chord found a chords folder from an earlier install that it hasn't been allowed to use, so a new chord wouldn't run. Chords in this folder can run code on your Mac, so only allow it if you recognize it.
							</p>
							<code className="block break-all text-[11.5px]">{blockedFolder.path}</code>
							<Button type="button" size="sm" variant="outline" disabled={isAllowing} onClick={onAllowFolder}>
								{isAllowing ? 'Allowing…' : 'Allow This Folder'}
							</Button>
						</div>
					)
				: null}
			{error
				? <p role="alert" className="text-[12.5px] font-medium text-red-600 dark:text-red-400">{error}</p>
				: null}
		</div>
	);
}

export function FirstChordStep({
	stepNumber,
	stepCount,
	onCreated,
	onSkip,
}: {
	stepNumber: number;
	stepCount: number;
	onCreated: (chord: CreatedChord) => void;
	onSkip: () => void;
}) {
	const apps = useQuery({
		queryKey: ['installed-apps'],
		queryFn: () => taurpc.listInstalledApps(),
		staleTime: Number.POSITIVE_INFINITY,
	});
	const [selected, setSelected] = useState<InstalledApp>();
	const [keys, setKeys] = useState('');
	const [search, setSearch] = useState('');
	const add = useMutation({
		mutationFn: ({ app, sequence }: { app: InstalledApp; sequence: string }) =>
			taurpc.addAppChord(sequence, app.name, app.bundleId),
		// The screen explains the problem where the keys are chosen.
		onError: () => {},
		onSuccess: (_, { app, sequence }) => onCreated({ sequence, appName: app.name }),
	});
	const allowFolder = useMutation({
		mutationFn: (folder: ThirdPartyPackage) => taurpc.setPackageApprovals([folder.identity], true),
		onError: () => {},
	});
	const { data: packageManager } = useChordPackageManagerState();
	const existingChords = packageManager?.packages.flatMap(pkg => pkg.globalChords) ?? [];
	const blockedFolder = packageManager ? findBlockedChordsFolder(packageManager) : undefined;
	// A chord shows up in this list as soon as it's saved, and doesn't conflict with itself.
	const conflict = keys && !add.isPending && !add.isSuccess
		? findChordConflict(chordSequence(keys), existingChords)
		: undefined;

	const normalizedSearch = search.trim().toLowerCase();
	const visible = (apps.data ?? [])
		.filter(app => app.name.toLowerCase().includes(normalizedSearch))
		.slice(0, normalizedSearch ? 24 : SUGGESTED_APPS);

	function select(app: InstalledApp) {
		setSelected(app);
		setKeys(suggestChordKeys(app.name, existingChords));
		add.reset();
	}

	return (
		<StepLayout
			stepNumber={stepNumber}
			stepCount={stepCount}
			title="Make your first chord"
			description="Pick an app you use a lot. We'll make a chord that opens it from anywhere."
			actions={(
				<Button
					type="button"
					size="lg"
					disabled={!selected || !keys || !packageManager || conflict !== undefined || blockedFolder !== undefined || add.isPending}
					onClick={() => {
						if (selected) add.mutate({ app: selected, sequence: chordSequence(keys) });
					}}
				>
					{add.isPending ? 'Creating…' : 'Create Chord'}
				</Button>
			)}
			secondary={(
				<Button type="button" variant="ghost" onClick={onSkip}>
					Skip for Now
				</Button>
			)}
		>
			<div className="space-y-2">
				<div className="relative">
					<Search className="pointer-events-none absolute left-2.5 top-1/2 z-10 size-3.5 -translate-y-1/2 text-muted-foreground" />
					<Input
						aria-label="Search your apps"
						placeholder="Search your apps"
						value={search}
						onChange={event => setSearch(event.target.value)}
						className="pl-8"
					/>
				</div>
				{apps.isPending
					? (
							<p className="flex h-[212px] items-center justify-center gap-2 text-[13px] text-muted-foreground">
								<Loader2 className="size-4 animate-spin" /> Finding your apps…
							</p>
						)
					: (
							<div className="grid h-[212px] grid-cols-6 content-start gap-1 overflow-y-auto">
								{visible.map(app => (
									<AppTile
										key={app.bundleId}
										app={app}
										selected={selected?.bundleId === app.bundleId}
										onSelect={() => select(app)}
									/>
								))}
								{visible.length === 0
									? <p className="col-span-6 py-8 text-center text-[13px] text-muted-foreground">No apps match "{search}".</p>
									: null}
							</div>
						)}
			</div>
			{selected
				? (
						<ChordKeysPanel
							appName={selected.name}
							keys={keys}
							conflict={conflict}
							blockedFolder={blockedFolder}
							isAllowing={allowFolder.isPending}
							error={
								allowFolder.isError
									? errorMessage(allowFolder.error)
									: add.isError
										? `${errorMessage(add.error)}. Try a different letter.`
										: undefined
							}
							onKeysChange={(next) => {
								setKeys(next);
								add.reset();
							}}
							onAllowFolder={() => {
								if (blockedFolder) allowFolder.mutate(blockedFolder);
							}}
						/>
					)
				: null}
		</StepLayout>
	);
}
