import { Badge } from "#/components/ui/badge.tsx";
import { Button } from "#/components/ui/button.tsx";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "#/components/ui/card.tsx";
import { Input } from "#/components/ui/input.tsx";
import { ActiveChordTree } from "#/components/settings/chord-views.tsx";
import type { SettingsPageData } from "#/utils/use-settings-page.ts";

export function ActiveChordsTab({
  activeChords,
  appMetadataByBundleId,
}: {
  activeChords: SettingsPageData["activeChordsTab"];
  appMetadataByBundleId: SettingsPageData["appMetadataByBundleId"];
}) {
  return (
    <Card size="sm">
      <CardHeader>
        <div className="flex items-center justify-between gap-3">
          <div>
            <CardTitle>Registered Chords</CardTitle>
            <CardDescription>
              Live view of the chord registry loaded in `context.loaded_app_chords`.
            </CardDescription>
          </div>
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={() => {
              void activeChords.refreshActiveChords({ showSuccessToast: true });
            }}
            disabled={activeChords.activeChordsBusy}
          >
            {activeChords.activeChordsBusy ? "Refreshing..." : "Refresh"}
          </Button>
        </div>
      </CardHeader>
      <CardContent className="space-y-3 pt-0">
        <div className="flex flex-col gap-3 sm:flex-row sm:items-center">
          <Input
            value={activeChords.chordSearch}
            onChange={(event) => {
              activeChords.setChordSearch(event.target.value);
            }}
            placeholder="Filter by app, trigger, name, or action"
          />
          <Badge variant="outline" className="self-start sm:self-center">
            {activeChords.filteredActiveChords.length} matches
          </Badge>
        </div>

        {activeChords.activeChordsBusy ? (
          <p className="text-sm text-muted-foreground">Loading active chords...</p>
        ) : activeChords.activeChords.length === 0 ? (
          <p className="text-sm text-muted-foreground">No chords are currently loaded.</p>
        ) : activeChords.filteredActiveChords.length === 0 ? (
          <p className="text-sm text-muted-foreground">No chords match that filter.</p>
        ) : (
          <ActiveChordTree
            groups={activeChords.chordGroups}
            forceExpandAll={activeChords.normalizedChordSearch.length > 0}
            appMetadataByBundleId={appMetadataByBundleId}
            openGroups={activeChords.openChordGroups}
            onGroupOpenChange={(groupKey, open) => {
              activeChords.setChordGroupOpen(groupKey, open);
            }}
          />
        )}
      </CardContent>
    </Card>
  );
}
