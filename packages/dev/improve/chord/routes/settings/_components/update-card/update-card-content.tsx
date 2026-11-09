import { cn } from "@chord/com.npmjs.utils-cn";
import { Button } from "@chord/dev.improve.chord.components.ui.button";
import type { AppUpdateState } from "@chord/dev.improve.chord.lib.typeshare";
import { ProgressBar } from "@heroui/react/progress-bar";

export type OfferedUpdate = Exclude<AppUpdateState, { status: "idle" }>;

const TITLES: Record<OfferedUpdate["status"], string> = {
  available: "Update available",
  downloading: "Downloading update…",
  installing: "Installing update…",
  ready: "Update installed",
};

export function UpdateCardContent({
  update,
  onInstall,
  onRestart,
  className,
}: {
  update: OfferedUpdate;
  onInstall: () => void;
  onRestart: () => void;
  className?: string;
}) {
  const title = TITLES[update.status];
  // Rust sends null until the download reports its size.
  const percent =
    update.status === "downloading" ? (update.update.percent ?? undefined) : undefined;
  const showsProgress = update.status === "downloading" || update.status === "installing";

  return (
    <section
      aria-label="Chord update"
      className={cn("rounded-xl border border-border bg-surface p-3 shadow-sm", className)}
    >
      <div className="flex items-center justify-between gap-2">
        <div className="min-w-0">
          <p className="truncate text-[13px] font-medium leading-4 text-foreground">{title}</p>
          <p className="truncate text-xs leading-4 text-muted">Chord {update.update.version}</p>
        </div>
        {update.status === "available" ? (
          <Button type="button" size="sm" onClick={onInstall}>
            Update
          </Button>
        ) : null}
        {update.status === "ready" ? (
          <Button type="button" size="sm" onClick={onRestart}>
            Restart
          </Button>
        ) : null}
        {percent === undefined ? null : (
          <span className="shrink-0 text-xs tabular-nums text-muted">{percent}%</span>
        )}
      </div>
      {showsProgress ? (
        <ProgressBar
          aria-label={title}
          className="mt-2"
          size="sm"
          value={percent}
          isIndeterminate={percent === undefined}
        >
          <ProgressBar.Track>
            <ProgressBar.Fill />
          </ProgressBar.Track>
        </ProgressBar>
      ) : null}
    </section>
  );
}
