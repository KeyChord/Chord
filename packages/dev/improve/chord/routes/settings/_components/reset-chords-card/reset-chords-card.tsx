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
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@chord/dev.improve.chord.components.ui.card";

export function ResetChordsCard() {
  const resetDefaultChordsMutation = useMutation({
    mutationFn: taurpc.resetDefaultChords,
    onSuccess: () => {
      toast.success("Reset managed chord repos to the default packages.");
    },
  });

  return (
    <Card size="sm">
      <CardHeader>
        <CardTitle>Reset Chords</CardTitle>
        <CardDescription>
          Replaces the managed git repo set with the bundled default packages. Local chord folders
          stay configured.
        </CardDescription>
      </CardHeader>
      <CardContent className="pt-0">
        <p className="text-sm text-muted-foreground">
          Use this to get back to the pinned default repos from{" "}
          <code>data/default-packages.toml</code>.
        </p>
      </CardContent>
      <CardFooter className="justify-end">
        <AlertDialog>
          <Button variant="destructive" disabled={resetDefaultChordsMutation.isPending}>
            {resetDefaultChordsMutation.isPending ? "Resetting..." : "Reset Chords"}
          </Button>

          <AlertDialogContent size="sm">
            <AlertDialogHeader>
              <AlertDialogTitle>Reset managed chord repos?</AlertDialogTitle>
              <AlertDialogDescription>
                This replaces the active managed repo set with the pinned default packages. Cached
                revisions are retained for reuse.
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
                Reset Chords
              </AlertDialogAction>
            </AlertDialogFooter>
          </AlertDialogContent>
        </AlertDialog>
      </CardFooter>
    </Card>
  );
}
