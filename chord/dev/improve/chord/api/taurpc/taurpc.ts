import type { GlobalShortcutMappingInfo, StartupStatusInfo } from '#bindings';
import {
	createTauRPCProxy,

} from '#bindings';

export const taurpc = createTauRPCProxy();

export type {
	GlobalShortcutMappingInfo,
	StartupStatusInfo,
};
