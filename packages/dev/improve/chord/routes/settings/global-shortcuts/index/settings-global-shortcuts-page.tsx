import { useMutation, useQuery, useQueryClient } from "@chord/com.npmjs.tanstack__react-query";
import { taurpc } from "@chord/dev.improve.chord.api.taurpc";
import { GroupCard } from "@chord/dev.improve.chord.components.ui.group-card";
import { StateQueries, useDesktopAppManagerState } from "@chord/dev.improve.chord.lib.state";
import { ShortcutKeys } from "@chord/dev.improve.chord.routes.settings._components.shortcut-keys";
import { AppWindow, Command, Search, X } from "lucide-react";
import { useState } from "react";

export function SettingsGlobalShortcutsPage() {
  return (
    <StateQueries queries={[useDesktopAppManagerState()]}>
      {(desktopAppManagerData) => (
        <SettingsGlobalShortcutsPageContent desktopAppManagerData={desktopAppManagerData} />
      )}
    </StateQueries>
  );
}

type Mapping = Awaited<ReturnType<typeof taurpc.listGlobalShortcutMappings>>[number];

/**
 * Packages reserve shortcuts from Chord's pool so a chord can trigger an action inside an app:
 * the package assigns the shortcut in the app, and Chord presses it. One card per app, like
 * the Chords page, lists what each app has been given.
 */
function SettingsGlobalShortcutsPageContent({
  desktopAppManagerData,
}: {
  desktopAppManagerData: NonNullable<ReturnType<typeof useDesktopAppManagerState>["data"]>;
}) {
  const [input, setInput] = useState("");
  const queryClient = useQueryClient();
  const removeGlobalShortcutMappingMutation = useMutation({
    mutationFn: taurpc.removeGlobalShortcutMapping,
    onSuccess: async () => {
      await queryClient.invalidateQueries({
        queryKey: ["global-shortcuts"],
      });
    },
  });
  const { data, isLoading } = useQuery({
    queryKey: ["global-shortcuts"],
    queryFn: taurpc.listGlobalShortcutMappings,
  });
  const mappings = data ?? [];
  const { appsMetadata } = desktopAppManagerData;
  const appLabel = (bundleId: string) => appsMetadata[bundleId]?.displayName?.trim() || bundleId;
  const normalizedFilter = input.trim().toLowerCase();
  const filteredMappings = mappings.filter((mapping) => {
    if (!normalizedFilter) {
      return true;
    }

    return [mapping.shortcut, mapping.bundleId, mapping.hotkeyId, appLabel(mapping.bundleId)].some(
      (value) => value.toLowerCase().includes(normalizedFilter),
    );
  });
  const byApp = new Map<string, Mapping[]>();
  for (const mapping of filteredMappings) {
    byApp.set(mapping.bundleId, [...(byApp.get(mapping.bundleId) ?? []), mapping]);
  }
  const apps = [...byApp].toSorted(([a], [b]) => appLabel(a).localeCompare(appLabel(b)));

  return (
    <div className="mac-page">
      {mappings.length > 0 && (
        <div className="mac-toolbar">
          <label className="mac-search ml-auto">
            <Search size={13} aria-hidden="true" />
            <input
              aria-label="Search global shortcuts"
              placeholder="Search"
              value={input}
              onChange={(event) => setInput(event.target.value)}
            />
            {input && (
              <button type="button" aria-label="Clear search" onClick={() => setInput("")}>
                <X size={13} />
              </button>
            )}
          </label>
        </div>
      )}

      {isLoading ? (
        <GroupCard title="Loading shortcuts…" />
      ) : mappings.length === 0 ? (
        <GroupCard
          icon={<Command size={17} strokeWidth={1.6} />}
          title="No shortcuts reserved yet"
          subtitle="When a package needs a shortcut to work an app, it appears here."
        />
      ) : apps.length === 0 ? (
        <GroupCard title="No shortcuts match your search" />
      ) : (
        <div className="mac-card-list" role="list" aria-label="Global shortcuts by app">
          {apps.map(([bundleId, appMappings]) => {
            const metadata = appsMetadata[bundleId];
            return (
              <div key={bundleId} role="listitem" className="mac-section">
                <GroupCard
                  icon={
                    metadata?.iconDataUrl ? (
                      <img src={metadata.iconDataUrl} alt="" />
                    ) : (
                      <AppWindow size={17} strokeWidth={1.6} />
                    )
                  }
                  title={appLabel(bundleId)}
                  subtitle={<span className="font-mono">{bundleId}</span>}
                  count={appMappings.length}
                  listLabel={`Shortcuts for ${appLabel(bundleId)}`}
                >
                  {appMappings.map((mapping) => (
                    <GlobalShortcutRow
                      key={mapping.shortcut}
                      appLabel={appLabel(bundleId)}
                      mapping={mapping}
                      isRemoving={removeGlobalShortcutMappingMutation.isPending}
                      onRemove={(shortcut) => {
                        removeGlobalShortcutMappingMutation.mutate(shortcut);
                      }}
                    />
                  ))}
                </GroupCard>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

/** Splits an ID such as `toggleMiniPlayer` or `toggle-mini-player` into words. */
function hotkeyTitle(hotkeyId: string) {
  const words = hotkeyId
    .replace(/([a-z])([A-Z])/g, "$1 $2")
    .replace(/[-_.]+/g, " ")
    .trim();
  return words.charAt(0).toUpperCase() + words.slice(1);
}

function GlobalShortcutRow({
  appLabel,
  mapping,
  isRemoving,
  onRemove,
}: {
  appLabel: string;
  mapping: Mapping;
  isRemoving: boolean;
  onRemove: (shortcut: string) => void;
}) {
  const queryClient = useQueryClient();
  const [draftShortcut, setDraftShortcut] = useState(mapping.shortcut);
  const [isRecording, setIsRecording] = useState(false);
  const isDirty = draftShortcut.trim() !== mapping.shortcut;
  const updateGlobalShortcutMappingMutation = useMutation({
    mutationFn: ({ oldShortcut, newShortcut }: { oldShortcut: string; newShortcut: string }) =>
      taurpc.updateGlobalShortcutMapping(oldShortcut, newShortcut),
    onSuccess: async () => {
      await queryClient.invalidateQueries({
        queryKey: ["global-shortcuts"],
      });
    },
  });

  return (
    <div className="mac-row" role="listitem">
      <span className="mac-row-text">
        <span className="mac-row-title">{hotkeyTitle(mapping.hotkeyId)}</span>
        <span className="mac-row-subtitle font-mono">{mapping.hotkeyId}</span>
      </span>
      {isDirty && (
        <>
          <button
            type="button"
            className="mac-button"
            onClick={() => {
              setIsRecording(false);
              setDraftShortcut(mapping.shortcut);
            }}
            disabled={updateGlobalShortcutMappingMutation.isPending}
          >
            Cancel
          </button>
          <button
            type="button"
            className="mac-button"
            data-default
            onClick={() => {
              updateGlobalShortcutMappingMutation.mutate({
                oldShortcut: mapping.shortcut,
                newShortcut: draftShortcut.trim(),
              });
            }}
            disabled={updateGlobalShortcutMappingMutation.isPending}
          >
            {updateGlobalShortcutMappingMutation.isPending ? "Saving…" : "Save"}
          </button>
        </>
      )}
      {/* Click, then press the new shortcut; Escape puts the old one back. */}
      <button
        type="button"
        onKeyDown={(event) => {
          event.preventDefault();

          if (event.key === "Escape") {
            setIsRecording(false);
            setDraftShortcut(mapping.shortcut);
            return;
          }

          if (!isRecording) {
            if (event.key === "Enter" || event.key === " ") {
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
        onBlur={() => setIsRecording(false)}
        className="mac-shortcut-field"
        data-recording={isRecording || undefined}
        aria-label={`Shortcut for ${hotkeyTitle(mapping.hotkeyId)} in ${appLabel}`}
        title="Click, then press a new shortcut"
      >
        {isRecording ? "Press a shortcut" : <ShortcutKeys shortcut={draftShortcut} />}
      </button>
      <button
        type="button"
        className="mac-button"
        data-icon
        data-borderless
        aria-label={`Remove ${mapping.shortcut}`}
        title="Remove this shortcut"
        onClick={() => {
          onRemove(mapping.shortcut);
        }}
        disabled={isRemoving || updateGlobalShortcutMappingMutation.isPending}
      >
        <X size={14} aria-hidden="true" />
      </button>
    </div>
  );
}

function serializeShortcutEvent(event: React.KeyboardEvent<HTMLButtonElement>) {
  const key = normalizeShortcutKey(event.key);
  if (!key) {
    return null;
  }

  const modifiers = [
    event.metaKey ? "cmd" : null,
    event.ctrlKey ? "ctrl" : null,
    event.altKey ? "option" : null,
    event.shiftKey ? "shift" : null,
  ].filter(Boolean);

  const shortcutParts = isModifierKey(key) ? modifiers : [...modifiers, key];
  if (shortcutParts.length === 0) {
    return null;
  }

  return shortcutParts.join("+");
}

function normalizeShortcutKey(key: string) {
  switch (key) {
    case "Meta":
      return "cmd";
    case "Control":
      return "ctrl";
    case "Alt":
      return "option";
    case "Shift":
      return "shift";
    case " ":
    case "Spacebar":
      return "space";
    case "CapsLock":
      return "capslock";
    case "ArrowUp":
      return "up";
    case "ArrowDown":
      return "down";
    case "ArrowLeft":
      return "left";
    case "ArrowRight":
      return "right";
    default:
      return key.length === 1 ? key.toLowerCase() : key.toLowerCase();
  }
}

function isModifierKey(key: string) {
  return key === "cmd" || key === "ctrl" || key === "option" || key === "shift";
}
