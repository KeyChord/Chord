import { AppsNeedingRelaunchCard } from "./apps-needing-relaunch-card.tsx";
import { LocalFoldersCard } from "./local-folders-card.tsx";
import { LaunchOnLoginCard } from "./launch-on-login-card.tsx";
import { ChordReposCard } from "./chord-repos-card.tsx";
import { PermissionsCard } from "./permissions-card.tsx";

export function SettingsTab() {
  return (
    <div className="space-y-4">
      <ChordReposCard />
      <LocalFoldersCard />
      <AppsNeedingRelaunchCard />
      <PermissionsCard />
      <LaunchOnLoginCard />
    </div>
  );
}
