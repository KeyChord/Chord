import type {
  ChordPackage,
  ChordsFilePlaceholder,
  DesktopAppMetadata,
} from "@chord/dev.improve.chord.lib.typeshare";
import { useMutation } from "@chord/com.npmjs.tanstack__react-query";
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
import {
  StateQueries,
  useChordPackageManagerState,
  useDesktopAppManagerState,
} from "@chord/dev.improve.chord.lib.state";
import { AppIcon } from "@chord/dev.improve.chord.routes.settings._components.app-icon";
import { useState } from "react";

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

/** Lets the user choose the keys a package's `<name>` placeholders stand for. */
export function PlaceholderChordsCard() {
  return (
    <StateQueries queries={[useChordPackageManagerState(), useDesktopAppManagerState()]}>
      {(chordPackageManagerData, desktopAppManagerData) => (
        <PlaceholderChordsCardContent
          entries={listPlaceholders(chordPackageManagerData.packages)}
          apps={desktopAppManagerData.appsMetadata}
        />
      )}
    </StateQueries>
  );
}

export function PlaceholderChordsCardContent({
  entries,
  apps,
}: {
  entries: PlaceholderEntry[];
  apps: Record<string, DesktopAppMetadata>;
}) {
  if (entries.length === 0) return null;
  return (
    <Card size="sm">
      <CardHeader>
        <CardTitle>Placeholders</CardTitle>
        <CardDescription>
          Choose the keys that packages use for placeholders in their chords, such as a prefix
          shared by every chord. Saving reloads the package&apos;s chords.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-2 pt-0">
        {entries.map((entry) => (
          <PlaceholderRow
            // Remount when the saved value changes, so the draft starts from it.
            key={`${entry.packageName}:${entry.pathslug}:${entry.placeholder.name}:${entry.placeholder.assigned ?? ""}`}
            entry={entry}
            app={entry.appId ? apps[entry.appId] : undefined}
          />
        ))}
      </CardContent>
    </Card>
  );
}

function PlaceholderRow({ entry, app }: { entry: PlaceholderEntry; app?: DesktopAppMetadata }) {
  const { packageName, pathslug, placeholder } = entry;
  const current = placeholder.assigned ?? placeholder.default ?? "";
  const [draft, setDraft] = useState(current);
  const save = useMutation({
    mutationFn: (sequence: string) =>
      taurpc.setPlaceholderChordBinding(packageName, pathslug, placeholder.name, sequence),
  });
  const reset = useMutation({
    mutationFn: () => taurpc.removePlaceholderChordBinding(packageName, pathslug, placeholder.name),
  });
  const pending = save.isPending || reset.isPending;
  const trimmed = draft.trim();
  const dirty = trimmed !== current;
  const appLabel = entry.appId ? app?.displayName?.trim() || entry.appId : "Every app";
  const label = placeholder.label?.trim() || `<${placeholder.name}>`;
  const inputId = `placeholder-${packageName}-${pathslug}-${placeholder.name}`;

  return (
    <div className="rounded-lg border bg-background/80 px-3 py-3">
      <div className="flex flex-col gap-3 lg:flex-row lg:items-center">
        <div className="min-w-0 flex-1 space-y-2">
          <div className="flex min-w-0 items-center gap-2">
            <AppIcon appMetadata={app} label={appLabel} tooltip={entry.appId} />
            <label htmlFor={inputId} className="min-w-0 truncate text-sm">
              <span className="font-medium text-foreground">{label}</span>
              <span className="mx-2 text-muted-foreground">·</span>
              <span className="text-muted-foreground">{packageName}</span>
            </label>
          </div>
          <div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
            <Badge variant="outline" className="font-mono">{`<${placeholder.name}>`}</Badge>
            <span>
              {placeholder.default === undefined ? (
                "No default: its chords are off until you choose keys"
              ) : (
                <>
                  Default{" "}
                  <code className="font-mono text-foreground/80">{placeholder.default}</code>
                </>
              )}
            </span>
            {pathslug !== "chords/macos.toml" && <span className="truncate">{pathslug}</span>}
          </div>
        </div>

        <form
          className="flex flex-col gap-2 sm:flex-row sm:items-center"
          onSubmit={(event) => {
            event.preventDefault();
            if (dirty && trimmed) save.mutate(trimmed);
          }}
        >
          <Input
            id={inputId}
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
            placeholder={placeholder.default ?? "keys"}
            autoComplete="off"
            spellCheck={false}
            maxLength={16}
            className="w-32 font-mono"
            disabled={pending}
          />
          {dirty ? (
            <>
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={() => setDraft(current)}
                disabled={pending}
              >
                Cancel
              </Button>
              <Button type="submit" size="sm" disabled={pending || !trimmed}>
                {save.isPending ? "Saving..." : "Save"}
              </Button>
            </>
          ) : placeholder.assigned !== undefined ? (
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={() => reset.mutate()}
              disabled={pending}
            >
              {reset.isPending
                ? "Resetting..."
                : placeholder.default === undefined
                  ? "Turn off"
                  : "Use default"}
            </Button>
          ) : null}
        </form>
      </div>
    </div>
  );
}
