import { useQuery } from "@tanstack/react-query";
import { taurpc } from "../api/taurpc.ts";

export function useAppMetadataQuery(bundleId: string) {
  return useQuery({
    queryKey: ["appMetadata", bundleId],
    queryFn: () => taurpc.getAppMetadata(bundleId),
  })
}