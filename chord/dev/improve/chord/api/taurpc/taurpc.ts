import type { GlobalShortcutMappingInfo } from '#bindings';
import {
	createTauRPCProxy,

} from '#bindings';

export const taurpc = createTauRPCProxy();

export type {
	GlobalShortcutMappingInfo,
};
