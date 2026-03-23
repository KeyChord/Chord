import { AppsNeedingRelaunchCard } from "./apps-needing-relaunch-card.tsx";
import { LocalFoldersCard } from "./local-folders-card.tsx";
import { LaunchOnLoginCard } from "./launch-on-login-card.tsx";

export function SettingsTab() {
  return (
    <div className="space-y-4">
      <LocalFoldersCard />
      <AppsNeedingRelaunchCard />
      <LaunchOnLoginCard />
    </div>
  );
}
