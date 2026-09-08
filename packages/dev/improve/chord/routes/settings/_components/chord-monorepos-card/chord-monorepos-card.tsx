import { useMutation, useQuery, useQueryClient } from "@chord/com.npmjs.tanstack__react-query";
import { toast } from "@chord/com.npmjs.sonner";
import { taurpc } from "@chord/dev.improve.chord.api.taurpc";
import { Badge } from "@chord/dev.improve.chord.components.ui.badge";
import { Button } from "@chord/dev.improve.chord.components.ui.button";
import { Input } from "@chord/dev.improve.chord.components.ui.input";
import { ChordReposCard, MonorepoPackagePicker } from "@chord/dev.improve.chord.routes.settings._components.chord-repos-card";
import { Link } from "lucide-react";
import { useState } from "react";

export function ChordMonoreposCard() {
  const [path, setPath] = useState("");
  const queryClient = useQueryClient();
  const queryKey = ["local-chord-monorepos"];
  const localMonorepos = useQuery({ queryKey, queryFn: taurpc.listLocalChordMonorepos });
  const addMutation = useMutation({
    mutationFn: taurpc.addLocalChordMonorepo,
    onSuccess: () => setPath(""),
    onSettled: () => queryClient.invalidateQueries({ queryKey }),
  });
  const removeMutation = useMutation({
    mutationFn: taurpc.removeLocalChordMonorepo,
    onSettled: () => queryClient.invalidateQueries({ queryKey }),
  });
  const reloadMutation = useMutation({
    mutationFn: taurpc.reloadChords,
    onSuccess: () => {
      toast.success("Chords reloaded.");
      return queryClient.invalidateQueries({ queryKey: ["monorepo-packages"] });
    },
  });
  const pending = addMutation.isPending || removeMutation.isPending || reloadMutation.isPending;
  const error =
    addMutation.error ?? removeMutation.error ?? reloadMutation.error ?? localMonorepos.error;
  function resetErrors() {
    addMutation.reset();
    removeMutation.reset();
    reloadMutation.reset();
  }
  return (
    <ChordReposCard monorepos>
      <div className="space-y-3 border-t pt-4">
        <p className="text-sm font-medium">Local monorepos</p>
        <form
          className="flex gap-2"
          onSubmit={(event) => {
            event.preventDefault();
            if (!path.trim() || pending) return;
            resetErrors();
            addMutation.mutate(path.trim());
          }}
        >
          <Input
            aria-label="Local chord monorepo path"
            placeholder="/path/to/monorepo"
            value={path}
            onChange={(event) => setPath(event.target.value)}
            disabled={pending}
            autoComplete="off"
            spellCheck={false}
          />
          <Button
            type="submit"
            variant="outline"
            size="icon-sm"
            disabled={pending || !path.trim()}
            aria-label="Link local chord monorepo"
            title="Link Local Monorepo"
          >
            <Link aria-hidden="true" />
          </Button>
        </form>
        <p className="text-xs text-muted-foreground">
          Link the monorepo root, then select packages below. Reload after editing files or adding and removing packages.
        </p>
        {localMonorepos.isLoading && (
          <p className="text-sm text-muted-foreground">Loading local monorepos...</p>
        )}
        {!localMonorepos.isLoading && !localMonorepos.error && !localMonorepos.data?.length && (
          <p className="text-sm text-muted-foreground">No local monorepos linked yet.</p>
        )}
        {localMonorepos.data?.map((folder) => (
          <div
            key={folder}
            className="flex flex-col gap-3 rounded-lg border bg-background/80 p-3 sm:flex-row sm:items-center sm:justify-between"
          >
            <div className="min-w-0 space-y-1">
              <Badge variant="secondary">Linked locally</Badge>
              <p className="break-all text-xs text-muted-foreground">{folder}</p>
              <MonorepoPackagePicker source={folder} disabled={pending} />
            </div>
            <div className="flex gap-2">
              <Button
                variant="outline"
                disabled={pending}
                onClick={() => {
                  resetErrors();
                  reloadMutation.mutate();
                }}
              >
                {reloadMutation.isPending ? "Reloading..." : "Reload"}
              </Button>
              <Button
                variant="outline"
                disabled={pending}
                onClick={() => {
                  resetErrors();
                  removeMutation.mutate(folder);
                }}
              >
                Unlink
              </Button>
            </div>
          </div>
        ))}
        {error && (
          <p role="alert" className="text-sm text-destructive">
            {typeof error === "object" && "Message" in error
              ? String(error.Message)
              : String(error)}
          </p>
        )}
      </div>
    </ChordReposCard>
  );
}
