import { Badge } from "#/components/ui/badge.tsx";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "#/components/ui/card.tsx";
import { Input } from "#/components/ui/input.tsx";
import { useState } from "react";
import { taurpc } from "../../api/taurpc.ts";
import { useQuery } from "@tanstack/react-query";

export function ActiveChordsTab() {
  const [searchInput, setSearchInput] = useState("");
  const { data, isSuccess } = useQuery({
    queryKey: ["listActiveChords"],
    queryFn: () => taurpc.listActiveChords(),
  });

  return (
    <Card size="sm">
      <CardHeader className="flex items-center justify-between gap-3">
        <CardTitle>Registered Chords</CardTitle>
        <CardDescription>
          Live view of the chord registry loaded in `context.loaded_app_chords`.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-3 pt-0">
        <div className="flex flex-col gap-3 sm:flex-row sm:items-center">
          <Input
            value={searchInput}
            onChange={(event) => {
              setSearchInput(event.target.value);
            }}
            placeholder="Filter by app, trigger, name, or action"
          />
          {isSuccess && (
            <Badge variant="outline" className="self-start sm:self-center">
              {data.length} matches
            </Badge>
          )}
        </div>

        {/* {isLoading ? (
          <p className="text-sm text-muted-foreground">Loading active chords...</p>
        ) : isSuccess && data.length === 0 ? (
          <p className="text-sm text-muted-foreground">No chords are currently loaded.</p>
        ) : is activeChords.filteredActiveChords.length === 0 ? (
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
        )} */}
      </CardContent>
    </Card>
  );
}
