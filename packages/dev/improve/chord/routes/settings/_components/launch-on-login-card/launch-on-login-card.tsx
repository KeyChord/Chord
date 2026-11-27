import { useMutation } from "@chord/com.npmjs.tanstack__react-query";
import { taurpc } from "@chord/dev.improve.chord.api.taurpc";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@chord/dev.improve.chord.components.ui.card";
import { Switch } from "@chord/dev.improve.chord.components.ui.switch";
import { ToggleGroup, ToggleGroupItem } from "@chord/dev.improve.chord.components.ui.toggle-group";
import { Label } from "@chord/dev.improve.chord.components.ui.label";
import {
  StateQueries,
  desktopPlatform,
  usePermissionsState,
  useSettingsState,
} from "@chord/dev.improve.chord.lib.state";
import {
  type Appearance,
  getAppearance,
  setAppearance,
} from "@chord/dev.improve.chord.lib.state/appearance";
import { useState } from "react";

export function LaunchOnLoginCard() {
  return (
    <StateQueries queries={[usePermissionsState(), useSettingsState()]}>
      {(permissionsData, settingsData) => (
        <LaunchOnLoginCardContent permissionsData={permissionsData} settingsData={settingsData} />
      )}
    </StateQueries>
  );
}

function LaunchOnLoginCardContent({
  permissionsData,
  settingsData,
}: {
  permissionsData: NonNullable<ReturnType<typeof usePermissionsState>["data"]>;
  settingsData: NonNullable<ReturnType<typeof useSettingsState>["data"]>;
}) {
  const permissions = permissionsData;
  const settings = settingsData;
  const toggleAutostartMutation = useMutation({
    mutationFn: taurpc.toggleAutostart,
  });
  const toggleMenuBarIconMutation = useMutation({
    mutationFn: taurpc.toggleMenuBarIcon,
  });
  const toggleDockIconMutation = useMutation({
    mutationFn: taurpc.toggleDockIcon,
  });
  const toggleHideHintPanelByDefaultMutation = useMutation({
    mutationFn: taurpc.toggleHideHintPanelByDefault,
  });

  return (
    <Card size="sm">
      <CardHeader>
        <CardTitle>Application</CardTitle>
        <CardDescription>
          Control how Chord looks and starts, where its icon appears, and whether the Hint Panel
          is shown by default.
        </CardDescription>
      </CardHeader>
      <CardContent className="pt-0">
        <div className="preference-group">
          <AppearanceRow />
          <SettingSwitchRow
            id="launch-on-login"
            label="Launch on login"
            description="Start Chord automatically when you sign in."
            checked={permissions.isAutostartEnabled === true}
            disabled={toggleAutostartMutation.isPending}
            onCheckedChange={() => {
              toggleAutostartMutation.mutate();
            }}
          />
          <SettingSwitchRow
            id="menu-bar-show-icon"
            label={desktopPlatform === "macos" ? "Show menu bar icon" : "Show tray icon"}
            description="Access settings and reload chords from the Chord icon."
            checked={settings.showMenuBarIcon}
            disabled={toggleMenuBarIconMutation.isPending}
            onCheckedChange={() => {
              toggleMenuBarIconMutation.mutate();
            }}
          />
          {desktopPlatform === "macos" && (
            <SettingSwitchRow
              id="show-dock-icon"
              label="Show Dock icon"
              description="Keep Chord visible in the Dock while it is running."
              checked={settings.showDockIcon}
              disabled={toggleDockIconMutation.isPending}
              onCheckedChange={() => {
                toggleDockIconMutation.mutate();
              }}
            />
          )}
          <SettingSwitchRow
            id="hide-hint-panel-by-default"
            label="Hide Hint Panel by default"
            description="Start chord mode with the Hint Panel hidden. Press Tab to reveal it when needed."
            checked={settings.isChordPanelHiddenByDefault}
            disabled={toggleHideHintPanelByDefaultMutation.isPending}
            onCheckedChange={() => {
              toggleHideHintPanelByDefaultMutation.mutate();
            }}
          />
        </div>
      </CardContent>
    </Card>
  );
}

const APPEARANCES: { value: Appearance; label: string }[] = [
  { value: "system", label: "Auto" },
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
];

/** Light or Dark for the Settings window, or Auto to follow macOS. */
function AppearanceRow() {
  const [appearance, setSelected] = useState(getAppearance);
  return (
    <div className="preference-row">
      <div className="flex min-w-0 flex-1 items-center gap-[9px]">
        <span id="appearance-label" className="preference-label">
          Appearance
        </span>
      </div>
      <ToggleGroup
        type="single"
        size="sm"
        aria-labelledby="appearance-label"
        value={appearance}
        onValueChange={(value) => {
          const next = value as Appearance;
          setSelected(next);
          void setAppearance(next);
        }}
      >
        {APPEARANCES.map(({ value, label }) => (
          <ToggleGroupItem key={value} value={value} className="appearance-option">
            {label}
          </ToggleGroupItem>
        ))}
      </ToggleGroup>
    </div>
  );
}

function SettingSwitchRow({
  id,
  label,
  description,
  checked,
  disabled,
  onCheckedChange,
}: {
  id: string;
  label: string;
  description: string;
  checked: boolean;
  disabled?: boolean;
  onCheckedChange: () => void;
}) {
  return (
    <div className="preference-row">
      {/* Like System Settings, the explanation sits under the label instead of behind a button. */}
      <div className="min-w-0 flex-1">
        <Label htmlFor={id} className="preference-label">
          {label}
        </Label>
        <p id={`${id}-description`} className="preference-description">
          {description}
        </p>
      </div>
      <Switch
        id={id}
        aria-label={label}
        aria-describedby={`${id}-description`}
        checked={checked}
        disabled={disabled}
        onCheckedChange={onCheckedChange}
      />
    </div>
  );
}
