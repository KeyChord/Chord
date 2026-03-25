import {
  createTauRPCProxy,
  type GlobalShortcutMappingInfo,
  type StartupStatusInfo,
} from "#/api/bindings.gen.ts";

export const taurpc = createTauRPCProxy();

export type {
  GlobalShortcutMappingInfo,
  StartupStatusInfo,
};
