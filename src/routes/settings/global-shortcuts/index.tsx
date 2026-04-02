import type { DesktopAppMetadata } from '#/types/generated.ts';
import { AppIcon } from '#/components/settings/app-icon.tsx';
import { ShortcutKeys } from '#/components/settings/shortcut-keys.tsx';
import { Badge } from '#/components/ui/badge.tsx';
import { Button } from '#/components/ui/button.tsx';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '#/components/ui/card.tsx';
import { Input } from '#/components/ui/input.tsx';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { X } from 'lucide-react';
import { useState } from 'react';
import { taurpc } from '#/api/taurpc.ts';
import { useDesktopAppManagerState } from '#/utils/state.ts';
import { createFileRoute } from '@tanstack/react-router';

export const Route = createFileRoute('/settings/global-shortcuts/')({
  component: SettingsGlobalShortcuts,
});

function SettingsGlobalShortcuts() {
  const [input, setInput] = useState('');
  const queryClient = useQueryClient();
  const removeGlobalShortcutMappingMutation = useMutation({
    mutationFn: taurpc.removeGlobalShortcutMapping,
    onSuccess: async () => {
      await queryClient.invalidateQueries({
        queryKey: ['global-shortcuts'],
      });
    },
  });
  const { data, isSuccess, isLoading } = useQuery({
    queryKey: ['global-shortcuts'],
    queryFn: taurpc.listGlobalShortcutMappings,
  });
  const mappings = data ?? [];
  const { appsMetadata } = useDesktopAppManagerState();
  const normalizedFilter = input.trim().toLowerCase();
  const filteredMappings = mappings.filter((mapping) => {
    if (!normalizedFilter) {
      return true;
    }

    return [
      mapping.shortcut,
      mapping.bundleId,
      mapping.hotkeyId,
      appsMetadata[mapping.bundleId]?.displayName ?? '',
    ].some(value => value.toLowerCase().includes(normalizedFilter));
  });

  return (
    <Card size="sm">
      <CardHeader className="flex items-center justify-between gap-3">
        <CardTitle>Global Shortcuts</CardTitle>
        <CardDescription>
          Current shortcut assignments for apps and hotkeys loaded by Chord.
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
              {data.length}
              {' '}
              mappings
            </Badge>
          )}
        </div>

        {isLoading
          ? (
              <p className="text-sm text-muted-foreground">Loading global shortcut mappings...</p>
            )
          : mappings.length === 0
            ? (
                <p className="text-sm text-muted-foreground">
                  No global shortcut mappings are currently registered.
                </p>
              )
            : filteredMappings.length === 0
              ? (
                  <p className="text-sm text-muted-foreground">No global shortcut mappings match that filter.</p>
                )
              : (
                  <div className="space-y-2">
                    {filteredMappings.map((mapping) => {
                      const appMetadata = appsMetadata[mapping.bundleId];
                      const appLabel = appMetadata?.displayName?.trim() || mapping.bundleId;

                      return (
                        <GlobalShortcutRow
                          key={mapping.shortcut}
                          appLabel={appLabel}
                          appMetadata={appMetadata}
                          mapping={mapping}
                          isRemoving={removeGlobalShortcutMappingMutation.isPending}
                          onRemove={(shortcut) => {
                            removeGlobalShortcutMappingMutation.mutate(shortcut);
                          }}
                        />
                      );
                    })}
                  </div>
                )}
      </CardContent>
    </Card>
  );
}

function GlobalShortcutRow({
  appLabel,
  appMetadata,
  mapping,
  isRemoving,
  onRemove,
}: {
  appLabel: string
  appMetadata?: DesktopAppMetadata
  mapping: Awaited<ReturnType<typeof taurpc.listGlobalShortcutMappings>>[number]
  isRemoving: boolean
  onRemove: (shortcut: string) => void
}) {
  const queryClient = useQueryClient();
  const [draftShortcut, setDraftShortcut] = useState(mapping.shortcut);
  const [isRecording, setIsRecording] = useState(false);
  const isDirty = draftShortcut.trim() !== mapping.shortcut;
  const updateGlobalShortcutMappingMutation = useMutation({
    mutationFn: ({ oldShortcut, newShortcut }: { oldShortcut: string, newShortcut: string }) =>
      taurpc.updateGlobalShortcutMapping(oldShortcut, newShortcut),
    onSuccess: async () => {
      await queryClient.invalidateQueries({
        queryKey: ['global-shortcuts'],
      });
    },
  });

  return (
    <div className="flex items-center gap-3 rounded-lg border bg-background/80 px-3 py-2">
      <div className="flex min-w-0 flex-1 items-center gap-2 overflow-hidden">
        <AppIcon appMetadata={appMetadata} label={appLabel} tooltip={mapping.bundleId} />
        <p className="truncate text-sm">
          <span className="font-medium text-foreground">{appLabel}</span>
          <span className="mx-2 text-muted-foreground">&gt;</span>
          <span className="truncate text-muted-foreground">{mapping.hotkeyId}</span>
        </p>
      </div>

      <div className="flex shrink-0 items-center gap-2">
        <button
          type="button"
          onKeyDown={(event) => {
            event.preventDefault();

            if (event.key === 'Escape') {
              setIsRecording(false);
              setDraftShortcut(mapping.shortcut);
              return;
            }

            if (!isRecording) {
              if (event.key === 'Enter' || event.key === ' ') {
                setIsRecording(true);
              }
              return;
            }

            const nextShortcut = serializeShortcutEvent(event);
            if (nextShortcut) {
              setDraftShortcut(nextShortcut);
              setIsRecording(false);
            }
          }}
          onClick={(event) => {
            event.currentTarget.focus();
            setIsRecording(true);
          }}
          className="inline-flex h-8 min-w-36 items-center rounded-lg border border-border bg-background px-2.5 text-left text-sm shadow-xs outline-none transition-colors focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50"
          aria-label={`Shortcut for ${appLabel}`}
        >
          {isRecording
            ? (
                <span className="text-xs text-muted-foreground">Press shortcut</span>
              )
            : (
                <ShortcutKeys shortcut={draftShortcut} />
              )}
        </button>
        {isDirty
          ? (
              <>
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={() => {
                    setIsRecording(false);
                    setDraftShortcut(mapping.shortcut);
                  }}
                  disabled={updateGlobalShortcutMappingMutation.isPending}
                >
                  Cancel
                </Button>
                <Button
                  type="button"
                  size="sm"
                  onClick={() => {
                    updateGlobalShortcutMappingMutation.mutate({
                      oldShortcut: mapping.shortcut,
                      newShortcut: draftShortcut.trim(),
                    });
                  }}
                  disabled={updateGlobalShortcutMappingMutation.isPending}
                >
                  {updateGlobalShortcutMappingMutation.isPending ? 'Saving...' : 'Save'}
                </Button>
              </>
            )
          : null}

        <Button
          type="button"
          variant="ghost"
          size="icon-sm"
          aria-label={`Remove ${mapping.shortcut}`}
          title="Remove mapping"
          onClick={() => {
            onRemove(mapping.shortcut);
          }}
          disabled={isRemoving || updateGlobalShortcutMappingMutation.isPending}
          className="text-muted-foreground hover:text-destructive"
        >
          <X />
        </Button>
      </div>
    </div>
  );
}

function serializeShortcutEvent(event: React.KeyboardEvent<HTMLButtonElement>) {
  const key = normalizeShortcutKey(event.key);
  if (!key) {
    return null;
  }

  const modifiers = [
    event.metaKey ? 'cmd' : null,
    event.ctrlKey ? 'ctrl' : null,
    event.altKey ? 'option' : null,
    event.shiftKey ? 'shift' : null,
  ].filter(Boolean);

  const shortcutParts = isModifierKey(key) ? modifiers : [...modifiers, key];
  if (shortcutParts.length === 0) {
    return null;
  }

  return shortcutParts.join('+');
}

function normalizeShortcutKey(key: string) {
  switch (key) {
    case 'Meta':
      return 'cmd';
    case 'Control':
      return 'ctrl';
    case 'Alt':
      return 'option';
    case 'Shift':
      return 'shift';
    case ' ':
    case 'Spacebar':
      return 'space';
    case 'CapsLock':
      return 'capslock';
    case 'ArrowUp':
      return 'up';
    case 'ArrowDown':
      return 'down';
    case 'ArrowLeft':
      return 'left';
    case 'ArrowRight':
      return 'right';
    default:
      return key.length === 1 ? key.toLowerCase() : key.toLowerCase();
  }
}

function isModifierKey(key: string) {
  return key === 'cmd' || key === 'ctrl' || key === 'option' || key === 'shift';
}
