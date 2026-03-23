import { Check } from "lucide-react";
import { Button } from "#/components/ui/button.tsx";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "#/components/ui/card.tsx";
import { useSettingsState } from "../../utils/state.ts";
import { useMutation } from "node_modules/@tanstack/react-query/build/modern/_tsup-dts-rollup";
import { taurpc } from "../../api/taurpc.ts";

export function PermissionsCard() {
  const { permissions } = useSettingsState();
  const openAccessibilitySettingsMutation = useMutation({
    mutationFn: taurpc.openAccessibilitySettings,
  });
  const openInputMonitoringSettingsMutation = useMutation({
    mutationFn: taurpc.openInputMonitoringSettings,
  });

  return (
    <Card size="sm">
      <CardHeader>
        <CardTitle>Permissions</CardTitle>
        <CardDescription>
          Grant macOS access for clicking chords and listening for the global shortcut.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-2 pt-0">
        <div className="flex items-center justify-between gap-3 rounded-lg border bg-background/80 px-3 py-2">
          <div className="min-w-0">
            <p className="truncate font-medium">Accessibility</p>
            <p className="truncate text-xs text-muted-foreground">Needed for automated clicking.</p>
          </div>
          {permissions.isAccessibilityEnabled ? (
            <Button
              type="button"
              variant="ghost"
              size="icon-sm"
              aria-label="Open Accessibility settings"
              title="Open Accessibility settings"
              onClick={() => {
                openAccessibilitySettingsMutation.mutate();
              }}
              disabled={openAccessibilitySettingsMutation.isPending}
            >
              <Check className="text-emerald-600" />
            </Button>
          ) : (
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={() => {
                openAccessibilitySettingsMutation.mutate();
              }}
              disabled={openAccessibilitySettingsMutation.isPending}
            >
              {openAccessibilitySettingsMutation.isPending ? "Requesting..." : "Grant"}
            </Button>
          )}
        </div>

        <div className="flex items-center justify-between gap-3 rounded-lg border bg-background/80 px-3 py-2">
          <div className="min-w-0">
            <p className="truncate font-medium">Input Monitoring</p>
            <p className="truncate text-xs text-muted-foreground">
              Needed for the global shortcut; restart after enabling.
            </p>
          </div>
          {permissions.isInputMonitoringEnabled ? (
            <Button
              type="button"
              variant="ghost"
              size="icon-sm"
              aria-label="Open Input Monitoring settings"
              title="Open Input Monitoring settings"
              onClick={() => {
                openInputMonitoringSettingsMutation.mutate();
              }}
              disabled={openInputMonitoringSettingsMutation.isPending}
            >
              <Check className="text-emerald-600" />
            </Button>
          ) : (
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={() => {
                openInputMonitoringSettingsMutation.mutate();
              }}
              disabled={openInputMonitoringSettingsMutation.isPending}
            >
              {openInputMonitoringSettingsMutation.isPending ? "Opening..." : "Grant"}
            </Button>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
