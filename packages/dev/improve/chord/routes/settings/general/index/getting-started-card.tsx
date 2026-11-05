import { useMutation } from "@chord/com.npmjs.tanstack__react-query";
import { useNavigate } from "@chord/com.npmjs.tanstack__react-router";
import { taurpc } from "@chord/dev.improve.chord.api.taurpc";
import { Button } from "@chord/dev.improve.chord.components.ui.button";
import { Card, CardContent, CardHeader, CardTitle } from "@chord/dev.improve.chord.components.ui.card";

export function GettingStartedCard() {
  const navigate = useNavigate();
  const restart = useMutation({
    mutationFn: () => taurpc.setOnboardingComplete(false),
    onSuccess: () => void navigate({ to: "/onboarding" }),
  });
  return (
    <Card size="sm">
      <CardHeader>
        <CardTitle>Getting Started</CardTitle>
      </CardHeader>
      <CardContent className="preference-group">
        <div className="preference-row">
          <div className="min-w-0">
            <p className="preference-label">Setup guide</p>
            <p className="preference-description">
              Walk through setup again, make a chord, and practice using it.
            </p>
          </div>
          <Button type="button" variant="outline" size="sm" onClick={() => restart.mutate()}>
            Open Guide
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}
