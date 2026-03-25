import { AppsNeedingRelaunchCard } from './apps-needing-relaunch-card.tsx';
import { ChordReposCard } from './chord-repos-card.tsx';
import { LocalFoldersCard } from './local-folders-card.tsx';
import { PlaceholderChordsCard } from './placeholder-chords-card.tsx';

export function ChordsTab() {
	return (
		<div className="space-y-4">
			<PlaceholderChordsCard />
			<ChordReposCard />
			<LocalFoldersCard />
			<AppsNeedingRelaunchCard />
		</div>
	);
}
