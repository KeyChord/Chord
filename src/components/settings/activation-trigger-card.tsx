import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "#/components/ui/card.tsx";
import { ShortcutKeys } from "#/components/settings/shortcut-keys.tsx";

export function ActivationTriggerCard() {
  return (
    <Card size="sm">
      <CardHeader>
        <CardTitle>Activation Trigger</CardTitle>
        <CardDescription>
          Use this keyboard trigger to open the chord overlay anywhere in macOS.
        </CardDescription>
      </CardHeader>
      <CardContent className="pt-0">
        <div className="flex items-center justify-between gap-3 rounded-lg border bg-background/80 px-3 py-3">
          <div className="min-w-0">
            <p className="font-medium">Open Chord Overlay</p>
            <p className="text-xs text-muted-foreground">
              This shortcut is fixed for now and shown here for reference.
            </p>
          </div>
          <div className="shrink-0 rounded-lg border bg-muted/50 px-2 py-1">
            <ShortcutKeys shortcut="capsLock+space" />
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
