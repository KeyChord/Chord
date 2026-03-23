import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "#/components/ui/card.tsx";
import { Checkbox } from "#/components/ui/checkbox.tsx";
import { Label } from "#/components/ui/label.tsx";
import { useMutation } from "@tanstack/react-query";
import { useSettingsState } from "../../utils/state.ts";
import { taurpc } from "../../api/taurpc.ts";

export function LaunchOnLoginCard() {
  const { permissions } = useSettingsState();
  const setAutostartMutation = useMutation({
    // TODO
    mutationFn: taurpc.getStartupStatus,
  });

  return (
    <Card size="sm">
      <CardHeader>
        <CardTitle>Launch on Login</CardTitle>
        <CardDescription>{permissions.isAutostartEnabled}</CardDescription>
      </CardHeader>
      <CardContent className="space-y-3 pt-0">
        <div className="flex items-start gap-3">
          <Checkbox
            id="launch-on-login"
            checked={permissions.isAutostartEnabled}
            disabled={setAutostartMutation.isPending}
            onCheckedChange={(checked) => {
              void setAutostartMutation.mutate();
            }}
          />
          <div className="space-y-1">
            <Label htmlFor="launch-on-login">Launch Chords on login</Label>
            <p className="text-sm text-muted-foreground">
              The app stays in the tray, reuses a single instance, and launches hidden on login.
            </p>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
