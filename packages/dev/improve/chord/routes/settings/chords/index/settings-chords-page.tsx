import { applyPackageOrder, ChordLibrary, listLibraryPackages } from "#chord-library";
import { useMutation } from "@chord/com.npmjs.tanstack__react-query";
import { taurpc } from "@chord/dev.improve.chord.api.taurpc";
import { PackageApprovals } from "#package-approvals";
import { Button } from "@chord/dev.improve.chord.components.ui.button";
import { Input } from "@chord/dev.improve.chord.components.ui.input";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@chord/dev.improve.chord.components.ui.collapsible";
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
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@chord/dev.improve.chord.components.ui.tabs";
import { ChevronRight, Settings2 } from "lucide-react";
import { useId, useMemo, useState } from "react";

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

export function SettingsChordsPageContent({
  chordPackageManagerData,
  desktopAppManagerData,
  configurePackage,
}: {
  chordPackageManagerData: NonNullable<ReturnType<typeof useChordPackageManagerState>["data"]>;
  desktopAppManagerData?: NonNullable<ReturnType<typeof useDesktopAppManagerState>["data"]>;
  configurePackage?: string;
}) {
  const [activeTab, setActiveTab] = useState("packages");
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
  // Trust warnings lead only when there is a decision to make. Otherwise, someone new would meet
  // "can run arbitrary code" before seeing a single chord.
  const thirdPartyPackages = chordPackageManagerData.thirdPartyPackages ?? [];
  const needsApproval = thirdPartyPackages.some((pkg) => !pkg.approved);
  const approvals = (
    <PackageApprovals
      key="approvals"
      packages={thirdPartyPackages}
      errors={chordPackageManagerData.errors}
    />
  );
  const library = (
    <ChordLibrary
      key="library"
      packages={shownPackages}
      apps={apps}
      personalDirnames={{}}
      configurePackage={configurePackage}
      onReorder={setPackageOrder}
    />
  );
  return (
    <Tabs value={activeTab} onValueChange={setActiveTab} className="gap-4">
      <TabsList aria-label="Chords views">
        <TabsTrigger value="packages" className="min-w-max whitespace-nowrap px-3 text-[13px]">
          Packages
        </TabsTrigger>
        <TabsTrigger value="personal" className="min-w-max whitespace-nowrap px-3 text-[13px]">
          Local Chords
        </TabsTrigger>
      </TabsList>
      <TabsContent
        value="packages"
        forceMount
        hidden={activeTab !== "packages"}
        className="space-y-4 p-0"
      >
        {needsApproval ? [approvals, library] : [library, approvals]}
        <DefaultGlobalPrefixSetting />
        <Collapsible className="group/sources preference-disclosure">
          <CollapsibleTrigger className="flex h-auto w-full items-center justify-start gap-3 px-4 py-3 text-[13px] font-medium">
            <Settings2 className="size-4 text-muted-foreground" /> Manage sources
            <ChevronRight className="ml-auto size-4 text-muted-foreground transition-transform group-data-[expanded=true]/sources:rotate-90" />
          </CollapsibleTrigger>
          <CollapsibleContent>
            <div className="space-y-5 px-4 pb-4">
              <LocalFoldersCard />
              <ChordReposCard />
              <ChordMonoreposCard />
            </div>
          </CollapsibleContent>
        </Collapsible>
      </TabsContent>
      <TabsContent value="personal" forceMount hidden={activeTab !== "personal"} className="p-0">
        <div className="preference-disclosure p-5">
          <PersonalChords />
        </div>
      </TabsContent>
    </Tabs>
  );
}

function DefaultGlobalPrefixSetting() {
  const current = useSettingsState().data?.defaultGlobalPrefixForPlaceholders;
  const [draft, setDraft] = useState<string | null>(null);
  const inputId = useId();
  const save = useMutation({
    mutationFn: (symbol: string) => taurpc.setDefaultGlobalPrefixForPlaceholders(symbol),
    onSuccess: () => setDraft(null),
  });
  if (current === undefined) return null;
  const canSave = !!draft && (draft !== current || save.isError) && !save.isPending;

  return (
    <form
      className="preference-disclosure flex flex-wrap items-center gap-x-3 gap-y-2 px-4 py-3"
      onSubmit={(event) => {
        event.preventDefault();
        if (canSave) save.mutate(draft);
      }}
    >
      <div className="min-w-44 flex-1">
        <label htmlFor={inputId} className="text-[13px] font-medium">
          Default global prefix for placeholders
        </label>
        <p id={`${inputId}-help`} className="text-xs text-(--settings-description)">
          Added before a package&apos;s prefix letter, unless the package has a custom prefix.
        </p>
      </div>
      <div className="flex shrink-0 items-center gap-2">
        <Input
          id={inputId}
          aria-describedby={`${inputId}-help`}
          value={draft ?? current}
          onChange={(event) => {
            save.reset();
            setDraft(event.target.value);
          }}
          className="h-7 w-10 px-0 text-center font-mono"
          autoComplete="off"
          spellCheck={false}
          maxLength={1}
          disabled={save.isPending}
        />
        <Button type="submit" variant="outline" size="sm" disabled={!canSave}>
          {save.isPending ? "Saving..." : "Save"}
        </Button>
      </div>
      {save.isError && (
        <p role="alert" className="w-full text-xs text-destructive">
          {typeof save.error === "object" && "Message" in save.error
            ? String(save.error.Message)
            : String(save.error)}
        </p>
      )}
    </form>
  );
}
