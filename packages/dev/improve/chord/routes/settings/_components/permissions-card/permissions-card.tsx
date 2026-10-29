import { useMutation } from "@chord/com.npmjs.tanstack__react-query";
import { taurpc } from "@chord/dev.improve.chord.api.taurpc";
import { Button } from "@chord/dev.improve.chord.components.ui.button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@chord/dev.improve.chord.components.ui.card";
import { StateQueries, usePermissionsState } from "@chord/dev.improve.chord.lib.state";
import { Check } from "lucide-react";

export function PermissionsCard() {
  return (
    <StateQueries queries={[usePermissionsState()]}>
      {(permissionsData) => <PermissionsCardContent permissionsData={permissionsData} />}
    </StateQueries>
  );
}

function PermissionsCardContent({
  permissionsData,
}: {
  permissionsData: NonNullable<ReturnType<typeof usePermissionsState>["data"]>;
}) {
  const permissions = permissionsData;
  const openAccessibilitySettingsMutation = useMutation({
    mutationFn: taurpc.openAccessibilitySettings,
  });

  return (
    <Card size="sm">
      <CardHeader>
        <CardTitle>Permissions</CardTitle>
        <CardDescription>
          Grant macOS access for the global trigger and for clicking chords.
        </CardDescription>
      </CardHeader>
      <CardContent className="preference-group">
        <div className="preference-row">
          <div className="min-w-0">
            <p className="preference-label">Accessibility</p>
            <p className="preference-description">
              Needed for the global trigger and automated clicking.
            </p>
          </div>
          {permissions.isAccessibilityEnabled ? (
            <Button
              type="button"
              variant="ghost"
              size="sm"
              aria-label="Open Accessibility settings"
              title="Open Accessibility settings"
              onClick={() => {
                openAccessibilitySettingsMutation.mutate();
              }}
              disabled={openAccessibilitySettingsMutation.isPending}
            >
              <Check className="size-4 text-foreground" />
              <span className="text-muted-foreground">Enabled</span>
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
      </CardContent>
    </Card>
  );
}
