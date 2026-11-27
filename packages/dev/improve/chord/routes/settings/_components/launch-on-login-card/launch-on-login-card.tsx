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
import {
  getCollapseEarlierKeys,
  setCollapseEarlierKeys,
} from "@chord/dev.improve.chord.lib.state/panel-preferences";
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
  const [collapseEarlierKeys, setCollapseEarlierKeysState] = useState(getCollapseEarlierKeys);
  const toggleHideHintPanelByDefaultMutation = useMutation({
    mutationFn: taurpc.toggleHideHintPanelByDefault,
  });

  return (
    <>
      <Card size="sm">
        <CardHeader>
          <CardTitle>Application</CardTitle>
          <CardDescription>How Chord looks and starts, and where its icon appears.</CardDescription>
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
          </div>
        </CardContent>
      </Card>

      <Card size="sm">
        <CardHeader>
          <CardTitle>Hint Panel</CardTitle>
          <CardDescription>
            The keys you can press next, shown while you type a chord.
          </CardDescription>
        </CardHeader>
        <CardContent className="pt-0">
          <div className="preference-group">
            {/* Says what the panel is, since its name alone doesn't. */}
            <p className="preference-intro">
              While you type a chord, the Hint Panel lists the keys you can press next and what each
              one does.
            </p>
            <SettingSwitchRow
              id="hide-hint-panel-by-default"
              label="Hide until I press Tab"
              description="Start chord mode without the panel. Press Tab to show it when you need a reminder."
              checked={settings.isChordPanelHiddenByDefault}
              disabled={toggleHideHintPanelByDefaultMutation.isPending}
              onCheckedChange={() => {
                toggleHideHintPanelByDefaultMutation.mutate();
              }}
            />
            <SettingSwitchRow
              id="collapse-earlier-keys"
              label="Fold earlier keys of long chords"
              description="When a chord gets long, show the keys you've already typed as a short trail beside the panel."
              checked={collapseEarlierKeys}
              onCheckedChange={() => {
                setCollapseEarlierKeys(!collapseEarlierKeys);
                setCollapseEarlierKeysState(!collapseEarlierKeys);
              }}
            />
          </div>
        </CardContent>
      </Card>
    </>
  );
}

const APPEARANCES: { value: Appearance; label: string }[] = [
  { value: "system", label: "Auto" },
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
];

/**
 * Light or Dark for the Settings window, or Auto to follow macOS, picked from small window
 * previews like System Settings' Appearance.
 */
function AppearanceRow() {
  const [appearance, setSelected] = useState(getAppearance);
  return (
    <div className="preference-row items-start">
      <span id="appearance-label" className="preference-label pt-1">
        Appearance
      </span>
      <div role="radiogroup" aria-labelledby="appearance-label" className="appearance-picker">
        {APPEARANCES.map(({ value, label }) => (
          <button
            key={value}
            type="button"
            role="radio"
            aria-checked={appearance === value}
            className="appearance-choice"
            onClick={() => {
              setSelected(value);
              void setAppearance(value);
            }}
          >
            <span className="appearance-preview" data-appearance={value} aria-hidden="true">
              <span className="appearance-preview-window">
                <span className="appearance-preview-sidebar" />
                <span className="appearance-preview-content" />
              </span>
            </span>
            <span className="appearance-choice-label">{label}</span>
          </button>
        ))}
      </div>
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
