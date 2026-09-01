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

function createUseTauriState<T>(stateId: string, initialState: T) {
	const useTauriState = () => {
		const [state, setState] = useState<T>(initialState);
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
] = await (async () => {
	const initialStates = JSON.parse(await taurpc.getCurrentStates()) as Record<string, unknown>;
	return [
		createUseTauriState<KeyboardState>('keyboard', initialStates.keyboard as KeyboardState),
		createUseTauriState<ChordPanelState>(
			'chord-panel',
			initialStates['chord-panel'] as ChordPanelState,
		),
		createUseTauriState<ChordInputState>(
			'chord-input',
			initialStates['chord-input'] as ChordInputState,
		),
		createUseTauriState<AppSettingsState>('settings', initialStates.settings as AppSettingsState),
		createUseTauriState<AppPermissionsState>(
			'permissions',
			initialStates.permissions as AppPermissionsState,
		),
		createUseTauriState<GitReposState>(
			'git-repos',
			initialStates['git-repos'] as GitReposState,
		),
		createUseTauriState<FrontmostState>(
			'frontmost',
			initialStates.frontmost as FrontmostState,
		),
		createUseTauriState<ChordPackageManagerState>(
			'chord-package-manager',
			initialStates['chord-package-manager'] as ChordPackageManagerState,
		),
		createUseTauriState<DesktopAppManagerState>(
			'desktop-app-manager',
			initialStates['desktop-app-manager'] as DesktopAppManagerState,
		),
		createUseTauriState<ChordPackageStoreState>(
			'chord-package-store',
			initialStates['chord-package-store'] as ChordPackageStoreState,
		),
	] as const;
})();
