import { Input } from "@chord/dev.improve.chord.components.ui.input";
import type {
  Chord,
  ChordPackage,
  DesktopAppMetadata,
} from "@chord/dev.improve.chord.lib.typeshare";
import type { ColumnDef, Row } from "@chord/com.npmjs.tanstack__react-table";
import {
  columnFilteringFeature,
  createExpandedRowModel,
  createFilteredRowModel,
  globalFilteringFeature,
  rowExpandingFeature,
  Subscribe,
  tableFeatures,
  useTable,
} from "@chord/com.npmjs.tanstack__react-table";
import { Button } from "@chord/dev.improve.chord.components.ui.button";
import { usePersonalChordsEditor } from "@chord/dev.improve.chord.routes.settings._components.personal-chords";
import {
  Braces,
  ChevronDown,
  ChevronUp,
  Command,
  Keyboard,
  Pencil,
  Package,
  Search,
  Terminal,
  X,
} from "lucide-react";
import { Fragment, useId, useMemo } from "react";

export interface LibraryChord {
  id: string;
  chord: Chord;
  packageName: string;
  pathslug: string;
  personal: boolean;
  appId?: string;
  appLabel: string;
}

/** Keep registry priority and distinguish identical triggers in different files and packages. */
export function listLibraryChords(
  packages: ChordPackage[],
  personalNames: Set<string>,
  apps: Record<string, DesktopAppMetadata>,
): LibraryChord[] {
  return packages
    .flatMap((pkg) =>
      Object.values(pkg.compiledChordsFiles).flatMap((file) => {
        const appPath = /^chords\/(?!@)(.+)\/macos\.toml$/.exec(file.pathslug)?.[1];
        const fileAppId = appPath?.replaceAll("/", ".");
        return file.chords.map((chord) => {
          const global = pkg.globalChords.some(
            (ref) =>
              ref.chordsFilePathslug === file.pathslug &&
              ref.chord.index === chord.index &&
              ref.chord.rawTrigger === chord.rawTrigger,
          );
          const appId = global ? undefined : fileAppId;
          return {
            id: JSON.stringify([pkg.name, file.pathslug, chord.index, chord.rawTrigger]),
            chord,
            packageName: pkg.name,
            pathslug: file.pathslug,
            personal: personalNames.has(pkg.name),
            appId,
            appLabel: appId
              ? apps[appId]?.displayName?.trim() || file.name.trim() || appId
              : "Every app",
          };
        });
      }),
    )
    .toSorted((a, b) => Number(b.personal) - Number(a.personal));
}

/**
 * Where a package ranks. Chord's core package always comes first and personal packages follow
 * it; the user orders installed packages, which always rank above built-in ones.
 */
export type PackageRank = "core" | "personal" | "installed" | "builtin";

export const CORE_PACKAGE = "@keychord/chords-chord";

export interface LibraryPackage {
  name: string;
  personal: boolean;
  rank: PackageRank;
  chords: LibraryChord[];
  fileCount: number;
  appLabels: string[];
  appId?: string;
}

/** Keeps the native rank order of `packages`, highest first. */
export function listLibraryPackages(
  packages: ChordPackage[],
  personalNames: Set<string>,
  apps: Record<string, DesktopAppMetadata>,
  builtinNames: Set<string> = new Set(),
): LibraryPackage[] {
  return packages
    .map((pkg): LibraryPackage => {
      const chords = listLibraryChords([pkg], personalNames, apps);
      const appIds = new Set(chords.map((row) => row.appId));
      const personal = personalNames.has(pkg.name);
      return {
        name: pkg.name,
        personal,
        rank: (pkg.name === CORE_PACKAGE
          ? "core"
          : personal
            ? "personal"
            : builtinNames.has(pkg.name)
              ? "builtin"
              : "installed"),
        chords,
        fileCount: new Set([
          ...Object.keys(pkg.rawChordsFiles),
          ...Object.keys(pkg.compiledChordsFiles),
        ]).size,
        appLabels: [...new Set(chords.map((row) => row.appLabel))],
        appId: appIds.size === 1 ? chords[0]?.appId : undefined,
      };
    })
    .toSorted((a, b) => rankOrder.indexOf(a.rank) - rankOrder.indexOf(b.rank));
}

const rankOrder: PackageRank[] = ["core", "personal", "installed", "builtin"];

const rankable = (rank: PackageRank) => rank === "installed" || rank === "builtin";

/**
 * The order to save after moving one package up (-1) or down (1) within its group, or undefined
 * when it is already at the edge. Installed packages stay above built-in ones.
 */
export function movePackage(
  packages: LibraryPackage[],
  name: string,
  offset: -1 | 1,
): string[] | undefined {
  const ranked = packages.filter((pkg) => rankable(pkg.rank));
  const index = ranked.findIndex((pkg) => pkg.name === name);
  const target = ranked[index + offset];
  if (index === -1 || !target || target.rank !== ranked[index]!.rank) return undefined;
  const names = ranked.map((pkg) => pkg.name);
  names[index] = target.name;
  names[index + offset] = name;
  return names;
}

const rankGroups: Partial<Record<PackageRank, { label: string; description: string }>> = {
  installed: {
    label: "Installed",
    description: "When two packages define the same chord, the higher one runs.",
  },
  builtin: {
    label: "Built-in",
    description: "Chord's default packages. Installed packages always override them.",
  },
};

type PackageOrder = {
  move: (name: string, offset: -1 | 1) => void;
  pending: boolean;
};

function RankControls({
  entry,
  packages,
  order,
}: {
  entry: LibraryPackage;
  packages: LibraryPackage[];
  order?: PackageOrder;
}) {
  if (!order || !rankable(entry.rank)) return null;
  return (
    <span className="inline-flex items-center gap-0.5">
      {([-1, 1] as const).map((offset) => {
        const Icon = offset === -1 ? ChevronUp : ChevronDown;
        return (
          <Button
            key={offset}
            variant="ghost"
            type="button"
            size="icon-xs"
            aria-label={`Move ${entry.name} ${offset === -1 ? "up" : "down"}`}
            disabled={order.pending || !movePackage(packages, entry.name, offset)}
            onClick={() => order.move(entry.name, offset)}
          >
            <Icon size={14} />
          </Button>
        );
      })}
    </span>
  );
}

type PersonalEditor = ReturnType<typeof usePersonalChordsEditor>;

const chordFeatures = tableFeatures({});
const packageFeatures = tableFeatures({
  columnFilteringFeature,
  globalFilteringFeature,
  filteredRowModel: createFilteredRowModel(),
  rowExpandingFeature,
  expandedRowModel: createExpandedRowModel(),
});

const actionKinds = {
  shortcut: { label: "Shortcut", icon: Command },
  shell: { label: "Shell command", icon: Terminal },
  emit: { label: "Script", icon: Braces },
  builtin: { label: "Chord action", icon: Keyboard },
} as const;

function ChordName({
  entry,
  dirname,
  editor,
}: {
  entry: LibraryChord;
  dirname?: string;
  editor: PersonalEditor;
}) {
  const file = dirname ? { dirname, pathslug: entry.pathslug } : undefined;
  return (
    <div className="flex items-center gap-3">
      <span className="min-w-0 flex-1 truncate text-[15px] tracking-[-0.015em]" title={entry.chord.name}>
        {entry.chord.name}
      </span>
      {entry.personal && file && (
        <Button
          type="button"
          variant="ghost"
          size="sm"
          className="chord-library-edit"
          aria-label={`Edit ${entry.chord.name} in ${dirname}/${entry.pathslug}`}
          aria-expanded={editor.rowId === entry.id}
          onClick={() => editor.open(entry.id, file)}
        >
          <Pencil className="size-3.5" />
          <span className="chord-library-edit-label">Edit</span>
          {editor.hasDraft(file) && (
            <span className="size-1.5 rounded-full bg-primary" aria-label="Unsaved changes" />
          )}
        </Button>
      )}
    </div>
  );
}

function ChordType({ chord }: { chord: Chord }) {
  const kind = chord.actions[0] ? actionKinds[chord.actions[0].type] : undefined;
  const Icon = kind?.icon ?? Keyboard;
  return (
    <span className="chord-library-type" title={kind?.label ?? "No actions"}>
      <Icon size={17} strokeWidth={1.6} />
      <span className="sr-only">{kind?.label ?? "No actions"}</span>
    </span>
  );
}

function PackageChords({
  rows,
  personalDirnames,
  editor,
}: {
  rows: LibraryChord[];
  personalDirnames: Record<string, string>;
  editor: PersonalEditor;
}) {
  const groups = useMemo(() => {
    const byApp = new Map<string | undefined, LibraryChord[]>();
    for (const entry of rows) {
      const group = byApp.get(entry.appId);
      if (group) group.push(entry);
      else byApp.set(entry.appId, [entry]);
    }
    return [...byApp];
  }, [rows]);

  return (
    <section aria-label="Package chords" className="space-y-6">
      {groups.map(([appId, chords]) => (
        <AppChords
          key={appId ?? "global"}
          rows={chords}
          personalDirnames={personalDirnames}
          editor={editor}
        />
      ))}
      {rows.length === 0 && (
        <p className="py-6 text-center text-sm text-muted-foreground">
          No chords loaded in this package.
        </p>
      )}
    </section>
  );
}

function AppChords({
  rows,
  personalDirnames,
  editor,
}: {
  rows: LibraryChord[];
  personalDirnames: Record<string, string>;
  editor: PersonalEditor;
}) {
  const headingId = useId();
  const appLabel = rows[0]!.appLabel;
  const columns = useMemo<ColumnDef<typeof chordFeatures, LibraryChord>[]>(
    () => [
      {
        id: "sequence",
        accessorFn: (entry) => entry.chord.rawTrigger,
        header: "Sequence",
        cell: ({ getValue }) => (
          <code className="chord-library-sequence" title={getValue<string>()}>
            {getValue<string>()}
          </code>
        ),
      },
      {
        id: "name",
        accessorFn: (entry) => entry.chord.name,
        header: "Action",
        cell: ({ row }) => (
          <ChordName
            entry={row.original}
            dirname={personalDirnames[row.original.packageName]}
            editor={editor}
          />
        ),
      },
      {
        id: "type",
        accessorFn: (entry) => entry.chord.actions[0]?.type,
        header: "Type",
        cell: ({ row }) => <ChordType chord={row.original.chord} />,
      },
    ],
    [personalDirnames, editor],
  );
  const table = useTable({
    features: chordFeatures,
    data: rows,
    columns,
    getRowId: (entry) => entry.id,
  });

  return (
    <section aria-labelledby={headingId}>
      <h3 id={headingId} className="px-3 pb-2 pt-1 text-sm font-medium" title={rows[0]!.appId}>
        {appLabel}
      </h3>
      <div className="chord-library-scroll">
        <table className="chord-library-table app-chords-table">
          <caption className="sr-only">
            {appLabel} sequences, actions, and action types
          </caption>
          <thead>
            {table.getHeaderGroups().map((group) => (
              <tr key={group.id}>
                {group.headers.map((header) => (
                  <th
                    key={header.id}
                    scope="col"
                    className={header.column.id === "type" ? "text-center" : undefined}
                  >
                    {header.isPlaceholder ? null : <table.FlexRender header={header} />}
                  </th>
                ))}
              </tr>
            ))}
          </thead>
          <tbody>
            {table.getRowModel().rows.map((row) => (
              <Fragment key={row.id}>
                <tr>
                  {row.getAllCells().map((cell) => (
                    <td
                      key={cell.id}
                      className={cell.column.id === "type" ? "text-center" : undefined}
                    >
                      <table.FlexRender cell={cell} />
                    </td>
                  ))}
                </tr>
                {editor.rowId === row.id && (
                  <tr>
                    <td
                      colSpan={row.getAllCells().length}
                      style={{ height: "auto", padding: "8px 0 20px" }}
                    >
                      {editor.editor}
                    </td>
                  </tr>
                )}
              </Fragment>
            ))}
          </tbody>
        </table>
      </div>
    </section>
  );
}

function PackageName({
  row,
  app,
  detailsId,
}: {
  row: Row<typeof packageFeatures, LibraryPackage>;
  app?: DesktopAppMetadata;
  detailsId: string;
}) {
  const entry = row.original;
  // The core row is stable. Subscribe here so the React Compiler sees expansion changes.
  return (
    <Subscribe
      source={row.table.atoms.expanded}
      selector={(expanded) => expanded === true || !!expanded[row.id]}
    >
      {(expanded) => (
        <Button
          variant="ghost"
          type="button"
          className="package-library-name h-auto justify-start p-0 font-normal"
          aria-label={`${expanded ? "Hide" : "Show"} chords in ${entry.name}`}
          aria-expanded={expanded}
          aria-controls={detailsId}
          onClick={row.getToggleExpandedHandler()}
        >
          <span
            className={`chord-library-icon ${entry.personal ? "chord-library-icon-personal" : ""}`}
            aria-hidden="true"
          >
            {app?.iconDataUrl ? (
              <img src={app.iconDataUrl} alt="" />
            ) : (
              <Package size={18} strokeWidth={1.6} />
            )}
          </span>
          <span className="min-w-0 flex-1">
            <span className="flex items-center gap-2">
              <span className="truncate text-[15px] tracking-[-0.015em]" title={entry.name}>
                {entry.name}
              </span>
              <ChevronDown
                className={`size-3.5 shrink-0 text-muted-foreground transition-transform ${expanded ? "rotate-180" : ""}`}
                aria-hidden="true"
              />
            </span>
            <span className="block text-[11px] text-muted-foreground">
              {entry.chords.length} chord{entry.chords.length === 1 ? "" : "s"}
              {" · "}
              {entry.fileCount} file{entry.fileCount === 1 ? "" : "s"}
            </span>
          </span>
        </Button>
      )}
    </Subscribe>
  );
}

export function ChordLibrary({
  packages,
  apps,
  personalDirnames,
  order,
}: {
  packages: LibraryPackage[];
  apps: Record<string, DesktopAppMetadata>;
  personalDirnames: Record<string, string>;
  /** Lets the user rank packages; rows stay in the given rank order. */
  order?: PackageOrder;
}) {
  // Keep drafts outside table rows so search and collapse preserve unsaved changes.
  const editor = usePersonalChordsEditor();
  const detailsPrefix = useId();
  const columns = useMemo<ColumnDef<typeof packageFeatures, LibraryPackage>[]>(
    () => [
      {
        accessorKey: "name",
        header: "Package name",
        cell: ({ row }) => (
          <PackageName
            row={row}
            app={row.original.appId ? apps[row.original.appId] : undefined}
            detailsId={`${detailsPrefix}-${encodeURIComponent(row.id)}`}
          />
        ),
      },
      {
        id: "rank",
        header: () => <span className="sr-only">Rank</span>,
        cell: ({ row }) => <RankControls entry={row.original} packages={packages} order={order} />,
      },
    ],
    [apps, detailsPrefix, packages, order],
  );
  const table = useTable({
    features: packageFeatures,
    data: packages,
    columns,
    getRowId: (entry) => entry.name,
    getRowCanExpand: () => true,
    autoResetExpanded: false,
    globalFilterFn: (row, _columnId, value: string) =>
      [row.original.name, ...row.original.appLabels].some((label) =>
        label.toLocaleLowerCase().includes(value.trim().toLocaleLowerCase()),
      ),
  });
  const visibleRows = table.getRowModel().rows;
  const search = String(table.state.globalFilter ?? "");

  return (
    <section aria-label="Chord packages">
      <div className="chord-library-toolbar">
        <label className="chord-library-search">
          <Search size={16} aria-hidden="true" />
          <Input
            aria-label="Search packages"
            placeholder="Search packages"
            value={search}
            onChange={(event) => table.setGlobalFilter(event.target.value)}
          />
          {search && (
            <Button
              variant="ghost"
              type="button"
              size="icon-xs"
              aria-label="Clear search"
              onClick={() => table.setGlobalFilter("")}
            >
              <X size={14} />
            </Button>
          )}
        </label>
        <span className="ml-auto whitespace-nowrap text-xs text-muted-foreground" role="status">
          {visibleRows.length} package{visibleRows.length === 1 ? "" : "s"}
        </span>
      </div>
      <div className="chord-library-scroll">
        <table className="chord-library-table package-library-table">
          <caption className="sr-only">
            Your chord packages, highest rank first; expand a package to view its chords grouped
            by app
          </caption>
          <thead>
            {table.getHeaderGroups().map((group) => (
              <tr key={group.id}>
                {group.headers.map((header) => (
                  <th key={header.id} scope="col">
                    {header.isPlaceholder ? null : <table.FlexRender header={header} />}
                  </th>
                ))}
              </tr>
            ))}
          </thead>
          <tbody>
            {visibleRows.map((row, index) => {
              const expanded = table.state.expanded === true || !!table.state.expanded[row.id];
              const rank = row.original.rank;
              const group = rankGroups[rank];
              const startsGroup = visibleRows[index - 1]?.original.rank !== rank;
              return (
                <Fragment key={row.id}>
                  {group && startsGroup && (
                    <tr className="package-library-group">
                      <th scope="colgroup" colSpan={row.getAllCells().length}>
                        <span className="block text-xs font-medium">{group.label}</span>
                        <span className="block text-[11px] font-normal text-muted-foreground">
                          {group.description}
                        </span>
                      </th>
                    </tr>
                  )}
                  <tr className="package-library-row" data-expanded={expanded}>
                    {row.getAllCells().map((cell) => (
                      <td
                        key={cell.id}
                        className={
                          cell.column.id === "name"
                            ? undefined
                            : "text-[13px] text-muted-foreground"
                        }
                      >
                        <table.FlexRender cell={cell} />
                      </td>
                    ))}
                  </tr>
                  <tr className="package-library-details" hidden={!expanded}>
                    <td colSpan={row.getAllCells().length}>
                      <div
                        id={`${detailsPrefix}-${encodeURIComponent(row.id)}`}
                        className="package-library-details-content"
                      >
                        {expanded && (
                          <PackageChords
                            editor={editor}
                            rows={row.original.chords}
                            personalDirnames={personalDirnames}
                          />
                        )}
                      </div>
                    </td>
                  </tr>
                </Fragment>
              );
            })}
          </tbody>
        </table>
      </div>
      {visibleRows.length === 0 && (
        <div className="py-16 text-center">
          <Package className="mx-auto mb-3 size-7 text-muted-foreground/50" strokeWidth={1.4} />
          <p className="text-sm">
            {packages.length === 0 ? "Your package library is empty" : "No packages found"}
          </p>
          <p className="mt-1 text-xs text-muted-foreground">
            {packages.length === 0
              ? "Find a package in Browse to get started."
              : "Try a different package name or app."}
          </p>
        </div>
      )}
    </section>
  );
}
