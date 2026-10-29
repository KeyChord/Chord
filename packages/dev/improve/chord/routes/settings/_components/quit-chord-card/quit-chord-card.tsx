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

export function QuitChordCard() {
  const quitAppMutation = useMutation({
    mutationFn: taurpc.quitApp,
  });

  return (
    <Card size="sm">
      <CardHeader>
        <CardTitle>App Control</CardTitle>
        <CardDescription>
          Exit Chord immediately. You can still relaunch it later from Applications or your login
          items.
        </CardDescription>
      </CardHeader>
      <CardContent className="preference-group">
        <div className="preference-row">
          <div className="min-w-0 flex-1">
            <p className="preference-label">Quit Chord</p>
            <p className="preference-description">Stop Chord and its background actions.</p>
          </div>
          <Button
            variant="outline"
            onClick={() => quitAppMutation.mutate()}
            disabled={quitAppMutation.isPending}
          >
            {quitAppMutation.isPending ? "Quitting…" : "Quit Chord"}
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}
