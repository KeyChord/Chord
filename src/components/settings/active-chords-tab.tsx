import { Badge } from '#/components/ui/badge.tsx';
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from '#/components/ui/card.tsx';
import { Input } from '#/components/ui/input.tsx';
import { useState, useMemo } from 'react';
import { useChordPackageManagerState } from '../../utils/state.ts';
import { useDesktopAppManagerState } from '../../utils/state.ts';
import { ActiveChordTree } from './active-chords-tree.tsx';
import type { ChordPackage, ChordsFile, Chord, AppBundleId, DesktopAppMetadata } from '#/types/generated.ts';

// Helper function to normalize strings for case-insensitive comparison
const normalizeString = (str: string): string => str.trim().toLowerCase();

export function ActiveChordsTab() {
	const [searchInput, setSearchInput] = useState('');
	const { packages } = useChordPackageManagerState();
	const { appsMetadata } = useDesktopAppManagerState();

	const normalizedFilter = normalizeString(searchInput);

	// Derive activeChords state from packages and searchInput
	const activeChords = useMemo(() => {
		let filteredActiveChords: any[] = []; // Placeholder type, ideally use a specific type
		let chordGroups: any[] = []; // Placeholder type

		packages.forEach((pkg: ChordPackage) => {
			// Process global chords
			Object.entries(pkg.globalChords).forEach(([stringKey, chord]) => {
				const matchesFilter =
					normalizeString(chord.name).includes(normalizedFilter) ||
					normalizeString(chord.string_key).includes(normalizedFilter) ||
					chord.actions.some(action => normalizeString(JSON.stringify(action)).includes(normalizedFilter)); // Basic check for actions

				if (matchesFilter) {
					filteredActiveChords.push({
						pkgName: pkg.name,
						scope: 'global',
						scopeKind: 'global',
						chord: chord,
					});
				}
			});

			// Process app-specific chords
			Object.entries(pkg.appChordsFiles).forEach(([appBundleId, chordsFile]) => {
				const appMetadata = appsMetadata[appBundleId as AppBundleId];
				const appLabel = appMetadata?.displayName?.trim() || appBundleId;

				chordsFile.chords.forEach((chord: Chord) => {
					const matchesFilter =
						normalizeString(chord.name).includes(normalizedFilter) ||
						normalizeString(chord.string_key).includes(normalizedFilter) ||
						normalizeString(appLabel).includes(normalizedFilter) ||
						chord.actions.some(action => normalizeString(JSON.stringify(action)).includes(normalizedFilter)); // Basic check for actions

					if (matchesFilter) {
						filteredActiveChords.push({
							pkgName: pkg.name,
							scope: appBundleId,
							scopeKind: 'app',
							chord: chord,
						});
					}
				});
			});
		});

		// Grouping logic would go here if needed, for now assume flat list is fine
		// For simplicity, directly using filtered list and mapping to groups conceptually
		chordGroups = [{
			key: 'all', // A single group for simplicity
			name: 'All Chords',
			chords: filteredActiveChords.map(item => ({
				...item.chord,
				pkgName: item.pkgName,
				scope: item.scope,
				scopeKind: item.scopeKind,
			}))
		}];


		return {
			filteredActiveChords,
			chordGroups,
			normalizedChordSearch: normalizedFilter,
			openChordGroups: {}, // Initialize as empty, state management for open groups would be complex
			setChordGroupOpen: (_groupKey: string, _open: boolean) => {}, // Placeholder function
		};
	}, [packages, normalizedFilter, appsMetadata]);

	const isLoading = false; // Assuming data is available immediately from hooks
	const isSuccess = true; // Assuming data is available immediately from hooks

	return (
		<Card size="sm">
			<CardHeader className="flex items-center justify-between gap-3">
				<CardTitle>Registered Chords</CardTitle>
				<CardDescription>
					Live view of the chord registry loaded in `context.loaded_app_chords`.
				</CardDescription>
			</CardHeader>
			<CardContent className="space-y-3 pt-0">
				<div className="flex flex-col gap-3 sm:flex-row sm:items-center">
					<Input
						value={searchInput}
						onChange={(event) => {
							setSearchInput(event.target.value);
						}}
						placeholder="Filter by app, trigger, name, or action"
					/>
					<Badge variant="outline" className="self-start sm:self-center">
						{' '}
						matches
					</Badge>
				</div>

				{isLoading ? (
          <p className="text-sm text-muted-foreground">Loading active chords...</p>
        ) : packages.length === 0 ? ( // Check if packages are loaded
          <p className="text-sm text-muted-foreground">No chord packages are currently loaded.</p>
        ) : activeChords.filteredActiveChords.length === 0 ? (
          <p className="text-sm text-muted-foreground">No chords match that filter.</p>
        ) : (
          <ActiveChordTree
            groups={activeChords.chordGroups}
            forceExpandAll={activeChords.normalizedChordSearch.length > 0}
            appMetadataByBundleId={appMetadataByBundleId}
            openGroups={activeChords.openChordGroups}
            onGroupOpenChange={(groupKey, open) => {
              activeChords.setChordGroupOpen(groupKey, open);
            }}
          />
        )}
			</CardContent>
		</Card>
	);
}
