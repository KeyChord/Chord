import type { GlobalShortcutMappingInfo, StartupStatusInfo } from '#/api/bindings.gen.ts';
import {
	createTauRPCProxy,

} from '#/api/bindings.gen.ts';

export const taurpc = createTauRPCProxy();

export type {
	GlobalShortcutMappingInfo,
	StartupStatusInfo,
};
