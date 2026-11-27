import {
  applyPackageOrder,
  canReorderPackages,
  ChordLibrary,
  listLibraryPackages,
} from "#chord-library";
import { useMutation } from "@chord/com.npmjs.tanstack__react-query";
import { taurpc } from "@chord/dev.improve.chord.api.taurpc";
import { PackageApprovals } from "#package-approvals";
import {
  StateQueries,
  useChordPackageManagerState,
  useDesktopAppManagerState,
  useSettingsState,
} from "@chord/dev.improve.chord.lib.state";
import { ChordMonoreposCard } from "@chord/dev.improve.chord.routes.settings._components.chord-monorepos-card";
import { ChordReposCard } from "@chord/dev.improve.chord.routes.settings._components.chord-repos-card";
import { LocalFoldersCard } from "@chord/dev.improve.chord.routes.settings._components.local-folders-card";
import { PersonalChords } from "@chord/dev.improve.chord.routes.settings._components.personal-chords";
import { ChevronRight, ListOrdered, Search, X } from "lucide-react";
import { useId, useMemo, useRef, useState, type CSSProperties, type KeyboardEvent } from "react";

export function SettingsChordsPage({ configurePackage }: { configurePackage?: string }) {
  return (
    <StateQueries queries={[useChordPackageManagerState(), useDesktopAppManagerState()]}>
      {(chordPackageManagerData, desktopAppManagerData) => (
        <SettingsChordsPageContent
          chordPackageManagerData={chordPackageManagerData}
          desktopAppManagerData={desktopAppManagerData}
          configurePackage={configurePackage}
        />
      )}
    </StateQueries>
  );
}

type View = "packages" | "personal";

const views: { value: View; label: string }[] = [
  { value: "packages", label: "Packages" },
  { value: "personal", label: "Local Chords" },
];

/**
 * A capsule segmented control. Segments share one width, so the control never resizes, a thumb
 * slides to the selected one, and the arrow keys move between them like an AppKit tab view.
 */
function SegmentedControl({
  value,
  onChange,
  badges,
  idPrefix,
}: {
  value: View;
  onChange: (value: View) => void;
  badges: Partial<Record<View, number>>;
  idPrefix: string;
}) {
  const refs = useRef<Partial<Record<View, HTMLButtonElement | null>>>({});
  function onKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    const step = event.key === "ArrowRight" ? 1 : event.key === "ArrowLeft" ? -1 : 0;
    if (!step) return;
    event.preventDefault();
    const index = views.findIndex((view) => view.value === value);
    const next = views[(index + step + views.length) % views.length]!.value;
    onChange(next);
    refs.current[next]?.focus();
  }
  return (
    <div
      role="tablist"
      aria-label="Chords views"
      className="mac-segmented"
      style={
        {
          "--count": views.length,
          "--index": views.findIndex((view) => view.value === value),
        } as CSSProperties
      }
      onKeyDown={onKeyDown}
    >
      {views.map((view) => {
        const selected = view.value === value;
        const badge = badges[view.value];
        return (
          <button
            key={view.value}
            ref={(node) => {
              refs.current[view.value] = node;
            }}
            type="button"
            role="tab"
            id={`${idPrefix}-${view.value}-tab`}
            aria-controls={`${idPrefix}-${view.value}`}
            aria-selected={selected}
            tabIndex={selected ? 0 : -1}
            className="mac-segment"
            onClick={() => onChange(view.value)}
          >
            {view.label}
            {badge ? (
              <span className="mac-segment-badge" aria-label={`${badge} awaiting approval`}>
                {badge}
              </span>
            ) : null}
          </button>
        );
      })}
    </div>
  );
}

export function SettingsChordsPageContent({
  chordPackageManagerData,
  desktopAppManagerData,
  configurePackage,
}: {
  chordPackageManagerData: NonNullable<ReturnType<typeof useChordPackageManagerState>["data"]>;
  desktopAppManagerData?: NonNullable<ReturnType<typeof useDesktopAppManagerState>["data"]>;
  configurePackage?: string;
}) {
  const idPrefix = useId();
  const [view, setView] = useState<View>("packages");
  const [search, setSearch] = useState("");
  // Local Chords keeps its own query, so switching tabs doesn't carry a search across.
  const [localSearch, setLocalSearch] = useState("");
  const [editingOrder, setEditingOrder] = useState(false);
  const { packages, personalPackages } = chordPackageManagerData;
  const apps = useMemo(
    () => desktopAppManagerData?.appsMetadata ?? {},
    [desktopAppManagerData?.appsMetadata],
  );
  const libraryPackages = useMemo(() => {
    const personalNames = new Set(
      personalPackages.flatMap((state) => (state.name ? [state.name] : [])),
    );
    return listLibraryPackages(
      packages.filter((pkg) => !personalNames.has(pkg.name)),
      personalNames,
      apps,
      new Set(chordPackageManagerData.builtinPackages ?? []),
    );
  }, [packages, personalPackages, apps, chordPackageManagerData.builtinPackages]);
  const {
    mutate: setPackageOrder,
    variables: savingOrder,
    isPending: savingPackageOrder,
  } = useMutation({
    mutationFn: (order: string[]) => taurpc.setPackageOrder(order),
    // Saves run one at a time, so the last order the user chose is the one Chord keeps.
    scope: { id: "package-order" },
  });
  // Show a dropped package in its new place while Chord saves the order and reranks.
  const shownPackages =
    savingPackageOrder && savingOrder
      ? applyPackageOrder(libraryPackages, savingOrder)
      : libraryPackages;
  const thirdPartyPackages = chordPackageManagerData.thirdPartyPackages ?? [];
  const pendingApprovals = thirdPartyPackages.filter((pkg) => !pkg.approved).length;

  const thirdParty = (
    <div className="mac-third-party">
      <PackageApprovals packages={thirdPartyPackages} errors={chordPackageManagerData.errors} />
    </div>
  );

  function panel(value: View) {
    return {
      role: "tabpanel",
      id: `${idPrefix}-${value}`,
      "aria-labelledby": `${idPrefix}-${value}-tab`,
      hidden: view !== value,
    } as const;
  }

  return (
    <div className="mac-page">
      <div className="mac-toolbar">
        <SegmentedControl
          value={view}
          onChange={setView}
          badges={{ packages: pendingApprovals }}
          idPrefix={idPrefix}
        />
        {view === "packages" && canReorderPackages(shownPackages, search) && (
          <button
            type="button"
            className="mac-circle-button ml-auto"
            aria-pressed={editingOrder}
            aria-label={editingOrder ? "Done Editing Order" : "Edit Order"}
            title={
              editingOrder
                ? "Done"
                : "Edit Order: when two packages define the same chord, the higher one runs."
            }
            onClick={() => setEditingOrder(!editingOrder)}
          >
            <ListOrdered size={15} strokeWidth={2} aria-hidden="true" />
          </button>
        )}
        {
          <label className="mac-search">
            <Search size={13} aria-hidden="true" />
            <input
              aria-label={view === "packages" ? "Search packages" : "Search your chords files"}
              placeholder="Search"
              value={view === "packages" ? search : localSearch}
              onChange={(event) =>
                (view === "packages" ? setSearch : setLocalSearch)(event.target.value)
              }
            />
            {(view === "packages" ? search : localSearch) && (
              <button
                type="button"
                aria-label="Clear search"
                onClick={() => (view === "packages" ? setSearch : setLocalSearch)("")}
              >
                <X size={13} />
              </button>
            )}
          </label>
        }
      </div>

      <div {...panel("packages")}>
        {/*
         * Third-party packages live among the others, not in a tab of their own: at the top
         * while one waits for approval, so it's seen, and at the end otherwise.
         */}
        {pendingApprovals > 0 && thirdParty}
        {/* Opening Chords to configure a package discloses that package in place. */}
        <ChordLibrary
          packages={shownPackages}
          apps={apps}
          search={search}
          initiallyExpanded={configurePackage}
          editing={editingOrder && canReorderPackages(shownPackages, search)}
          onReorder={setPackageOrder}
        />
        <DefaultGlobalPrefixSetting />
        {pendingApprovals === 0 && thirdParty}
        <OtherSources />
      </div>
      <div {...panel("personal")}>
        <PersonalChords search={localSearch} />
      </div>
    </div>
  );
}

/**
 * Packages from places besides Chord's own catalog: a local folder, a GitHub repository or a
 * local monorepo. Folded away, since most people never need it.
 */
function OtherSources() {
  return (
    <details className="mac-section other-sources">
      <summary className="mac-section-header other-sources-summary">
        <ChevronRight size={13} aria-hidden="true" className="other-sources-chevron" />
        <span className="mac-section-title">Other sources</span>
        <span className="mac-section-subtitle">Local folders and GitHub repositories</span>
      </summary>
      <div className="space-y-5 pt-2">
        <LocalFoldersCard />
        <ChordReposCard />
        <ChordMonoreposCard />
      </div>
    </details>
  );
}

/** Saves the prefix as soon as it changes, like a native preference: no Save button. */
function DefaultGlobalPrefixSetting() {
  const current = useSettingsState().data?.defaultGlobalPrefixForPlaceholders;
  const [draft, setDraft] = useState<string | null>(null);
  const inputId = useId();
  const save = useMutation({
    mutationFn: (symbol: string) => taurpc.setDefaultGlobalPrefixForPlaceholders(symbol),
    onSuccess: () => setDraft(null),
  });
  if (current === undefined) return null;

  return (
    <div className="mac-section">
      <div className="mac-list">
        <div className="mac-row">
          <label htmlFor={inputId} className="mac-row-text">
            <span className="mac-row-title">Default global prefix for placeholders</span>
            <span id={`${inputId}-help`} className="mac-row-subtitle">
              Added before each package&apos;s prefix letter. Expand a package to give it its own.
            </span>
          </label>
          <input
            id={inputId}
            aria-describedby={`${inputId}-help`}
            value={draft ?? current}
            onChange={(event) => {
              const next = event.target.value;
              save.reset();
              setDraft(next);
              if (next && next !== current) save.mutate(next);
            }}
            className="mac-field w-9 px-0 text-center font-mono"
            autoComplete="off"
            spellCheck={false}
            maxLength={1}
            disabled={save.isPending}
          />
        </div>
      </div>
      {save.isError ? (
        <p role="alert" className="mac-section-footer text-destructive">
          {typeof save.error === "object" && "Message" in save.error
            ? String(save.error.Message)
            : String(save.error)}
        </p>
      ) : null}
    </div>
  );
}
