import { toast } from "@chord/dev.improve.chord.components.ui.toast";
import { useMutation } from "@chord/com.npmjs.tanstack__react-query";
import { taurpc } from "@chord/dev.improve.chord.api.taurpc";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@chord/dev.improve.chord.components.ui.alert-dialog";
import { Button } from "@chord/dev.improve.chord.components.ui.button";
import { GroupCard } from "@chord/dev.improve.chord.components.ui.group-card";
import { RotateCcw } from "lucide-react";

export function ResetChordsCard() {
  const resetDefaultChordsMutation = useMutation({
    mutationFn: taurpc.resetDefaultChords,
    onSuccess: () => {
      toast.success("Reset your packages to the defaults.");
    },
  });

  // One card, like the rest of Settings: what the reset does, and the button that does it.
  return (
    <GroupCard
      icon={<RotateCcw size={17} strokeWidth={1.6} />}
      title="Reset packages to defaults"
      subtitle="Replaces your installed packages with Chord's default set. Your local chords stay."
      actions={
        <AlertDialog>
          <Button
            variant="outline"
            className="mac-button"
            data-destructive
            disabled={resetDefaultChordsMutation.isPending}
          >
            {resetDefaultChordsMutation.isPending ? "Resetting…" : "Reset…"}
          </Button>

          <AlertDialogContent size="sm">
            <AlertDialogHeader>
              <AlertDialogTitle>Reset packages to defaults?</AlertDialogTitle>
              <AlertDialogDescription>
                Chord replaces your installed packages with its default set. Your local chords
                aren&apos;t changed, and downloaded versions are kept for reuse.
              </AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel disabled={resetDefaultChordsMutation.isPending}>
                Cancel
              </AlertDialogCancel>
              <AlertDialogAction
                variant="destructive"
                disabled={resetDefaultChordsMutation.isPending}
                onClick={() => {
                  resetDefaultChordsMutation.mutate();
                }}
              >
                Reset
              </AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>
      }
    />
  );
}
