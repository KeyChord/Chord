import { AppsNeedingRelaunchCard } from "./apps-needing-relaunch-card.tsx";
import { ChordReposCard } from "./chord-repos-card.tsx";
import { LocalFoldersCard } from "./local-folders-card.tsx";

export function ChordsTab() {
  return (
    <div className="space-y-4">
      <ChordReposCard />
      <LocalFoldersCard />
      <AppsNeedingRelaunchCard />
    </div>
  );
}
