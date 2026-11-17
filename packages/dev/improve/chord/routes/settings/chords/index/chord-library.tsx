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
import {
  listPlaceholders,
  PackagePlaceholders,
  placeholderNeedsKeys,
  type PlaceholderEntry,
} from "@chord/dev.improve.chord.routes.settings._components.package-placeholders";
import { usePersonalChordsEditor } from "@chord/dev.improve.chord.routes.settings._components.personal-chords";
import {
  Braces,
  ChevronDown,
  Command,
  GripVertical,
  Keyboard,
  Pencil,
  Package,
  Search,
  Terminal,
  X,
} from "lucide-react";
import {
  Fragment,
  useEffect,
  useId,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent,
  type PointerEvent,
} from "react";
import { flushSync } from "react-dom";

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
  placeholders: PlaceholderEntry[];
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
        placeholders: listPlaceholders([pkg]),
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

export type DropPlace = "before" | "after";

/**
 * The order to save after moving package `name` before or after `target`, or undefined when the
 * two are in different groups or nothing would change. Installed packages stay above built-in
 * ones, and the order lists only the packages the user ranks.
 */
export function movePackage(
  packages: LibraryPackage[],
  name: string,
  target: string,
  place: DropPlace,
): string[] | undefined {
  const ranked = packages.filter((pkg) => rankable(pkg.rank));
  const rank = ranked.find((pkg) => pkg.name === name)?.rank;
  if (!rank || name === target || ranked.find((pkg) => pkg.name === target)?.rank !== rank) {
    return undefined;
  }
  const group = ranked.filter((pkg) => pkg.rank === rank).map((pkg) => pkg.name);
  const moved = group.filter((other) => other !== name);
  moved.splice(moved.indexOf(target) + (place === "after" ? 1 : 0), 0, name);
  if (moved.every((other, index) => other === group[index])) return undefined;
  const next = moved.values();
  return ranked.map((pkg) => (pkg.rank === rank ? next.next().value! : pkg.name));
}

/**
 * Shows `packages` in a saved `order` before Chord reports it back. Packages the order doesn't
 * name keep their place.
 */
export function applyPackageOrder(packages: LibraryPackage[], order: string[]): LibraryPackage[] {
  const position = new Map(order.map((name, index) => [name, index]));
  const listed = (pkg: LibraryPackage) => rankable(pkg.rank) && position.has(pkg.name);
  const sorted = packages
    .filter(listed)
    .toSorted((a, b) => position.get(a.name)! - position.get(b.name)!)
    .values();
  return packages.map((pkg) => (listed(pkg) ? sorted.next().value! : pkg));
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

interface Drop {
  target: string;
  place: DropPlace;
  /** The order to save if the package is dropped here. */
  order: string[];
}

interface DragState {
  name: string;
  rank: PackageRank;
  /** The pointer's latest viewport y. */
  y: number;
  scroller: HTMLElement;
  drop?: Drop;
  /** The pending auto-scroll frame, while the pointer rests near the scroller's edge. */
  frame?: number;
}

/** The nearest ancestor that scrolls vertically, where dragging near an edge scrolls. */
function scrollParent(node: HTMLElement): HTMLElement {
  for (let parent = node.parentElement; parent; parent = parent.parentElement) {
    const { overflowY } = getComputedStyle(parent);
    if (
      (overflowY === "auto" || overflowY === "scroll") &&
      parent.scrollHeight > parent.clientHeight
    ) {
      return parent;
    }
  }
  return (document.scrollingElement as HTMLElement | null) ?? document.documentElement;
}

const autoScrollEdge = 48;

/** Pixels to scroll this frame: faster the closer the pointer is to (or past) an edge. */
function autoScrollStep({ scroller, y }: DragState): number {
  const { top, bottom } =
    scroller === document.scrollingElement
      ? { top: 0, bottom: window.innerHeight }
      : scroller.getBoundingClientRect();
  const past =
    y < top + autoScrollEdge
      ? y - (top + autoScrollEdge)
      : y > bottom - autoScrollEdge
        ? y - (bottom - autoScrollEdge)
        : 0;
  return Math.max(-24, Math.min(24, Math.round(past / 3)));
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
              {entry.placeholders.length > 0 && (
                <>
                  {" · "}
                  {entry.placeholders.length} placeholder
                  {entry.placeholders.length === 1 ? "" : "s"}
                </>
              )}
              {entry.placeholders.some(placeholderNeedsKeys) && (
                <>
                  {" · "}
                  <span className="text-amber-800 dark:text-amber-300">needs keys</span>
                </>
              )}
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
  configurePackage,
  onReorder,
}: {
  packages: LibraryPackage[];
  apps: Record<string, DesktopAppMetadata>;
  personalDirnames: Record<string, string>;
  configurePackage?: string;
  /**
   * Lets the user drag installed and built-in packages within their group, and receives the
   * order to save. Rows stay in the given rank order.
   */
  onReorder?: (order: string[]) => void;
}) {
  // Keep drafts outside table rows so search and collapse preserve unsaved changes.
  const editor = usePersonalChordsEditor();
  const detailsPrefix = useId();
  const hintId = useId();
  const tableRef = useRef<HTMLTableElement>(null);
  const configureRowRef = useRef<HTMLTableRowElement>(null);
  // Pointer moves read and update this without waiting for a render; `dragging` mirrors it.
  const dragRef = useRef<DragState | null>(null);
  const [dragging, setDragging] = useState<{ name: string; drop?: Drop } | null>(null);
  const [announcement, setAnnouncement] = useState("");

  function save(name: string, { target, place, order }: Drop) {
    onReorder?.(order);
    setAnnouncement(`Moved ${name} ${place === "before" ? "above" : "below"} ${target}.`);
  }

  /** Where the dragged package would land: between the visible packages of its group. */
  function findDrop(drag: DragState): Drop | undefined {
    const rows = tableRef.current?.querySelectorAll<HTMLTableRowElement>(
      `tr.package-library-row[data-rank="${drag.rank}"]`,
    );
    // An expanded package's chords move with it, so its details row counts toward its middle.
    const blocks = [...(rows ?? [])].map((row) => {
      const details = row.nextElementSibling;
      const last = details instanceof HTMLElement && !details.hidden ? details : row;
      const { top } = row.getBoundingClientRect();
      const { bottom } = last.getBoundingClientRect();
      return { name: row.dataset.package!, middle: (top + bottom) / 2 };
    });
    const slot = blocks.filter((block) => block.middle < drag.y).length;
    const target = blocks[Math.min(slot, blocks.length - 1)];
    if (!target) return undefined;
    const place: DropPlace = slot < blocks.length ? "before" : "after";
    const order = movePackage(packages, drag.name, target.name, place);
    return order && { target: target.name, place, order };
  }

  function track() {
    const drag = dragRef.current;
    if (!drag) return;
    const drop = findDrop(drag);
    drag.drop = drop;
    setDragging((shown) =>
      shown?.drop?.target === drop?.target && shown?.drop?.place === drop?.place
        ? shown
        : { name: drag.name, drop },
    );
  }

  function autoScroll() {
    const drag = dragRef.current;
    if (!drag) return;
    const step = autoScrollStep(drag);
    const before = drag.scroller.scrollTop;
    drag.scroller.scrollTop += step;
    // Stop at rest or at either end; the next pointer move starts it again.
    drag.frame =
      step !== 0 && drag.scroller.scrollTop !== before
        ? requestAnimationFrame(autoScroll)
        : undefined;
  }

  function end(commit: boolean) {
    const drag = dragRef.current;
    if (!drag) return;
    dragRef.current = null;
    if (drag.frame !== undefined) cancelAnimationFrame(drag.frame);
    setDragging(null);
    if (commit && drag.drop) save(drag.name, drag.drop);
  }

  function startDrag(event: PointerEvent<HTMLButtonElement>, entry: LibraryPackage) {
    if (event.button !== 0 || dragRef.current) return;
    // Keep the press from selecting text or starting a native drag.
    event.preventDefault();
    event.currentTarget.setPointerCapture(event.pointerId);
    dragRef.current = {
      name: entry.name,
      rank: entry.rank,
      y: event.clientY,
      scroller: scrollParent(event.currentTarget),
    };
    setDragging({ name: entry.name });
  }

  function moveDrag(event: PointerEvent<HTMLButtonElement>) {
    const drag = dragRef.current;
    if (!drag) return;
    drag.y = event.clientY;
    track();
    if (drag.frame === undefined) autoScroll();
  }

  /** Arrow keys on a focused handle move its package one place within its group. */
  function stepDrag(event: KeyboardEvent<HTMLButtonElement>, entry: LibraryPackage) {
    if (event.key !== "ArrowUp" && event.key !== "ArrowDown") return;
    event.preventDefault();
    const up = event.key === "ArrowUp";
    const group = table
      .getRowModel()
      .rows.map((row) => row.original)
      .filter((pkg) => pkg.rank === entry.rank);
    const target = group[group.findIndex((pkg) => pkg.name === entry.name) + (up ? -1 : 1)];
    if (!target) return;
    const place: DropPlace = up ? "before" : "after";
    const order = movePackage(packages, entry.name, target.name, place);
    if (!order) return;
    // Moving the row can blur its handle, so render the new order now and focus it again.
    const handle = event.currentTarget;
    flushSync(() => save(entry.name, { target: target.name, place, order }));
    handle.focus();
  }

  const isDragging = dragging !== null;
  useEffect(() => {
    if (!isDragging) return;
    const cancel = (event: globalThis.KeyboardEvent) => {
      if (event.key !== "Escape") return;
      event.preventDefault();
      event.stopPropagation();
      end(false);
    };
    // Scrolling moves rows under a resting pointer, so the drop target can change.
    window.addEventListener("scroll", track, { capture: true, passive: true });
    window.addEventListener("keydown", cancel, true);
    return () => {
      window.removeEventListener("scroll", track, { capture: true });
      window.removeEventListener("keydown", cancel, true);
    };
  });

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
    ],
    [apps, detailsPrefix],
  );
  const table = useTable({
    features: packageFeatures,
    data: packages,
    columns,
    getRowId: (entry) => entry.name,
    getRowCanExpand: () => true,
    autoResetExpanded: false,
    initialState: configurePackage ? { expanded: { [configurePackage]: true } } : undefined,
    globalFilterFn: (row, _columnId, value: string) =>
      [row.original.name, ...row.original.appLabels].some((label) =>
        label.toLocaleLowerCase().includes(value.trim().toLocaleLowerCase()),
      ),
  });
  const visibleRows = table.getRowModel().rows;
  const search = String(table.state.globalFilter ?? "");
  const hasConfigurePackage = packages.some((pkg) => pkg.name === configurePackage);

  useEffect(() => {
    if (!hasConfigurePackage) return;
    // Let the settings shell reset its scroll position before revealing the selected row.
    const frame = requestAnimationFrame(() => configureRowRef.current?.scrollIntoView());
    return () => cancelAnimationFrame(frame);
  }, [configurePackage, hasConfigurePackage]);

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
      {onReorder && (
        <p id={hintId} className="sr-only">
          Drag to reorder, or press the Up or Down arrow key. When two packages define the same
          chord, the higher one runs.
        </p>
      )}
      <p className="sr-only" aria-live="polite">
        {announcement}
      </p>
      <div className="chord-library-scroll">
        <table
          ref={tableRef}
          className="chord-library-table package-library-table"
          data-dragging={isDragging || undefined}
        >
          <caption className="sr-only">
            Your chord packages, highest rank first; expand a package to view its placeholders
            and its chords grouped by app
          </caption>
          <thead>
            {table.getHeaderGroups().map((group) => (
              <tr key={group.id}>
                <th scope="col">
                  <span className="sr-only">Order</span>
                </th>
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
              const drop = dragging?.drop?.target === row.id ? dragging.drop.place : undefined;
              return (
                <Fragment key={row.id}>
                  {group && startsGroup && (
                    <tr className="package-library-group">
                      <th scope="colgroup" colSpan={row.getAllCells().length + 1}>
                        <span className="block text-xs font-medium">{group.label}</span>
                        <span className="block text-[11px] font-normal text-muted-foreground">
                          {group.description}
                        </span>
                      </th>
                    </tr>
                  )}
                  <tr
                    ref={row.id === configurePackage ? configureRowRef : undefined}
                    className="package-library-row"
                    data-expanded={expanded}
                    data-package={row.id}
                    data-rank={rank}
                    data-dragging={dragging?.name === row.id || undefined}
                    // An expanded package's line goes under its chords instead.
                    data-drop={
                      drop === "before" || (drop === "after" && !expanded) ? drop : undefined
                    }
                  >
                    <td className="package-library-handle-cell">
                      {onReorder && rankable(rank) && (
                        // The shared Button drops `title` and handles presses itself.
                        <button
                          type="button"
                          className="package-library-handle"
                          aria-label={`Reorder ${row.id}`}
                          aria-describedby={hintId}
                          title="Drag to reorder"
                          onPointerDown={(event) => startDrag(event, row.original)}
                          onPointerMove={moveDrag}
                          onPointerUp={() => end(true)}
                          onPointerCancel={() => end(false)}
                          onLostPointerCapture={() => end(false)}
                          onKeyDown={(event) => stepDrag(event, row.original)}
                        >
                          <GripVertical size={16} strokeWidth={1.8} aria-hidden="true" />
                        </button>
                      )}
                    </td>
                    {row.getAllCells().map((cell) => (
                      <td key={cell.id}>
                        <table.FlexRender cell={cell} />
                      </td>
                    ))}
                  </tr>
                  <tr
                    className="package-library-details"
                    hidden={!expanded}
                    data-drop={drop === "after" && expanded ? drop : undefined}
                  >
                    <td colSpan={row.getAllCells().length + 1}>
                      <div
                        id={`${detailsPrefix}-${encodeURIComponent(row.id)}`}
                        className="package-library-details-content space-y-6"
                      >
                        {expanded && (
                          <>
                            <PackagePlaceholders entries={row.original.placeholders} apps={apps} />
                            <PackageChords
                              editor={editor}
                              rows={row.original.chords}
                              personalDirnames={personalDirnames}
                            />
                          </>
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
