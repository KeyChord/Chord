import {
  createTauRPCProxy,
  type ActiveChordInfo,
  type AppMetadataInfo,
  type AppNeedsRelaunchInfo,
  type GitRepoInfo,
  type GlobalShortcutMappingInfo,
  type LocalChordFolderInfo,
} from "#/api/bindings.gen.ts";

export const taurpc = createTauRPCProxy();

export type {
  ActiveChordInfo,
  AppMetadataInfo,
  AppNeedsRelaunchInfo,
  GitRepoInfo,
  GlobalShortcutMappingInfo,
  LocalChordFolderInfo,
};
