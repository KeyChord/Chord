import { useMutation } from "@tanstack/react-query";
import { taurpc } from "../../api/taurpc.ts";
import { Button } from "#/components/ui/button.tsx";

export function RelaunchAppButton({ app }: { app: { bundleId: string } }) {
  const relaunchAppMutation = useMutation({
    mutationFn: taurpc.relaunchApp
  });

return <Button
                    type="button"
                    variant="outline"
                    size="sm"
                    onClick={() => {
                      relaunchAppMutation.mutate(app.bundleId)
                    }}
                    disabled={relaunchAppMutation.isPending}
                  >
                    {relaunchAppMutation.isPending ? "Relaunching..." : "Relaunch"}
                  </Button>
}