import type {
  ChordPackage,
  ChordsFilePlaceholder,
  DesktopAppMetadata,
} from "@chord/dev.improve.chord.lib.typeshare";
import { useMutation } from "@chord/com.npmjs.tanstack__react-query";
import { taurpc } from "@chord/dev.improve.chord.api.taurpc";
import { AppIcon } from "@chord/dev.improve.chord.routes.settings._components.app-icon";
import { useId, useState } from "react";

export interface PlaceholderEntry {
  packageName: string;
  /** The chords file that declares the placeholder, such as `chords/macos.toml`. */
  pathslug: string;
  /** The app whose chords file this is, or undefined for a package's global file. */
  appId?: string;
  placeholder: ChordsFilePlaceholder;
}

/** Every placeholder that a loaded chords file declares, in package rank order. */
export function listPlaceholders(packages: ChordPackage[]): PlaceholderEntry[] {
  return packages.flatMap((pkg) =>
    Object.values(pkg.compiledChordsFiles)
      .toSorted((a, b) => a.pathslug.localeCompare(b.pathslug))
      .flatMap((file) => {
        const appPath = /^chords\/(?!@)(.+)\/macos\.toml$/.exec(file.pathslug)?.[1];
        return file.placeholders.map((placeholder) => ({
          packageName: pkg.name,
          pathslug: file.pathslug,
          appId: appPath?.replaceAll("/", "."),
          placeholder,
        }));
      }),
  );
}

/** A placeholder with no default and no chosen keys leaves its chords off. */
export function placeholderNeedsKeys({ placeholder }: PlaceholderEntry): boolean {
  return placeholder.assigned === undefined && placeholder.default === undefined;
}

/** Lets the user choose the keys a package's placeholders stand for. */
export function PackagePlaceholders({
  entries,
  apps,
  inline = false,
}: {
  entries: PlaceholderEntry[];
  apps: Record<string, DesktopAppMetadata>;
  /** Render only the rows, for a package's disclosed contents. */
  inline?: boolean;
}) {
  const headingId = useId();
  if (entries.length === 0) return null;
  const rows = entries.map((entry) => (
    <PlaceholderRow
      // Remount when the saved value or its inherited default changes.
      key={`${entry.pathslug}:${entry.placeholder.name}:${entry.placeholder.assigned ?? entry.placeholder.default ?? ""}`}
      entry={entry}
      app={entry.appId ? apps[entry.appId] : undefined}
    />
  ));
  if (inline) {
    return (
      <div role="group" aria-label="Placeholders">
        {rows}
      </div>
    );
  }
  return (
    <section aria-labelledby={headingId} className="mac-section">
      <div className="mac-section-header">
        <h3 id={headingId} className="mac-section-title">
          Placeholders
        </h3>
      </div>
      <div className="mac-list">{rows}</div>
      <p className="mac-section-footer">
        Choose the keys this package&apos;s chords use for each placeholder, such as a prefix shared
        by every chord. Press Return to save; the package&apos;s chords reload.
      </p>
    </section>
  );
}

function PlaceholderRow({ entry, app }: { entry: PlaceholderEntry; app?: DesktopAppMetadata }) {
  const { packageName, pathslug, placeholder } = entry;
  const current = placeholder.assigned ?? placeholder.default ?? "";
  const [draft, setDraft] = useState(current);
  const inputId = useId();
  const save = useMutation({
    mutationFn: (sequence: string) =>
      taurpc.setPlaceholderChordBinding(packageName, pathslug, placeholder.name, sequence),
  });
  const reset = useMutation({
    mutationFn: () => taurpc.removePlaceholderChordBinding(packageName, pathslug, placeholder.name),
  });
  const pending = save.isPending || reset.isPending;
  const error = save.error ?? reset.error;
  const trimmed = draft.trim();
  const dirty = trimmed !== current;
  const appLabel = entry.appId ? app?.displayName?.trim() || entry.appId : undefined;
  const label = placeholder.label?.trim() || placeholder.name;

  return (
    <form
      className="mac-row"
      onSubmit={(event) => {
        event.preventDefault();
        if (dirty && trimmed) save.mutate(trimmed);
      }}
    >
      {appLabel && <AppIcon appMetadata={app} label={appLabel} tooltip={entry.appId} />}
      <div className="mac-row-text">
        <label htmlFor={inputId} className="mac-row-title">
          <span className="text-foreground">{label}</span>
          {appLabel && <span className="text-muted-foreground"> · {appLabel}</span>}
        </label>
        <span className="mac-row-subtitle">
          <code className="font-mono">{placeholder.name}</code>
          {" · "}
          {placeholder.default === undefined ? (
            "No default: its chords are off until you choose keys"
          ) : (
            <>
              Default <code className="font-mono">{placeholder.default}</code>
            </>
          )}
          {pathslug !== "chords/macos.toml" && <> · {pathslug}</>}
        </span>
        {error && (
          <span role="alert" className="mac-row-subtitle text-destructive">
            {errorMessage(error)}
          </span>
        )}
      </div>
      {dirty ? (
        <>
          <button
            type="button"
            className="mac-button"
            onClick={() => setDraft(current)}
            disabled={pending}
          >
            Cancel
          </button>
          <button type="submit" className="mac-button" data-default disabled={pending || !trimmed}>
            {save.isPending ? "Saving…" : "Save"}
          </button>
        </>
      ) : placeholder.assigned !== undefined ? (
        <button
          type="button"
          className="mac-button"
          data-borderless
          onClick={() => reset.mutate()}
          disabled={pending}
        >
          {reset.isPending
            ? "Resetting…"
            : placeholder.default === undefined
              ? "Turn off"
              : "Use default"}
        </button>
      ) : null}
      <input
        id={inputId}
        className="mac-field w-20 text-center font-mono"
        value={draft}
        onChange={(event) => setDraft(event.target.value)}
        placeholder={placeholder.default ?? "keys"}
        autoComplete="off"
        spellCheck={false}
        maxLength={16}
        disabled={pending}
      />
    </form>
  );
}

function errorMessage(error: unknown) {
  return error && typeof error === "object" && "Message" in error
    ? String(error.Message)
    : String(error);
}
