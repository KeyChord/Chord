import type { AppMetadataInfo } from "#/api/taurpc.ts";

export function AppIcon({ appMetadata, label }: { appMetadata?: AppMetadataInfo; label: string }) {
  const fallback = label.trim().charAt(0).toUpperCase() || "?";

  return (
    <div className="flex size-5 shrink-0 items-center justify-center overflow-hidden rounded-md border bg-muted text-[10px] font-medium text-muted-foreground">
      {appMetadata?.iconDataUrl ? (
        <img src={appMetadata.iconDataUrl} alt="" className="size-full object-contain" />
      ) : (
        <span aria-hidden="true">{fallback}</span>
      )}
      <span className="sr-only">{label}</span>
    </div>
  );
}
