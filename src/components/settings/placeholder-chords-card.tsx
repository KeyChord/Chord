import type { DesktopAppMetadata } from '../../types/generated.ts';
import { taurpc } from '#/api/taurpc.ts';
import { AppIcon } from '#/components/settings/app-icon.tsx';
import { Badge } from '#/components/ui/badge.tsx';
import { Button } from '#/components/ui/button.tsx';
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from '#/components/ui/card.tsx';
import { Input } from '#/components/ui/input.tsx';
import { useChordPackageManagerState, useDesktopAppManagerState } from '#/utils/state.ts';
import { useMutation } from '@tanstack/react-query';
import { useState } from 'react';

const LETTERS_ONLY_REGEX = /[^a-z]/gi;

export function PlaceholderChordsCard() {
	const [input, setInput] = useState('');
	// Destructure placeholderChords from the hook. This assumes the hook actually provides it.
	// The type definition might be outdated.
	const { placeholderChords = [] } = useChordPackageManagerState() as any; // Using 'as any' for now to bridge potential type gap.
	const { appsMetadata } = useDesktopAppManagerState();
	const normalizedFilter = input.trim().toLowerCase();

	// Populate filteredPlaceholders based on placeholderChords and input
	const filteredPlaceholders: PlaceholderChordInfo[] = placeholderChords.filter((placeholder: PlaceholderChordInfo) => {
		const appLabel = placeholder.scopeKind === 'app' && placeholder.scope in appsMetadata
			? appsMetadata[placeholder.scope]?.displayName?.trim() || placeholder.scope
			: 'Global';

		return (
			placeholder.name.toLowerCase().includes(normalizedFilter) ||
			placeholder.placeholder.toLowerCase().includes(normalizedFilter) ||
			placeholder.sequenceTemplate.toLowerCase().includes(normalizedFilter) ||
			placeholder.filePath.toLowerCase().includes(normalizedFilter) ||
			appLabel.toLowerCase().includes(normalizedFilter)
		);
	});

	return (
		<Card size="sm">
			<CardHeader className="flex items-center justify-between gap-3">
				<CardTitle>Placeholder Chords</CardTitle>
				<CardDescription>
					Configure letter sequences for template chords like
					{' '}
					<code>{`\\<Dark Mode>`}</code>
					.
				</CardDescription>
			</CardHeader>
			<CardContent className="space-y-3 pt-0">
				<div className="flex flex-col gap-3 sm:flex-row sm:items-center">
					<Input
						value={input}
						onChange={(event) => {
							setInput(event.target.value);
						}}
						placeholder="Filter by app, chord, placeholder, sequence, or file"
					/>
					<Badge variant="outline" className="self-start sm:self-center">
						{' '}
						placeholders
					</Badge>
				</div>

				{placeholderChords.length === 0
					? (
							<p className="text-sm text-muted-foreground">No placeholder chords are currently loaded.</p>
						)
					: filteredPlaceholders.length === 0
						? (
								<p className="text-sm text-muted-foreground">No placeholder chords match that filter.</p>
							)
						: (
								<div className="space-y-2">
									{filteredPlaceholders.map((placeholder) => {
										const appMetadata
											= placeholder.scopeKind === 'app' && placeholder.scope in appsMetadata
												? appsMetadata[placeholder.scope]
												: undefined;
										const appLabel
											= placeholder.scopeKind === 'app' && placeholder.scope in appsMetadata
												? appMetadata?.displayName?.trim() || placeholder.scope
												: 'Global';

										return (
											<PlaceholderChordRow
												key={`${placeholder.filePath}:${placeholder.sequenceTemplate}:${placeholder.assignedSequence ?? ''}`}
												appLabel={appLabel}
												appMetadata={appMetadata}
												placeholder={placeholder}
											/>
										);
									})}
								</div>
							)}
			</CardContent>
		</Card>
	);
}

function PlaceholderChordRow({
	appLabel,
	appMetadata,
	placeholder,
}: {
	appLabel: string
	appMetadata?: DesktopAppMetadata
	placeholder: any
}) {
	const [draftSequence, setDraftSequence] = useState(placeholder.assignedSequence ?? '');
	const setPlaceholderChordBindingMutation = useMutation({
		mutationFn: (sequence: string) =>
			taurpc.setPlaceholderChordBinding(
				placeholder.filePath,
				placeholder.sequenceTemplate,
				sequence,
			),
	});
	const removePlaceholderChordBindingMutation = useMutation({
		mutationFn: () =>
			taurpc.removePlaceholderChordBinding(placeholder.filePath, placeholder.sequenceTemplate),
	});
	const isPending
		= setPlaceholderChordBindingMutation.isPending || removePlaceholderChordBindingMutation.isPending;
	const normalizedDraftSequence = draftSequence.trim().toLowerCase();
	const assignedSequence = placeholder.assignedSequence ?? '';
	const isDirty = normalizedDraftSequence !== assignedSequence;
	const hasDraftSequence = normalizedDraftSequence.length > 0;
	const hasAssignedSequence = assignedSequence.length > 0;

	return (
		<div className="rounded-lg border bg-background/80 px-3 py-3">
			<div className="flex flex-col gap-3 lg:flex-row lg:items-center">
				<div className="min-w-0 flex-1 space-y-2">
					<div className="flex min-w-0 items-center gap-2">
						<AppIcon appMetadata={appMetadata} label={appLabel} tooltip={placeholder.scope} />
						<p className="min-w-0 truncate text-sm">
							<span className="font-medium text-foreground">{appLabel}</span>
							<span className="mx-2 text-muted-foreground">&gt;</span>
							<span className="truncate text-muted-foreground">{placeholder.name}</span>
						</p>
					</div>
					<div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
						<Badge variant="outline">{placeholder.placeholder}</Badge>
						<span className="rounded-md bg-muted px-2 py-1 font-mono text-[11px] text-foreground/80">
							{placeholder.sequenceTemplate}
						</span>
						<span className="truncate">{placeholder.filePath}</span>
					</div>
				</div>

				<div className="flex flex-col gap-2 sm:flex-row sm:items-center">
					<div className="flex h-8 min-w-48 items-center rounded-lg border border-input bg-background px-2.5 shadow-xs">
						{placeholder.sequencePrefix
							? (
									<span className="shrink-0 font-mono text-xs text-muted-foreground">
										{placeholder.sequencePrefix}
									</span>
								)
							: null}
						<input
							type="text"
							value={draftSequence}
							onChange={(event) => {
								setDraftSequence(event.target.value.replace(LETTERS_ONLY_REGEX, '').toLowerCase());
							}}
							placeholder="letters"
							className="min-w-0 flex-1 bg-transparent px-2 font-mono text-sm outline-none placeholder:text-muted-foreground"
						/>
						{placeholder.sequenceSuffix
							? (
									<span className="shrink-0 font-mono text-xs text-muted-foreground">
										{placeholder.sequenceSuffix}
									</span>
								)
							: null}
					</div>

					{isDirty
						? (
								<>
									<Button
										type="button"
										variant="outline"
										size="sm"
										onClick={() => {
											setDraftSequence(assignedSequence);
										}}
										disabled={isPending}
									>
										Cancel
									</Button>
									<Button
										type="button"
										size="sm"
										onClick={() => {
											if (!hasDraftSequence) {
												return;
											}

											setPlaceholderChordBindingMutation.mutate(normalizedDraftSequence);
										}}
										disabled={isPending || !hasDraftSequence}
									>
										{setPlaceholderChordBindingMutation.isPending ? 'Saving...' : 'Save'}
									</Button>
								</>
							)
						: hasAssignedSequence
							? (
									<Button
										type="button"
										variant="outline"
										size="sm"
										onClick={() => {
											removePlaceholderChordBindingMutation.mutate();
										}}
										disabled={isPending}
									>
										{removePlaceholderChordBindingMutation.isPending ? 'Disabling...' : 'Disable'}
									</Button>
								)
							: null}
				</div>
			</div>
		</div>
	);
}
