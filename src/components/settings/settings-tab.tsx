import { ActivationTriggerCard } from "./activation-trigger-card.tsx";
import { LaunchOnLoginCard } from "./launch-on-login-card.tsx";
import { PermissionsCard } from "./permissions-card.tsx";

export function SettingsTab() {
  return (
    <div className="space-y-4">
      <PermissionsCard />
      <ActivationTriggerCard />
      <LaunchOnLoginCard />
    </div>
  );
}
