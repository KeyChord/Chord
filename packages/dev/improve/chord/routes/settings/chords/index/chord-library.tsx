import type {
  Chord,
  ChordPackage,
  DesktopAppMetadata,
} from "@chord/dev.improve.chord.lib.typeshare";
import {
  listPlaceholders,
  PackagePlaceholders,
  placeholderNeedsKeys,
  type PlaceholderEntry,
} from "@chord/dev.improve.chord.routes.settings._components.package-placeholders";
import {
  AppWindow,
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  Globe,
  GripVertical,
  Keyboard,
  Package,
  TriangleAlert,
} from "lucide-react";
import {
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
        rank:
          pkg.name === CORE_PACKAGE
            ? "core"
            : personal
              ? "personal"
              : builtinNames.has(pkg.name)
                ? "builtin"
                : "installed",
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
  core: {
    label: "Core",
    description: "Chord's own chords. They override every other package.",
  },
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

const actionKinds = {
  shortcut: { label: "Shortcut" },
  shell: { label: "Shell command" },
  emit: { label: "Script" },
  builtin: { label: "Chord action" },
} as const;

/** Splits `AccessibilityInspector` into `Accessibility Inspector`. */
function spaceWords(text: string) {
  return text.replace(/([a-z])([A-Z])/g, "$1 $2");
}

/**
 * A readable title for a package: the app's own name for an app package, otherwise the part
 * after `chords-`, so `@keychord/chords-spotify` reads as "Spotify".
 */
export function packageTitle(entry: LibraryPackage, app?: DesktopAppMetadata): string {
  const appName = app?.displayName?.trim();
  if (appName) return appName;
  const base = entry.name.replace(/^@[^/]+\//, "").replace(/^chords-/, "");
  // Reverse-DNS names (com.apple.Safari) read best as their last part.
  const last = /^[a-z]+\.[a-z0-9-]+\./i.test(base) ? base.split(".").at(-1)! : base;
  const words = spaceWords(last).replace(/[-_]+/g, " ");
  return words.charAt(0).toUpperCase() + words.slice(1);
}

function plural(count: number, noun: string) {
  return `${count} ${noun}${count === 1 ? "" : "s"}`;
}

function PackageIcon({ entry, app }: { entry: LibraryPackage; app?: DesktopAppMetadata }) {
  if (app?.iconDataUrl) {
    return (
      <span className="mac-icon" aria-hidden="true">
        <img src={app.iconDataUrl} alt="" />
      </span>
    );
  }
  const Icon = entry.rank === "core" ? Keyboard : Package;
  return (
    <span className="mac-icon" aria-hidden="true">
      <Icon size={17} strokeWidth={1.6} />
    </span>
  );
}

/**
 * A chord sequence on one keycap, like Homerow's hint labels. A placeholder such as `<>` is
 * dimmed, since the user's prefix replaces it.
 */
export function ChordKeys({ sequence }: { sequence: string }) {
  const keys = sequence.match(/<[^<>]*>|[\s\S]/gu) ?? [];
  return (
    <kbd className="mac-keycap" title={sequence} aria-label={sequence}>
      {keys.map((key, index) => (
        <span key={index} data-placeholder={key.length > 1 || undefined} aria-hidden="true">
          {key === " " ? "␣" : key}
        </span>
      ))}
    </kbd>
  );
}

/** A chord: its name and action type lead, and its keys trail on one keycap. */
function ChordRow({ entry }: { entry: LibraryChord }) {
  const kind = entry.chord.actions[0] ? actionKinds[entry.chord.actions[0].type] : undefined;
  return (
    <div className="mac-row" data-chord>
      <span className="mac-row-text">
        <span className="mac-row-title" title={entry.chord.name}>
          {entry.chord.name}
        </span>
        <span className="mac-row-subtitle">{kind?.label ?? "No actions"}</span>
      </span>
      <ChordKeys sequence={entry.chord.rawTrigger} />
    </div>
  );
}

/** One app's chords in a card that collapses under its header, like Homerow's app groups. */
function ChordGroup({
  appId,
  chords,
  apps,
}: {
  appId: string | undefined;
  chords: LibraryChord[];
  apps: Record<string, DesktopAppMetadata>;
}) {
  const [open, setOpen] = useState(true);
  const listId = useId();
  const label = chords[0]!.appLabel;
  const icon = appId ? apps[appId]?.iconDataUrl : undefined;
  return (
    <div className="mac-section">
      <div className="mac-list mac-group" data-open={open || undefined}>
        <h3>
          <button
            type="button"
            className="mac-group-header"
            aria-expanded={open}
            aria-controls={listId}
            title={appId}
            onClick={() => setOpen(!open)}
          >
            <span className="mac-icon" aria-hidden="true">
              {icon ? (
                <img src={icon} alt="" />
              ) : appId ? (
                <AppWindow size={17} strokeWidth={1.6} />
              ) : (
                <Globe size={17} strokeWidth={1.6} />
              )}
            </span>
            <span className="mac-row-title flex-1">{label}</span>
            <span className="mac-row-detail">{chords.length}</span>
            <ChevronDown
              className="mac-group-chevron"
              size={15}
              strokeWidth={2}
              aria-hidden="true"
            />
          </button>
        </h3>
        <div id={listId} role="list" aria-label={`${label} chords`} hidden={!open}>
          {chords.map((row) => (
            <div key={row.id} role="listitem">
              <ChordRow entry={row} />
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

/** A package's chords, one section per app, with its placeholders on top. */
export function PackageDetail({
  entry,
  apps,
  onBack,
}: {
  entry: LibraryPackage;
  apps: Record<string, DesktopAppMetadata>;
  onBack: () => void;
}) {
  const app = entry.appId ? apps[entry.appId] : undefined;
  const groups = useMemo(() => {
    const byApp = new Map<string | undefined, LibraryChord[]>();
    for (const row of entry.chords) {
      const group = byApp.get(row.appId);
      if (group) group.push(row);
      else byApp.set(row.appId, [row]);
    }
    return [...byApp];
  }, [entry.chords]);
  const rankLabel = rankGroups[entry.rank]?.label ?? "Local";

  return (
    <section aria-label={`${entry.name} details`}>
      <div className="mac-detail-bar">
        <button type="button" className="mac-back" onClick={onBack}>
          <ChevronLeft size={16} strokeWidth={2} aria-hidden="true" />
          Packages
        </button>
      </div>
      <div className="mac-detail-header">
        {app?.iconDataUrl && <PackageIcon entry={entry} app={app} />}
        <div className="min-w-0">
          <h2 className="mac-detail-title truncate">{packageTitle(entry, app)}</h2>
          <p className="mac-detail-subtitle truncate" title={entry.name}>
            {entry.name} · {rankLabel} · {plural(entry.chords.length, "chord")}
          </p>
        </div>
      </div>

      <PackagePlaceholders entries={entry.placeholders} apps={apps} />

      {groups.map(([appId, chords]) => (
        <ChordGroup key={appId ?? "global"} appId={appId} chords={chords} apps={apps} />
      ))}
      {entry.chords.length === 0 && (
        <div className="mac-section">
          <div className="mac-list">
            <p className="mac-row-empty">No chords loaded in this package.</p>
          </div>
        </div>
      )}
    </section>
  );
}

function matchesSearch(entry: LibraryPackage, search: string) {
  const query = search.trim().toLocaleLowerCase();
  return (
    !query ||
    [entry.name, ...entry.appLabels].some((label) => label.toLocaleLowerCase().includes(query))
  );
}

/**
 * The package library: inset grouped lists for core, installed and built-in packages. A row
 * opens the package; Edit shows handles to reorder installed and built-in packages.
 */
export function ChordLibrary({
  packages,
  apps,
  search = "",
  onOpen,
  onReorder,
}: {
  packages: LibraryPackage[];
  apps: Record<string, DesktopAppMetadata>;
  search?: string;
  onOpen?: (name: string) => void;
  /**
   * Lets the user drag installed and built-in packages within their group, and receives the
   * order to save. Rows stay in the given rank order.
   */
  onReorder?: (order: string[]) => void;
}) {
  const hintId = useId();
  const listRef = useRef<HTMLDivElement>(null);
  const [editing, setEditing] = useState(false);
  // Pointer moves read and update this without waiting for a render; `dragging` mirrors it.
  const dragRef = useRef<DragState | null>(null);
  const [dragging, setDragging] = useState<{ name: string; drop?: Drop } | null>(null);
  const [announcement, setAnnouncement] = useState("");
  const visible = packages.filter((entry) => matchesSearch(entry, search));
  // Reordering a filtered list would be confusing, so Edit needs the full list.
  const canReorder = !!onReorder && !search.trim() && packages.some((pkg) => rankable(pkg.rank));
  const reordering = editing && canReorder;

  function save(name: string, { target, place, order }: Drop) {
    onReorder?.(order);
    setAnnouncement(`Moved ${name} ${place === "before" ? "above" : "below"} ${target}.`);
  }

  /** Where the dragged package would land: between the packages of its group. */
  function findDrop(drag: DragState): Drop | undefined {
    const rows = listRef.current?.querySelectorAll<HTMLElement>(
      `.mac-row[data-rank="${drag.rank}"]`,
    );
    const blocks = [...(rows ?? [])].map((row) => {
      const { top, bottom } = row.getBoundingClientRect();
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
    const group = packages.filter((pkg) => pkg.rank === entry.rank);
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

  const sections = rankOrder.flatMap((rank) => {
    const rows = visible.filter((entry) => entry.rank === rank);
    return rows.length ? [{ rank, rows }] : [];
  });

  function renderRow(entry: LibraryPackage) {
    const app = entry.appId ? apps[entry.appId] : undefined;
    const drop = dragging?.drop?.target === entry.name ? dragging.drop.place : undefined;
    const needsKeys = entry.placeholders.some(placeholderNeedsKeys);
    const title = packageTitle(entry, app);
    const content = (
      <>
        <PackageIcon entry={entry} app={app} />
        <span className="mac-row-text">
          <span className="mac-row-title">{title}</span>
          <span className="mac-row-subtitle" title={entry.name}>
            {entry.name}
          </span>
        </span>
        {needsKeys && (
          <span className="mac-row-detail inline-flex items-center gap-1 text-amber-700 dark:text-amber-300">
            <TriangleAlert size={13} aria-hidden="true" />
            needs keys
          </span>
        )}
        <span className="mac-row-detail">
          {plural(entry.chords.length, "chord")}
          {entry.placeholders.length > 0 && (
            <> · {plural(entry.placeholders.length, "placeholder")}</>
          )}
        </span>
      </>
    );
    if (reordering) {
      return (
        <div
          key={entry.name}
          className="mac-row"
          data-package={entry.name}
          data-rank={entry.rank}
          data-dragging={dragging?.name === entry.name || undefined}
          data-drop={drop}
        >
          {content}
          {rankable(entry.rank) && (
            // The shared Button drops `title` and handles presses itself.
            <button
              type="button"
              className="mac-handle"
              aria-label={`Reorder ${entry.name}`}
              aria-describedby={hintId}
              title="Drag to reorder"
              onPointerDown={(event) => startDrag(event, entry)}
              onPointerMove={moveDrag}
              onPointerUp={() => end(true)}
              onPointerCancel={() => end(false)}
              onLostPointerCapture={() => end(false)}
              onKeyDown={(event) => stepDrag(event, entry)}
            >
              <GripVertical size={15} strokeWidth={1.8} aria-hidden="true" />
            </button>
          )}
        </div>
      );
    }
    return (
      <button
        key={entry.name}
        type="button"
        className="mac-row"
        data-package={entry.name}
        data-rank={entry.rank}
        aria-label={`Open ${entry.name}`}
        onClick={() => onOpen?.(entry.name)}
      >
        {content}
        <ChevronRight className="mac-row-chevron" size={15} strokeWidth={2} aria-hidden="true" />
      </button>
    );
  }

  return (
    <section aria-label="Chord packages" ref={listRef}>
      <p className="sr-only" role="status">
        {plural(visible.length, "package")}
      </p>
      {canReorder && (
        <p id={hintId} className="sr-only">
          Drag to reorder, or press the Up or Down arrow key. When two packages define the same
          chord, the higher one runs.
        </p>
      )}
      <p className="sr-only" aria-live="polite">
        {announcement}
      </p>
      {sections.map(({ rank, rows }) => {
        const group = rankGroups[rank];
        const showEdit = canReorder && rank === "installed";
        return (
          <div key={rank} className="mac-section">
            <div className="mac-section-header">
              <h3 className="mac-section-title">{group?.label ?? "Local"}</h3>
              {showEdit && (
                <button
                  type="button"
                  className="mac-button -mr-2.5 ml-auto"
                  data-borderless
                  aria-pressed={reordering}
                  onClick={() => setEditing(!reordering)}
                >
                  {reordering ? "Done" : "Edit Order"}
                </button>
              )}
            </div>
            <div className="mac-list" data-icons data-dragging={isDragging || undefined}>
              {rows.map(renderRow)}
            </div>
            {group && <p className="mac-section-footer">{group.description}</p>}
          </div>
        );
      })}
      {visible.length === 0 && (
        <div className="mac-list">
          <div className="px-4 py-8 text-center">
            <Package className="mx-auto mb-3 size-7 opacity-40" strokeWidth={1.4} />
            <p>{packages.length === 0 ? "Your package library is empty" : "No packages found"}</p>
            <p className="mac-row-subtitle mt-1">
              {packages.length === 0
                ? "Find a package in Browse to get started."
                : "Try a different package name or app."}
            </p>
          </div>
        </div>
      )}
    </section>
  );
}
