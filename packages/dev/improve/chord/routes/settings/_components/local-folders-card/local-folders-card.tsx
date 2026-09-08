import { useMutation, useQuery, useQueryClient } from "@chord/com.npmjs.tanstack__react-query";
import { taurpc } from "@chord/dev.improve.chord.api.taurpc";
import { Badge } from "@chord/dev.improve.chord.components.ui.badge";
import { Button } from "@chord/dev.improve.chord.components.ui.button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@chord/dev.improve.chord.components.ui.card";
import { Input } from "@chord/dev.improve.chord.components.ui.input";
import { useState } from "react";

export function LocalFoldersCard() {
  const [folderPath, setFolderPath] = useState("");
  const queryClient = useQueryClient();
  const addLocalChordFolderMutation = useMutation({
    mutationFn: taurpc.addLocalChordFolder,
    onSuccess: async () => {
      setFolderPath("");
      await queryClient.invalidateQueries({
        queryKey: ["local-chord-folders"],
      });
    },
  });
  const pickLocalChordFolderMutation = useMutation({
    mutationFn: async () => {
      const path = await taurpc.pickLocalChordFolder();
      if (path) await taurpc.addLocalChordFolder(path);
    },
    onSuccess: async () => {
      await queryClient.invalidateQueries({
        queryKey: ["local-chord-folders"],
      });
    },
  });
  const localFoldersQuery = useQuery({
    queryKey: ["local-chord-folders"],
    queryFn: taurpc.listLocalChordFolders,
  });
  const folders = localFoldersQuery.data ?? [];
  const isAdding = addLocalChordFolderMutation.isPending || pickLocalChordFolderMutation.isPending;
  const error = addLocalChordFolderMutation.error ?? pickLocalChordFolderMutation.error;

  return (
    <Card size="sm">
      <CardHeader className="flex items-center justify-between gap-3">
        <CardTitle>Local Folders</CardTitle>
        <CardDescription>
          Local folders are loaded in place. Use the tray reload action after editing files to
          rebuild the JS runtime.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4 pt-0">
        <form
          className="flex flex-col gap-3 sm:flex-row"
          onSubmit={(event) => {
            event.preventDefault();
            if (!folderPath.trim() || isAdding) return;
            pickLocalChordFolderMutation.reset();
            addLocalChordFolderMutation.mutate(folderPath.trim());
          }}
        >
          <Input
            aria-label="Chords folder path"
            placeholder="/path/to/chords-folder"
            value={folderPath}
            onChange={(event) => {
              setFolderPath(event.target.value);
            }}
            autoComplete="off"
            spellCheck={false}
            disabled={isAdding}
          />
          <Button type="submit" disabled={isAdding || !folderPath.trim()}>
            {addLocalChordFolderMutation.isPending ? "Adding..." : "Add Folder"}
          </Button>
          <Button
            type="button"
            variant="outline"
            onClick={() => {
              addLocalChordFolderMutation.reset();
              pickLocalChordFolderMutation.mutate();
            }}
            disabled={isAdding}
          >
            {pickLocalChordFolderMutation.isPending ? "Adding..." : "Browse..."}
          </Button>
        </form>
        {error && (
          <p role="alert" className="text-sm text-destructive">
            Failed to add folder:{" "}
            {typeof error === "object" && "Message" in error
              ? String(error.Message)
              : String(error)}
          </p>
        )}

        {localFoldersQuery.isLoading ? (
          <p className="text-sm text-muted-foreground">Loading local folders...</p>
        ) : folders.length === 0 ? (
          <p className="text-sm text-muted-foreground">
            No local folders added yet. Add one to load chords directly from disk.
          </p>
        ) : (
          <div className="space-y-2">
            {folders.map((folder) => (
              <div
                key={folder}
                className="flex items-center justify-between gap-3 rounded-lg border bg-background/80 px-3 py-3"
              >
                <div className="min-w-0">
                  <p className="font-medium">Local Chord Folder</p>
                  <p className="truncate text-xs text-muted-foreground">{folder}</p>
                </div>
                <Badge variant="secondary">Local</Badge>
              </div>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
