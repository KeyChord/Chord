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
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@chord/dev.improve.chord.components.ui.popover";
import {
  StateQueries,
  usePermissionsState,
  useSettingsState,
} from "@chord/dev.improve.chord.lib.state";
import { Info } from "lucide-react";

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
  const toggleHideGuideByDefaultMutation = useMutation({
    mutationFn: taurpc.toggleHideGuideByDefault,
  });

  return (
    <Card size="sm">
      <CardHeader>
        <CardTitle>Application</CardTitle>
        <CardDescription>
          Control how Chord starts, where its icon appears, and whether the guide is shown by
          default.
        </CardDescription>
      </CardHeader>
      <CardContent className="pt-0">
        <div className="preference-group">
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
            label="Show menu bar icon"
            description="Access settings and reload chords from the menu bar."
            checked={settings.showMenuBarIcon}
            disabled={toggleMenuBarIconMutation.isPending}
            onCheckedChange={() => {
              toggleMenuBarIconMutation.mutate();
            }}
          />
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
          <SettingSwitchRow
            id="hide-guide-by-default"
            label="Hide guide by default"
            description="Start chord mode with the guide hidden. Press Shift+Tab to reveal it when needed."
            checked={settings.isChordPanelHiddenByDefault}
            disabled={toggleHideGuideByDefaultMutation.isPending}
            onCheckedChange={() => {
              toggleHideGuideByDefaultMutation.mutate();
            }}
          />
        </div>
      </CardContent>
    </Card>
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
      <div className="flex min-w-0 flex-1 items-center gap-[9px]">
        <Label htmlFor={id} className="preference-label">
          {label}
        </Label>
        <Popover>
          <PopoverTrigger>
            <button
              type="button"
              className="preference-info"
              aria-label={`About ${label.toLowerCase()}`}
            >
              <Info size={12} strokeWidth={1.8} aria-hidden="true" />
            </button>
          </PopoverTrigger>
          <PopoverContent align="start" sideOffset={8} className="preference-help">
            {description}
          </PopoverContent>
        </Popover>
        <span id={`${id}-description`} className="sr-only">
          {description}
        </span>
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
