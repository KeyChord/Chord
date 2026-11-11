import { useMutation } from "@chord/com.npmjs.tanstack__react-query";
import { taurpc } from "@chord/dev.improve.chord.api.taurpc";
import { Badge } from "@chord/dev.improve.chord.components.ui.badge";
import { Button } from "@chord/dev.improve.chord.components.ui.button";
import { toast } from "@chord/dev.improve.chord.components.ui.toast";
import {
  type GitRepo,
  type PackageUpdate,
  type PackageUpdateCheck,
  PackageUpdateSource,
} from "@chord/dev.improve.chord.lib.typeshare";
import { ArrowRight, Download, RefreshCw } from "lucide-react";
import { useEffect, useState } from "react";

const relativeTime = new Intl.RelativeTimeFormat("en", { numeric: "auto" });
const units = [
  ["day", 24 * 60 * 60 * 1000],
  ["hour", 60 * 60 * 1000],
  ["minute", 60 * 1000],
] as const;

/** "just now", "5 minutes ago", "yesterday". */
export function formatCheckedAt(checkedAt: number, now: number) {
  const elapsed = Math.max(0, now - checkedAt);
  for (const [unit, length] of units) {
    if (elapsed >= length) {
      return relativeTime.format(-Math.floor(elapsed / length), unit);
    }
  }
  return "just now";
}

/** The time, updated every minute so labels like "5 minutes ago" stay true. */
function useMinuteClock() {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const timer = setInterval(() => {
      setNow(Date.now());
    }, 60 * 1000);
    return () => {
      clearInterval(timer);
    };
  }, []);
  return now;
}

/**
 * Packages with a newer commit than the installed one. Chord never installs these by itself:
 * a built-in package moves to the commit this version of Chord ships with, and any other package
 * to the newest commit in its repository, only when someone presses Update.
 */
export function PackageUpdates({
  repos,
  updates,
  updateCheck,
  displayNames,
  now: fixedNow,
}: {
  repos: Record<string, GitRepo>;
  updates: Record<string, PackageUpdate>;
  updateCheck: PackageUpdateCheck;
  /** Catalog names by lowercase repository slug. */
  displayNames: Map<string, string>;
  /** For tests: the time to measure the last check from. */
  now?: number;
}) {
  const clock = useMinuteClock();
  const now = fixedNow ?? clock;
  const checkMutation = useMutation({
    mutationFn: () => taurpc.checkPackageUpdates(),
  });
  const updateMutation = useMutation({
    mutationFn: (slugs: string[]) => taurpc.updatePackages(slugs),
    onSuccess: (_, slugs) => {
      toast.success(
        slugs.length === 1
          ? `Updated ${displayName(slugs[0]!)}.`
          : `Updated ${slugs.length} packages.`,
      );
    },
  });

  function displayName(slug: string) {
    return displayNames.get(slug.toLowerCase()) ?? repos[slug]?.name ?? slug;
  }

  const pending = Object.values(updates)
    .filter((update) => repos[update.slug])
    .sort((a, b) => displayName(a.slug).localeCompare(displayName(b.slug), "en"));
  const checking = updateCheck.checking || checkMutation.isPending;
  const busy = updateMutation.isPending;
  const status = checking
    ? "Checking for updates..."
    : updateCheck.checkedAt != null
      ? `Checked ${formatCheckedAt(updateCheck.checkedAt, now)}`
      : "Not checked yet";

  return (
    <section aria-labelledby="package-updates" className="space-y-2">
      <div className="flex items-center justify-between gap-3">
        <div className="space-y-0.5">
          <h3 id="package-updates" className="font-medium">
            Updates
          </h3>
          <p className="text-xs text-muted-foreground">
            {pending.length > 0
              ? `${pending.length} package${pending.length === 1 ? " has" : "s have"} an update. `
              : checking || updateCheck.checkedAt == null
                ? ""
                : "Every package is up to date. "}
            {status}
          </p>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={() => {
              checkMutation.mutate();
            }}
            disabled={checking || busy}
          >
            <RefreshCw className={checking ? "animate-spin" : undefined} />
            Check Again
          </Button>
          {pending.length > 1 ? (
            <Button
              type="button"
              size="sm"
              onClick={() => {
                updateMutation.mutate(pending.map((update) => update.slug));
              }}
              disabled={busy}
            >
              <Download />
              {busy && (updateMutation.variables?.length ?? 0) > 1 ? "Updating..." : "Update All"}
            </Button>
          ) : null}
        </div>
      </div>

      {updateCheck.failed.length > 0 && !checking ? (
        <p className="text-xs text-destructive">
          Couldn't reach {updateCheck.failed.map(displayName).join(", ")}. Check your connection and
          try again.
        </p>
      ) : null}

      {pending.length > 0 ? (
        <ul className="divide-y rounded-lg border bg-default/30">
          {pending.map((update) => {
            const isUpdating =
              busy &&
              updateMutation.variables?.length === 1 &&
              updateMutation.variables[0] === update.slug;
            return (
              <li key={update.slug} className="flex items-center justify-between gap-3 px-4 py-2.5">
                <div className="min-w-0 space-y-0.5">
                  <div className="flex flex-wrap items-center gap-x-2 gap-y-1">
                    <span className="font-medium">{displayName(update.slug)}</span>
                    <Badge variant="outline">
                      {update.source === PackageUpdateSource.BuiltIn ? "Built in" : "Latest"}
                    </Badge>
                  </div>
                  <p className="flex items-center gap-1 font-mono text-xs text-muted-foreground">
                    <span>{update.installedRev.slice(0, 7)}</span>
                    <ArrowRight aria-label="to" className="size-3" />
                    <span>{update.rev.slice(0, 7)}</span>
                  </p>
                </div>
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={() => {
                    updateMutation.mutate([update.slug]);
                  }}
                  disabled={busy}
                >
                  {isUpdating ? "Updating..." : "Update"}
                </Button>
              </li>
            );
          })}
        </ul>
      ) : null}
    </section>
  );
}
