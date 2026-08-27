import type {
	AppPermissionsState,
	AppSettingsState,
	ChordInputState,
	ChordPackageManagerState,
	ChordPackageStoreState,
	ChordPanelState,
	DesktopAppManagerState,
	FrontmostState,
	GitReposState,
	KeyboardState,
} from '@chord/dev.improve.chord.lib.typeshare';
import renameFunction from '@chord/com.npmjs.rename-fn';
import { taurpc } from '@chord/dev.improve.chord.api.taurpc';
import { listen } from '@tauri-apps/api/event';
import { useEffect, useState } from 'react';

async function createUseTauriState<T>(stateId: string) {
	const initialStates = JSON.parse(await taurpc.getCurrentStates());
	const useTauriState = () => {
		const [state, setState] = useState<T>(initialStates[stateId]);
		useEffect(() => {
			const unlistenPromise = listen<T>(`state:${stateId}`, (event) => {
				setState(event.payload);
			});

			return () => {
				void unlistenPromise.then(unlisten => unlisten?.());
			};
		}, []);

		return state;
	};

	return renameFunction(useTauriState, stateId);
}

export const [
	useKeyboardState,
	useChordPanelState,
	useChordInputState,
	useSettingsState,
	usePermissionsState,
	useGitRepoStoreState,
	useFrontmostState,
	useChordPackageManagerState,
	useDesktopAppManagerState,
	useChordPackageStoreState,
] = await Promise.all([
	createUseTauriState<KeyboardState>('keyboard'),
	createUseTauriState<ChordPanelState>('chord-panel'),
	createUseTauriState<ChordInputState>('chord-input'),
	createUseTauriState<AppSettingsState>('settings'),
	createUseTauriState<AppPermissionsState>('permissions'),
	createUseTauriState<GitReposState>('git-repos'),
	createUseTauriState<FrontmostState>('frontmost'),
	createUseTauriState<ChordPackageManagerState>('chord-package-manager'),
	createUseTauriState<DesktopAppManagerState>('desktop-app-manager'),
	createUseTauriState<ChordPackageStoreState>('chord-package-store'),
]);
