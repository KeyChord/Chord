import type {
	AppPermissionsState,
	AppSettingsState,
	ChorderState,
	ChordPackageManagerState,
	ChordPackageStoreState,
	DesktopAppManagerState,
	FrontmostState,
	GitReposState,
} from '#/types/generated.ts';
import { taurpc } from '#/api/taurpc.ts';
import { listen } from '@tauri-apps/api/event';
import { useEffect, useState } from 'react';
import renameFunction from 'rename-fn';

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
  useChorderState,
  useSettingsState,
  usePermissionsState,
  useGitRepoStoreState,
  useFrontmostState,
  useChordPackageManagerState,
  useDesktopAppManagerState,
  useChordPackageStoreState
] = await Promise.all([
  createUseTauriState<ChorderState>('chorder'),
  createUseTauriState<AppSettingsState>('settings'),
  createUseTauriState<AppPermissionsState>('permissions'),
  createUseTauriState<GitReposState>('git-repos'),
  createUseTauriState<FrontmostState>('frontmost'),
  createUseTauriState<ChordPackageManagerState>('chord-package-manager'),
  createUseTauriState<DesktopAppManagerState>('desktop-app-manager'),
  createUseTauriState<ChordPackageStoreState>('chord-package-store'),
]);
