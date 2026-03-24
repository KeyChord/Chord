import {
  createTauRPCProxy,
  type AppMetadataInfo,
  type AppNeedsRelaunchInfo,
  type GlobalShortcutMappingInfo,
  type StartupStatusInfo,
} from "#/api/bindings.gen.ts";

export const taurpc = createTauRPCProxy();

export type {
  AppMetadataInfo,
  AppNeedsRelaunchInfo,
  GlobalShortcutMappingInfo,
  StartupStatusInfo,
};
