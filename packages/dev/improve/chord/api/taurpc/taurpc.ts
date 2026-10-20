import type {
	ChordsFileProblem,
	GlobalShortcutMappingInfo,
	PersonalPackageFiles,
	PersonalPackages,
} from '#bindings';
import {
	createTauRPCProxy,

} from '#bindings';

export const taurpc = createTauRPCProxy();

export type {
	ChordsFileProblem,
	GlobalShortcutMappingInfo,
	PersonalPackageFiles,
	PersonalPackages,
};
