import { useEffect, useMemo, useState } from "react";
import { useMutation, useQueries } from "@tanstack/react-query";
import { toast } from "sonner";
import { AppIcon } from "#/components/settings/app-icon.tsx";
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
import { taurpc } from "#/api/taurpc.ts";
import { useChordFilesState } from "#/utils/state.ts";

export function PlaceholderChordsCard() {
  const [input, setInput] = useState("");
  const { placeholderChords } = useChordFilesState();
  const appBundleIds = useMemo(
    () =>
      [...new Set(placeholderChords.filter((entry) => entry.scopeKind === "app").map((entry) => entry.scope))].sort(),
    [placeholderChords],
  );
  const appMetadataQueries = useQueries({
    queries: appBundleIds.map((bundleId) => ({
      queryKey: ["app-metadata", bundleId],
      queryFn: () => taurpc.getAppMetadata(bundleId),
      staleTime: 60_000,
    })),
  });
  const appMetadataByBundleId = Object.fromEntries(
    appBundleIds.map((bundleId, index) => [bundleId, appMetadataQueries[index]?.data]),
  );
  const normalizedFilter = input.trim().toLowerCase();
  const filteredPlaceholders = placeholderChords.filter((placeholder) => {
    if (!normalizedFilter) {
      return true;
    }

    const appLabel =
      placeholder.scopeKind === "app"
        ? appMetadataByBundleId[placeholder.scope]?.displayName?.trim() || placeholder.scope
        : "Global";

    return [
      placeholder.name,
      placeholder.placeholder,
      placeholder.sequenceTemplate,
      placeholder.assignedSequence ?? "",
      placeholder.scope,
      appLabel,
      placeholder.filePath,
    ].some((value) => value.toLowerCase().includes(normalizedFilter));
  });

  return (
    <Card size="sm">
      <CardHeader className="flex items-center justify-between gap-3">
        <CardTitle>Placeholder Chords</CardTitle>
        <CardDescription>
          Configure letter sequences for template chords like <code>{`\\<Dark Mode>`}</code>.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-3 pt-0">
        <div className="flex flex-col gap-3 sm:flex-row sm:items-center">
          <Input
            value={input}
            onChange={(event) => {
              setInput(event.target.value);
            }}
            placeholder="Filter by app, chord, placeholder, sequence, or file"
          />
          <Badge variant="outline" className="self-start sm:self-center">
            {placeholderChords.length} placeholders
          </Badge>
        </div>

        {placeholderChords.length === 0 ? (
          <p className="text-sm text-muted-foreground">No placeholder chords are currently loaded.</p>
        ) : filteredPlaceholders.length === 0 ? (
          <p className="text-sm text-muted-foreground">No placeholder chords match that filter.</p>
        ) : (
          <div className="space-y-2">
            {filteredPlaceholders.map((placeholder) => {
              const appMetadata =
                placeholder.scopeKind === "app"
                  ? appMetadataByBundleId[placeholder.scope]
                  : undefined;
              const appLabel =
                placeholder.scopeKind === "app"
                  ? appMetadata?.displayName?.trim() || placeholder.scope
                  : "Global";

              return (
                <PlaceholderChordRow
                  key={`${placeholder.filePath}:${placeholder.sequenceTemplate}`}
                  appLabel={appLabel}
                  appMetadata={appMetadata}
                  placeholder={placeholder}
                />
              );
            })}
          </div>
        )}
      </CardContent>
    </Card>
  );
}

function PlaceholderChordRow({
  appLabel,
  appMetadata,
  placeholder,
}: {
  appLabel: string;
  appMetadata?: Awaited<ReturnType<typeof taurpc.getAppMetadata>>;
  placeholder: ReturnType<typeof useChordFilesState>["placeholderChords"][number];
}) {
  const [draftSequence, setDraftSequence] = useState(placeholder.assignedSequence ?? "");
  const setPlaceholderChordBindingMutation = useMutation({
    mutationFn: (sequence: string) =>
      taurpc.setPlaceholderChordBinding(
        placeholder.filePath,
        placeholder.sequenceTemplate,
        sequence,
      ),
    onError: (error) => {
      toast.error(error.message);
    },
  });
  const removePlaceholderChordBindingMutation = useMutation({
    mutationFn: () =>
      taurpc.removePlaceholderChordBinding(placeholder.filePath, placeholder.sequenceTemplate),
    onError: (error) => {
      toast.error(error.message);
    },
  });
  const isPending =
    setPlaceholderChordBindingMutation.isPending || removePlaceholderChordBindingMutation.isPending;
  const normalizedDraftSequence = draftSequence.trim().toLowerCase();
  const assignedSequence = placeholder.assignedSequence ?? "";
  const isDirty = normalizedDraftSequence !== assignedSequence;
  const hasDraftSequence = normalizedDraftSequence.length > 0;
  const hasAssignedSequence = assignedSequence.length > 0;

  useEffect(() => {
    setDraftSequence(placeholder.assignedSequence ?? "");
  }, [placeholder.assignedSequence]);

  return (
    <div className="rounded-lg border bg-background/80 px-3 py-3">
      <div className="flex flex-col gap-3 lg:flex-row lg:items-center">
        <div className="min-w-0 flex-1 space-y-2">
          <div className="flex min-w-0 items-center gap-2">
            <AppIcon appMetadata={appMetadata} label={appLabel} tooltip={placeholder.scope} />
            <p className="min-w-0 truncate text-sm">
              <span className="font-medium text-foreground">{appLabel}</span>
              <span className="mx-2 text-muted-foreground">&gt;</span>
              <span className="truncate text-muted-foreground">{placeholder.name}</span>
            </p>
          </div>
          <div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
            <Badge variant="outline">{placeholder.placeholder}</Badge>
            <span className="rounded-md bg-muted px-2 py-1 font-mono text-[11px] text-foreground/80">
              {placeholder.sequenceTemplate}
            </span>
            <span className="truncate">{placeholder.filePath}</span>
          </div>
        </div>

        <div className="flex flex-col gap-2 sm:flex-row sm:items-center">
          <div className="flex h-8 min-w-48 items-center rounded-lg border border-input bg-background px-2.5 shadow-xs">
            {placeholder.sequencePrefix ? (
              <span className="shrink-0 font-mono text-xs text-muted-foreground">
                {placeholder.sequencePrefix}
              </span>
            ) : null}
            <input
              type="text"
              value={draftSequence}
              onChange={(event) => {
                setDraftSequence(event.target.value.replace(/[^a-z]/gi, "").toLowerCase());
              }}
              placeholder="letters"
              className="min-w-0 flex-1 bg-transparent px-2 font-mono text-sm outline-none placeholder:text-muted-foreground"
            />
            {placeholder.sequenceSuffix ? (
              <span className="shrink-0 font-mono text-xs text-muted-foreground">
                {placeholder.sequenceSuffix}
              </span>
            ) : null}
          </div>

          {isDirty ? (
            <>
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={() => {
                  setDraftSequence(assignedSequence);
                }}
                disabled={isPending}
              >
                Cancel
              </Button>
              <Button
                type="button"
                size="sm"
                onClick={() => {
                  if (!hasDraftSequence) {
                    return;
                  }

                  setPlaceholderChordBindingMutation.mutate(normalizedDraftSequence);
                }}
                disabled={isPending || !hasDraftSequence}
              >
                {setPlaceholderChordBindingMutation.isPending ? "Saving..." : "Save"}
              </Button>
            </>
          ) : hasAssignedSequence ? (
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={() => {
                removePlaceholderChordBindingMutation.mutate();
              }}
              disabled={isPending}
            >
              {removePlaceholderChordBindingMutation.isPending ? "Disabling..." : "Disable"}
            </Button>
          ) : null}
        </div>
      </div>
    </div>
  );
}
