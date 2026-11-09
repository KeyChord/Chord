import { useMutation } from "@chord/com.npmjs.tanstack__react-query";
import { taurpc } from "@chord/dev.improve.chord.api.taurpc";
import { useAppUpdateState } from "@chord/dev.improve.chord.lib.state";
import { UpdateCardContent } from "#update-card-content";

export function UpdateCard({ className }: { className?: string }) {
  const appUpdate = useAppUpdateState();
  const installUpdate = useMutation({ mutationFn: () => taurpc.installUpdate() });
  const restartToUpdate = useMutation({ mutationFn: () => taurpc.restartToUpdate() });

  if (!appUpdate.data || appUpdate.data.status === "idle") {
    return null;
  }

  return (
    <UpdateCardContent
      update={appUpdate.data}
      className={className}
      onInstall={() => {
        installUpdate.mutate();
      }}
      onRestart={() => {
        restartToUpdate.mutate();
      }}
    />
  );
}
