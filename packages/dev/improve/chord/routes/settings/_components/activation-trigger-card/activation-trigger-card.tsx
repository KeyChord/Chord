import { useMutation, useQuery } from "@chord/com.npmjs.tanstack__react-query";
import { taurpc } from "@chord/dev.improve.chord.api.taurpc";
import { Button } from "@chord/dev.improve.chord.components.ui.button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@chord/dev.improve.chord.components.ui.card";
import { ShortcutKeys } from "@chord/dev.improve.chord.routes.settings._components.shortcut-keys";
import { TriangleAlert } from "lucide-react";

export function ActivationTriggerCard() {
  return (
    <Card size="sm">
      <CardHeader>
        <CardTitle>Keyboard Shortcuts</CardTitle>
        <CardDescription>
          Start a chord from anywhere on your Mac.
        </CardDescription>
      </CardHeader>
      <CardContent className="preference-group">
        <div className="preference-row">
          <div className="min-w-0">
            <p className="preference-label">Start a chord</p>
            <p className="preference-description">
              Press caps lock and space together, and keep holding space while you type.
            </p>
          </div>
          <div className="shrink-0">
            <ShortcutKeys shortcut="capsLock+space" />
          </div>
        </div>
        <VoiceOverWarning />
      </CardContent>
    </Card>
  );
}

/** VoiceOver can't use the Caps Lock key as its modifier while Chord owns the key. */
function VoiceOverWarning() {
  const { data: usesCapsLock } = useQuery({
    queryKey: ["voice-over-uses-caps-lock"],
    queryFn: () => taurpc.voiceOverUsesCapsLock(),
    // VoiceOver doesn't announce setting changes; check again when the user returns.
    refetchOnWindowFocus: "always",
  });
  const openUtilityMutation = useMutation({
    mutationFn: () => taurpc.openVoiceOverUtility(),
  });

  if (usesCapsLock !== true) {
    return null;
  }

  return (
    <div className="flex items-center justify-between gap-3 rounded-lg border border-foreground/20 bg-foreground/5 px-3 py-2">
      <div className="flex min-w-0 gap-2">
        <TriangleAlert className="mt-0.5 size-4 shrink-0 text-foreground" />
        <div className="min-w-0">
          <p className="font-medium">VoiceOver can't use Caps Lock</p>
          <p className="text-xs text-muted-foreground">
            Chord takes over the Caps Lock key, so it no longer works as the VoiceOver modifier. In
            VoiceOver Utility, set the VoiceOver modifier to Control-Option.
          </p>
        </div>
      </div>
      <Button
        type="button"
        variant="outline"
        size="sm"
        className="shrink-0"
        onClick={() => {
          openUtilityMutation.mutate();
        }}
        disabled={openUtilityMutation.isPending}
      >
        {openUtilityMutation.isPending ? "Opening…" : "Open VoiceOver Utility"}
      </Button>
    </div>
  );
}
