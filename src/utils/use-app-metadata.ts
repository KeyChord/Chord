import { useEffect, useState } from "react";
import { taurpc, type AppMetadataInfo } from "#/api/taurpc.ts";
import type { AppMetadataByBundleId } from "#/utils/settings.ts";

export function useAppMetadata(bundleIds: string[]) {
  const [appMetadataByBundleId, setAppMetadataByBundleId] = useState<AppMetadataByBundleId>({});
  const bundleIdsKey = bundleIds.join("\u001f");

  useEffect(() => {
    let cancelled = false;

    if (bundleIds.length === 0) {
      setAppMetadataByBundleId({});
      return () => {
        cancelled = true;
      };
    }

    void taurpc
      .listAppMetadata(bundleIds)
      .then((items: AppMetadataInfo[]) => {
        if (cancelled) {
          return;
        }

        setAppMetadataByBundleId(Object.fromEntries(items.map((item) => [item.bundleId, item])));
      })
      .catch((error) => {
        if (!cancelled) {
          console.error("Failed to load app metadata", error);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [bundleIdsKey]);

  return appMetadataByBundleId;
}
