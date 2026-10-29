import { ChordLibrary, listLibraryPackages } from "#chord-library";
import { PackageApprovals } from "#package-approvals";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@chord/dev.improve.chord.components.ui.collapsible";
import {
  StateQueries,
  useChordPackageManagerState,
  useDesktopAppManagerState,
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
import { useMemo, useState } from "react";

export function SettingsChordsPage() {
  return (
    <StateQueries queries={[useChordPackageManagerState(), useDesktopAppManagerState()]}>
      {(chordPackageManagerData, desktopAppManagerData) => (
        <SettingsChordsPageContent
          chordPackageManagerData={chordPackageManagerData}
          desktopAppManagerData={desktopAppManagerData}
        />
      )}
    </StateQueries>
  );
}

export function SettingsChordsPageContent({
  chordPackageManagerData,
  desktopAppManagerData,
}: {
  chordPackageManagerData: NonNullable<ReturnType<typeof useChordPackageManagerState>["data"]>;
  desktopAppManagerData?: NonNullable<ReturnType<typeof useDesktopAppManagerState>["data"]>;
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
    );
  }, [packages, personalPackages, apps]);
  return (
    <Tabs value={activeTab} onValueChange={setActiveTab} className="gap-7">
      <TabsList aria-label="Chords views">
        <TabsTrigger
          value="packages"
          className="min-w-28 px-4 text-[13px]"
        >
          Packages
        </TabsTrigger>
        <TabsTrigger
          value="personal"
          className="min-w-28 px-4 text-[13px]"
        >
          My Chords
        </TabsTrigger>
      </TabsList>
      <TabsContent
        value="packages"
        forceMount
        hidden={activeTab !== "packages"}
        className="space-y-7"
      >
        <PackageApprovals
          packages={chordPackageManagerData.thirdPartyPackages ?? []}
          errors={chordPackageManagerData.errors}
        />
        <ChordLibrary packages={libraryPackages} apps={apps} personalDirnames={{}} />
        <Collapsible className="group/sources preference-disclosure">
          <CollapsibleTrigger className="flex h-auto w-full items-center justify-start gap-3 px-5 py-4 text-sm font-normal">
            <Settings2 className="size-4 text-muted-foreground" /> Manage sources
            <ChevronRight className="ml-auto size-4 text-muted-foreground transition-transform group-data-[expanded=true]/sources:rotate-90" />
          </CollapsibleTrigger>
          <CollapsibleContent className="space-y-5 px-5 pb-5">
            <LocalFoldersCard />
            <ChordReposCard />
            <ChordMonoreposCard />
          </CollapsibleContent>
        </Collapsible>
      </TabsContent>
      <TabsContent value="personal" forceMount hidden={activeTab !== "personal"}>
        <div className="preference-disclosure p-5">
          <PersonalChords />
        </div>
      </TabsContent>
    </Tabs>
  );
}
