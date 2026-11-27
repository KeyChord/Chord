import type {
  Chord,
  ChordPackage,
  DesktopAppMetadata,
} from "@chord/dev.improve.chord.lib.typeshare";
import { GroupCard } from "@chord/dev.improve.chord.components.ui.group-card";
import {
  listPlaceholders,
  PackagePlaceholders,
  placeholderNeedsKeys,
  type PlaceholderEntry,
} from "@chord/dev.improve.chord.routes.settings._components.package-placeholders";
import { GripVertical, Keyboard, Package } from "lucide-react";
import {
  useEffect,
  useId,
  useLayoutEffect,
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

/** Whether Edit Order has anything to reorder: installed or built-in packages, unfiltered. */
export function canReorderPackages(packages: LibraryPackage[], search = ""): boolean {
  return !search.trim() && packages.some((pkg) => rankable(pkg.rank));
}

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
  /** How far below the card's top edge the pointer grabbed it, so the card follows the pointer. */
  grab: number;
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

/** A package's icon for its card header: the app's own icon, else a glyph for its kind. */
function PackageIcon({ entry, app }: { entry: LibraryPackage; app?: DesktopAppMetadata }) {
  if (app?.iconDataUrl) return <img src={app.iconDataUrl} alt="" />;
  const Icon = entry.rank === "core" ? Keyboard : Package;
  return <Icon size={17} strokeWidth={1.6} />;
}

/** The keys each placeholder stands for in one chords file: the user's keys, else the default. */
export type PlaceholderKeys = (pathslug: string, name: string) => string | undefined;

export function placeholderKeys(entries: PlaceholderEntry[]): PlaceholderKeys {
  return (pathslug, name) => {
    const match =
      entries.find((e) => e.pathslug === pathslug && e.placeholder.name === name) ??
      entries.find((e) => e.placeholder.name === name);
    return match ? (match.placeholder.assigned ?? match.placeholder.default) : undefined;
  };
}

/**
 * A chord sequence on one keycap. Placeholders such as `<>` show the
 * keys they stand for, so the cap reads as what you press; one without keys stays dimmed.
 */
export function ChordKeys({
  sequence,
  resolve,
}: {
  sequence: string;
  resolve?: (name: string) => string | undefined;
}) {
  const tokens = (sequence.match(/<[^<>]*>|[\s\S]/gu) ?? []).map((token) => {
    if (token.length === 1) return { text: token === " " ? "␣" : token };
    const keys = resolve?.(token);
    return keys ? { text: keys, prefix: true } : { text: token, unresolved: true };
  });
  const pressed = tokens.map((token) => token.text).join("");
  return (
    <kbd className="mac-keycap" title={pressed} aria-label={pressed}>
      {tokens.map((token, index) => (
        <span
          key={index}
          data-prefix={token.prefix || undefined}
          data-placeholder={token.unresolved || undefined}
          aria-hidden="true"
        >
          {token.text}
        </span>
      ))}
    </kbd>
  );
}

/** A chord: its name and action type lead, and its keys trail on one keycap. */
function ChordRow({ entry, resolve }: { entry: LibraryChord; resolve?: PlaceholderKeys }) {
  const kind = entry.chord.actions[0] ? actionKinds[entry.chord.actions[0].type] : undefined;
  return (
    <div className="mac-row" data-chord>
      <span className="mac-row-text">
        <span className="mac-row-title" title={entry.chord.name}>
          {entry.chord.name}
        </span>
        <span className="mac-row-subtitle">{kind?.label ?? "No actions"}</span>
      </span>
      <ChordKeys
        sequence={entry.chord.rawTrigger}
        resolve={resolve && ((name) => resolve(entry.pathslug, name))}
      />
    </div>
  );
}

/**
 * What a package shows when its row is disclosed: its placeholders, then its chords under a
 * small heading per app.
 */
export function PackageContents({
  entry,
  apps,
}: {
  entry: LibraryPackage;
  apps: Record<string, DesktopAppMetadata>;
}) {
  const groups = useMemo(() => {
    const byApp = new Map<string | undefined, LibraryChord[]>();
    for (const row of entry.chords) {
      const group = byApp.get(row.appId);
      if (group) group.push(row);
      else byApp.set(row.appId, [row]);
    }
    return [...byApp];
  }, [entry.chords]);
  const resolve = useMemo(() => placeholderKeys(entry.placeholders), [entry.placeholders]);
  return (
    <>
      {/* Inside the package's card, small headings mark each part. */}
      {entry.placeholders.length > 0 && (
        <>
          <div className="mac-row mac-subheader">Placeholders</div>
          <PackagePlaceholders entries={entry.placeholders} apps={apps} inline />
        </>
      )}
      {groups.map(([appId, chords]) => (
        <div key={appId ?? "global"} role="group" aria-label={`${chords[0]!.appLabel} chords`}>
          {(groups.length > 1 || entry.placeholders.length > 0) && (
            <div className="mac-row mac-subheader" title={appId}>
              {chords[0]!.appLabel}
            </div>
          )}
          {chords.map((row) => (
            <ChordRow key={row.id} entry={row} resolve={resolve} />
          ))}
        </div>
      ))}
      {entry.chords.length === 0 && (
        <p className="mac-row mac-row-muted">No chords loaded in this package.</p>
      )}
    </>
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
  initiallyExpanded,
  editing = false,
  onReorder,
}: {
  packages: LibraryPackage[];
  apps: Record<string, DesktopAppMetadata>;
  search?: string;
  /** A package to show disclosed and scrolled into view, such as one opened to configure. */
  initiallyExpanded?: string;
  /** Edit Order mode, toggled from the page's toolbar: cards collapse and show drag handles. */
  editing?: boolean;
  /**
   * Lets the user drag installed and built-in packages within their group, and receives the
   * order to save. Rows stay in the given rank order.
   */
  onReorder?: (order: string[]) => void;
}) {
  const hintId = useId();
  const listRef = useRef<HTMLDivElement>(null);
  const [expanded, setExpanded] = useState<ReadonlySet<string>>(
    () => new Set(initiallyExpanded ? [initiallyExpanded] : []),
  );
  useEffect(() => {
    if (!initiallyExpanded) return;
    listRef.current
      ?.querySelector(`[data-package="${CSS.escape(initiallyExpanded)}"]`)
      ?.scrollIntoView({ block: "start" });
  }, [initiallyExpanded]);
  function toggle(name: string) {
    setExpanded((previous) => {
      const next = new Set(previous);
      if (!next.delete(name)) next.add(name);
      return next;
    });
  }
  // Pointer moves read and update this without waiting for a render; `dragging` mirrors it.
  const dragRef = useRef<DragState | null>(null);
  const [dragging, setDragging] = useState<{ name: string; drop?: Drop } | null>(null);
  const [announcement, setAnnouncement] = useState("");
  const visible = packages.filter((entry) => matchesSearch(entry, search));
  // Reordering a filtered list would be confusing, so Edit needs the full list.
  const canReorder = !!onReorder && canReorderPackages(packages, search);
  const reordering = editing && canReorder;

  function save(name: string, { target, place, order }: Drop) {
    onReorder?.(order);
    setAnnouncement(`Moved ${name} ${place === "before" ? "above" : "below"} ${target}.`);
  }

  /** Where the dragged package would land: between the packages of its group. */
  /** The list's cards by package, with their resting tops (ignoring any slide in progress). */
  function cards() {
    const list = listRef.current?.querySelector<HTMLElement>(".mac-card-list");
    const listTop = list?.getBoundingClientRect().top ?? 0;
    return [...(list?.querySelectorAll<HTMLElement>("[data-package]") ?? [])].map((card) => ({
      card,
      name: card.dataset.package!,
      rank: card.dataset.rank,
      top: listTop + card.offsetTop,
    }));
  }

  /**
   * Where the lifted package would land, as on macOS: once its centre passes another card's
   * middle, that card makes room. Only cards of the same group count.
   */
  function findDrop(drag: DragState): Drop | undefined {
    const all = cards();
    const lifted = all.find((entry) => entry.name === drag.name);
    if (!lifted) return undefined;
    const centre = drag.y - drag.grab + lifted.card.offsetHeight / 2;
    const others = all.filter((entry) => entry.rank === drag.rank && entry.name !== drag.name);
    const slot = others.filter((entry) => entry.top + entry.card.offsetHeight / 2 < centre).length;
    const target = others[Math.min(slot, others.length - 1)];
    if (!target) return undefined;
    const place: DropPlace = slot < others.length ? "before" : "after";
    const order = movePackage(packages, drag.name, target.name, place);
    return order && { target: target.name, place, order };
  }

  /** Keeps the lifted card under the pointer, wherever its slot in the list is now. */
  function follow() {
    const drag = dragRef.current;
    const lifted = drag && cards().find((entry) => entry.name === drag.name);
    if (!drag || !lifted) return;
    lifted.card.style.transform = `translateY(${drag.y - drag.grab - lifted.top}px) scale(1.015)`;
  }

  function track() {
    const drag = dragRef.current;
    if (!drag) return;
    const drop = findDrop(drag);
    if (drop?.order.join() !== drag.drop?.order.join()) {
      // Note where the other cards are now, so they can slide from there to their new places.
      slideFrom.current = new Map(
        cards()
          .filter((entry) => entry.name !== drag.name)
          .map((entry) => [entry.name, entry.card.getBoundingClientRect().top]),
      );
      drag.drop = drop;
      setDragging({ name: drag.name, drop });
    }
    follow();
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
    // Settle the lifted card into its slot.
    const lifted = cards().find((entry) => entry.name === drag.name)?.card;
    if (lifted) {
      lifted.style.transition = "transform 180ms ease";
      lifted.style.transform = "";
      lifted.addEventListener("transitionend", () => (lifted.style.transition = ""), {
        once: true,
      });
    }
    setDragging(null);
    if (commit && drag.drop) save(drag.name, drag.drop);
  }

  function startDrag(event: PointerEvent<HTMLButtonElement>, entry: LibraryPackage) {
    if (event.button !== 0 || dragRef.current) return;
    // Keep the press from selecting text or starting a native drag.
    event.preventDefault();
    const card = event.currentTarget.closest<HTMLElement>("[data-package]");
    dragRef.current = {
      name: entry.name,
      rank: entry.rank,
      y: event.clientY,
      grab: event.clientY - (card?.getBoundingClientRect().top ?? event.clientY),
      scroller: scrollParent(event.currentTarget),
    };
    setDragging({ name: entry.name });
  }

  function moveDrag(y: number) {
    const drag = dragRef.current;
    if (!drag) return;
    drag.y = y;
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
  // Slide the other cards from where they were to their new places, like a macOS list.
  const slideFrom = useRef<Map<string, number> | null>(null);
  useLayoutEffect(() => {
    const from = slideFrom.current;
    slideFrom.current = null;
    if (from) {
      for (const { card, name } of cards()) {
        const before = from.get(name);
        if (before === undefined) continue;
        const delta = before - card.getBoundingClientRect().top;
        if (!delta) continue;
        card.style.transition = "none";
        card.style.transform = `translateY(${delta}px)`;
        requestAnimationFrame(() => {
          card.style.transition = "transform 200ms cubic-bezier(0.2, 0.8, 0.2, 1)";
          card.style.transform = "";
        });
      }
    }
    follow();
  }, [dragging]);
  useEffect(() => {
    if (!isDragging) return;
    const cancel = (event: globalThis.KeyboardEvent) => {
      if (event.key !== "Escape") return;
      event.preventDefault();
      event.stopPropagation();
      end(false);
    };
    // The window follows the pointer, not the handle: the list moves the held card as it
    // reorders, and a moved element loses its pointer capture.
    const move = (event: globalThis.PointerEvent) => moveDrag(event.clientY);
    const up = () => end(true);
    const lost = () => end(false);
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", up);
    window.addEventListener("pointercancel", lost);
    // Scrolling moves cards under a resting pointer, so the drop target can change.
    window.addEventListener("scroll", track, { capture: true, passive: true });
    window.addEventListener("keydown", cancel, true);
    return () => {
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", up);
      window.removeEventListener("pointercancel", lost);
      window.removeEventListener("scroll", track, { capture: true });
      window.removeEventListener("keydown", cancel, true);
    };
  });

  function renderCard(entry: LibraryPackage) {
    const app = entry.appId ? apps[entry.appId] : undefined;
    const needsKeys = entry.placeholders.some(placeholderNeedsKeys);
    const rankLabel = rankGroups[entry.rank]?.label ?? "Local";
    return (
      <div key={entry.name} role="listitem" className="mac-section">
        {/* Each package is its own card: its chords sit inside. */}
        <GroupCard
          icon={<PackageIcon entry={entry} app={app} />}
          title={packageTitle(entry, app)}
          subtitle={
            <>
              {rankLabel} · <span title={entry.name}>{entry.name}</span>
              {needsKeys && (
                <span className="text-amber-700 dark:text-amber-300"> · needs keys</span>
              )}
            </>
          }
          count={entry.chords.length}
          headerLabel={`Show ${entry.name} chords`}
          listLabel={`${entry.name} details`}
          open={!reordering && expanded.has(entry.name)}
          dragging={dragging?.name === entry.name}
          onOpenChange={() => toggle(entry.name)}
          rootProps={{
            "data-package": entry.name,
            "data-rank": entry.rank,
          }}
          actions={
            reordering &&
            rankable(entry.rank) && (
              // The shared Button drops `title` and handles presses itself.
              <button
                type="button"
                className="mac-handle"
                aria-label={`Reorder ${entry.name}`}
                aria-describedby={hintId}
                title="Drag to reorder"
                onPointerDown={(event) => startDrag(event, entry)}
                onKeyDown={(event) => stepDrag(event, entry)}
              >
                <GripVertical size={15} strokeWidth={1.8} aria-hidden="true" />
              </button>
            )
          }
        >
          <PackageContents entry={entry} apps={apps} />
        </GroupCard>
      </div>
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
      <div
        role="list"
        aria-label="Packages"
        className="mac-card-list"
        data-dragging={isDragging || undefined}
      >
        {/* While dragging, the list shows where everything would land. */}
        {(dragging?.drop ? applyPackageOrder(visible, dragging.drop.order) : visible).map(
          renderCard,
        )}
      </div>
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
