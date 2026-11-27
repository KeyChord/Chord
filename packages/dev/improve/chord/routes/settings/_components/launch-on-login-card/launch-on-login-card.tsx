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
import { Select } from "@chord/dev.improve.chord.components.ui.select";
import {
  StateQueries,
  desktopPlatform,
  useChordPanelState,
  usePermissionsState,
  useSettingsState,
} from "@chord/dev.improve.chord.lib.state";
import { ChordLayout } from "@chord/dev.improve.chord.lib.typeshare";
import {
  type Appearance,
  getAppearance,
  setAppearance,
} from "@chord/dev.improve.chord.lib.state/appearance";
import {
  getCollapseEarlierKeys,
  getPanelKeyFont,
  getPanelTextFont,
  PANEL_KEY_FONTS,
  PANEL_TEXT_FONTS,
  setCollapseEarlierKeys,
  setPanelKeyFont,
  setPanelTextFont,
  type PanelKeyFont,
  type PanelTextFont,
} from "@chord/dev.improve.chord.lib.state/panel-preferences";
import { useState, type CSSProperties } from "react";

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
            <HintPanelStyleRow />
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
            <HintPanelFontRows />
          </div>
        </CardContent>
      </Card>
    </>
  );
}

const textFontOptions = Object.entries(PANEL_TEXT_FONTS).map(([value, { label }]) => ({
  value,
  label,
}));
const keyFontOptions = Object.entries(PANEL_KEY_FONTS).map(([value, { label }]) => ({
  value,
  label,
}));

function HintPanelFontRows() {
  const [textFont, setTextFont] = useState(getPanelTextFont);
  const [keyFont, setKeyFont] = useState(getPanelKeyFont);

  return (
    <>
      <div className="preference-row">
        <Label htmlFor="hint-panel-text-font" className="preference-label">
          Text font
        </Label>
        <Select
          id="hint-panel-text-font"
          value={textFont}
          options={textFontOptions}
          onValueChange={(value) => {
            const font = value as PanelTextFont;
            setPanelTextFont(font);
            setTextFont(font);
          }}
        />
      </div>
      <div className="preference-row">
        <Label htmlFor="hint-panel-key-font" className="preference-label">
          Keycap font
        </Label>
        <Select
          id="hint-panel-key-font"
          value={keyFont}
          options={keyFontOptions}
          onValueChange={(value) => {
            const font = value as PanelKeyFont;
            setPanelKeyFont(font);
            setKeyFont(font);
          }}
        />
      </div>
    </>
  );
}

const APPEARANCES: { value: Appearance; label: string }[] = [
  { value: "system", label: "Auto" },
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
];

const HINT_PANEL_STYLES: { value: ChordLayout; label: string }[] = [
  { value: ChordLayout.SidebarLeft, label: "Sidebar" },
  { value: ChordLayout.FloatingBottom, label: "Overlay" },
];

/**
 * Sidebar, which lists the keys you can press next beside the latest one, or the original
 * overlay along the bottom, which shows everything you've typed from left to right.
 */
function HintPanelStyleRow() {
  const layout = useChordPanelState().data?.layout ?? ChordLayout.SidebarLeft;
  const setLayout = useMutation({ mutationFn: taurpc.setHintPanelLayout });
  const shown = setLayout.isPending && setLayout.variables ? setLayout.variables : layout;
  const index = HINT_PANEL_STYLES.findIndex(({ value }) => value === shown);
  return (
    <div className="preference-row">
      <div className="min-w-0 flex-1">
        <span id="hint-panel-style-label" className="preference-label">
          Style
        </span>
        <p className="preference-description">
          {shown === ChordLayout.FloatingBottom
            ? "Everything you type, left to right, in an overlay along the bottom of the screen."
            : "The keys you can press next, in a sidebar beside the last key you typed."}
        </p>
      </div>
      <div
        role="radiogroup"
        aria-labelledby="hint-panel-style-label"
        className="mac-segmented appearance-segmented"
        style={{ "--count": HINT_PANEL_STYLES.length, "--index": index } as CSSProperties}
      >
        {HINT_PANEL_STYLES.map(({ value, label }) => (
          <button
            key={value}
            type="button"
            role="radio"
            aria-checked={shown === value}
            className="mac-segment"
            disabled={setLayout.isPending}
            onClick={() => setLayout.mutate(value)}
          >
            {label}
          </button>
        ))}
      </div>
    </div>
  );
}

/** Light or Dark for Settings and the Hint Panel, or Auto to follow macOS, as a segmented control. */
function AppearanceRow() {
  const [appearance, setSelected] = useState(getAppearance);
  const index = APPEARANCES.findIndex(({ value }) => value === appearance);
  return (
    <div className="preference-row">
      <span id="appearance-label" className="preference-label">
        Appearance
      </span>
      <div
        role="radiogroup"
        aria-labelledby="appearance-label"
        className="mac-segmented appearance-segmented"
        style={{ "--count": APPEARANCES.length, "--index": index } as CSSProperties}
      >
        {APPEARANCES.map(({ value, label }) => (
          <button
            key={value}
            type="button"
            role="radio"
            aria-checked={appearance === value}
            className="mac-segment"
            onClick={() => {
              setSelected(value);
              void setAppearance(value);
            }}
          >
            {label}
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
