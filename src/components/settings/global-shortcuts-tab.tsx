import { X } from "lucide-react";
import { Button } from "#/components/ui/button.tsx";
import { Badge } from "#/components/ui/badge.tsx";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "#/components/ui/card.tsx";
import { Input } from "#/components/ui/input.tsx";
import { AppIcon } from "#/components/settings/app-icon.tsx";
import { ShortcutKeys } from "#/components/settings/shortcut-keys.tsx";
import { getAppLabel } from "#/utils/settings.ts";
import type { SettingsPageData } from "#/utils/use-settings-page.ts";
import { useMutation, useQueries, useQuery } from "@tanstack/react-query";
import { taurpc } from "../../api/taurpc.ts";
import { useEffect, useState } from "react";

export function GlobalShortcutsTab() {
  const [input, setInput] = useState("");
  const removeGlobalShortcutMappingMutation = useMutation({
    mutationFn: taurpc.removeGlobalShortcutMapping
  })
  const { data, isSuccess, isLoading } = useQuery({
    queryKey: ["global-shortcuts"],
    queryFn: () => taurpc.listGlobalShortcutMappings()
  })

  return (
    <Card size="sm">
      <CardHeader className="flex items-center justify-between gap-3">
        <CardTitle>Global Shortcut Mappings</CardTitle>
        <CardDescription>
          Current shortcut assignments stored in `global-hotkeys.json`.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-3 pt-0">
        <div className="flex flex-col gap-3 sm:flex-row sm:items-center">
          <Input
            value={input}
            onChange={(event) => {
              setInput(event.target.value);
            }}
            placeholder="Filter by shortcut, app, bundle ID, or hotkey ID"
          />
          {isSuccess && (
            <Badge variant="outline" className="self-start sm:self-center">
              {data.length} mappings
            </Badge>
          )}
        </div>

        {/* {isLoading ? (
          <p className="text-sm text-muted-foreground">Loading global shortcut mappings...</p>
        ) : globalShortcutMappings.length === 0 ? (
          <p className="text-sm text-muted-foreground">
            No global shortcut mappings are currently registered.
          </p>
        ) : globalShortcuts.filteredGlobalShortcutMappings.length === 0 ? (
          <p className="text-sm text-muted-foreground">No global shortcut mappings match that filter.</p>
        ) : (
          <div className="space-y-2">
            {globalShortcuts.filteredGlobalShortcutMappings.map((mapping) => {
              const isRemoving = globalShortcuts.removingGlobalShortcut === mapping.shortcut;
              const appMetadata = appMetadataByBundleId[mapping.bundleId];
              const appLabel = getAppLabel(mapping.bundleId, appMetadata);

              return (
                <div
                  key={mapping.shortcut}
                  className="flex items-start justify-between gap-3 rounded-lg border bg-background/80 px-3 py-3"
                >
                  <div className="min-w-0 space-y-2">
                    <ShortcutKeys shortcut={mapping.shortcut} />
                    <div className="flex items-center gap-2">
                      <AppIcon appMetadata={appMetadata} label={appLabel} />
                      <div className="min-w-0">
                        <p className="truncate text-sm font-medium text-foreground">{appLabel}</p>
                        {appLabel !== mapping.bundleId ? (
                          <p className="truncate text-xs text-muted-foreground">{mapping.bundleId}</p>
                        ) : null}
                      </div>
                    </div>
                    <div className="space-y-1 text-xs text-muted-foreground">
                      <p className="break-all">
                        <span className="font-medium text-foreground">Hotkey:</span>{" "}
                        {mapping.hotkeyId}
                      </p>
                    </div>
                  </div>

                  <Button
                    type="button"
                    variant="ghost"
                    size="icon-sm"
                    aria-label={`Remove ${mapping.shortcut}`}
                    title="Remove mapping"
                    onClick={() => {
                      removeGlobalShortcutMappingMutation.mutate(mapping.shortcut);
                    }}
                    disabled={removeGlobalShortcutMappingMutation.isPending}
                    className="text-muted-foreground hover:text-destructive"
                  >
                    <X />
                  </Button>
                </div>
              );
            })}
          </div>
        )} */}
      </CardContent>
    </Card>
  );
}
