import { useMutation, useQuery } from "@chord/com.npmjs.tanstack__react-query";
import { taurpc } from "@chord/dev.improve.chord.api.taurpc";
import { Button } from "@chord/dev.improve.chord.components.ui.button";
import {
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@chord/dev.improve.chord.components.ui.dialog";
import { StateQueries, usePermissionsState } from "@chord/dev.improve.chord.lib.state";
import { MousePointer2 } from "lucide-react";
import { useEffect } from "react";

export function PermissionsDialog({ onDismiss }: { onDismiss: () => void }) {
  return (
    <StateQueries queries={[usePermissionsState()]} quiet>
      {(permissionsData) => (
        <PermissionsDialogContent onDismiss={onDismiss} permissionsData={permissionsData} />
      )}
    </StateQueries>
  );
}

function PermissionsDialogContent({
  onDismiss,
  permissionsData,
}: {
  onDismiss: () => void;
  permissionsData: NonNullable<ReturnType<typeof usePermissionsState>["data"]>;
}) {
  const { data: isAccessibilityEnabled, refetch: refetchPermissions } = useQuery({
    queryKey: ["macos-permissions"],
    queryFn: () => taurpc.refreshPermissions(),
    initialData: permissionsData.isAccessibilityEnabled ?? false,
    refetchOnWindowFocus: "always",
  });
  useEffect(() => {
    const handleFocus = () => {
      void refetchPermissions();
    };

    window.addEventListener("focus", handleFocus);
    return () => {
      window.removeEventListener("focus", handleFocus);
    };
  }, [refetchPermissions]);
  const openAccessibilitySettingsMutation = useMutation({
    mutationFn: taurpc.openAccessibilitySettings,
  });

  return (
    <DialogContent
      isOpen={!isAccessibilityEnabled}
      onOpenChange={(open) => {
        if (!open && !isAccessibilityEnabled) {
          onDismiss();
        }
      }}
      showCloseButton={false}
      className="gap-0 overflow-hidden p-0 shadow-2xl sm:max-w-[430px]"
    >
      <div className="flex gap-4 p-5">
        <div className="flex size-12 shrink-0 items-center justify-center rounded-xl bg-primary/10 text-primary">
          <MousePointer2 className="size-6" strokeWidth={1.75} />
        </div>
        <DialogHeader className="gap-2 pt-0.5">
          <DialogTitle className="text-[17px] leading-5">
            Chord needs Accessibility access
          </DialogTitle>
          <DialogDescription className="leading-5">
            Accessibility lets Chord detect your global trigger and run actions in other apps. macOS
            requires you to approve this in System Settings.
          </DialogDescription>
        </DialogHeader>
      </div>

      <DialogFooter className="m-0 rounded-none px-4 py-3">
        <Button type="button" variant="ghost" onClick={onDismiss}>
          Not Now
        </Button>
        <Button
          type="button"
          onClick={() => {
            openAccessibilitySettingsMutation.mutate();
          }}
          disabled={openAccessibilitySettingsMutation.isPending}
        >
          {openAccessibilitySettingsMutation.isPending ? "Opening…" : "Open System Settings"}
        </Button>
      </DialogFooter>
    </DialogContent>
  );
}
