import type {
	AppPermissionsState,
	AppSettingsState,
	ChorderState,
	ChordFilesState,
	ChordPackageManagerState,
	DesktopAppManagerState,
	FrontmostState,
	GitReposState,
} from '../types/generated.ts';
import { listen } from '@tauri-apps/api/event';
import { useEffect, useState } from 'react';
import renameFunction from 'rename-fn';
import { taurpc } from '../api/taurpc.ts';

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

export const useChorderState = await createUseTauriState<ChorderState>('chorder');
export const useSettingsState = await createUseTauriState<AppSettingsState>('settings');
export const usePermissionsState = await createUseTauriState<AppPermissionsState>('permissions');
export const useGitRepoStoreState = await createUseTauriState<GitReposState>('git-repos');
export const useFrontmostState = await createUseTauriState<FrontmostState>('frontmost');
export const useChordPackageManagerState = await createUseTauriState<ChordPackageManagerState>('chord-package-manager');
export const useDesktopAppManagerState = await createUseTauriState<DesktopAppManagerState>('desktop-app-manager');
