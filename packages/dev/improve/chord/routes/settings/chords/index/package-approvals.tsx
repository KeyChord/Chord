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
import { Checkbox } from "@chord/dev.improve.chord.components.ui.checkbox";
import { Input } from "@chord/dev.improve.chord.components.ui.input";
import type { ThirdPartyPackage } from "@chord/dev.improve.chord.lib.typeshare";
import { ChevronRight, Search, Shield, ShieldAlert, X } from "lucide-react";
import { useId, useState } from "react";

const PACKAGE_CODE_WARNING =
  "Chord scripts and native add-ons can run arbitrary code with your account permissions. Only approve packages you trust or whose code you have reviewed.";

function ApproveDialog({
  packages,
  disabled,
  label,
  ariaLabel,
  onApprove,
}: {
  packages: ThirdPartyPackage[];
  disabled: boolean;
  label: string;
  ariaLabel?: string;
  onApprove: () => void;
}) {
  const single = packages.length === 1 ? packages[0] : undefined;
  return (
    <AlertDialog>
      <Button size="sm" variant="outline" disabled={disabled} aria-label={ariaLabel}>
        {label}
      </Button>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>
            Approve {single ? single.name : `${packages.length} packages`}?
          </AlertDialogTitle>
          <AlertDialogDescription>{PACKAGE_CODE_WARNING}</AlertDialogDescription>
        </AlertDialogHeader>
        <ul className="my-3 max-h-60 space-y-2 overflow-y-auto">
          {packages.map((pkg) => (
            <li key={pkg.identity}>
              {!single && <span className="block wrap-break-word text-sm">{pkg.name}</span>}
              <code className="block wrap-anywhere text-xs text-muted-foreground">{pkg.path}</code>
            </li>
          ))}
        </ul>
        <p className="text-sm text-muted-foreground">
          Approval enables {single ? "this package" : "these packages"} immediately and stays in
          effect until you revoke it.
        </p>
        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <AlertDialogAction disabled={disabled} onClick={onApprove}>
            Approve and Enable
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}

export function PackageApprovals({
  packages,
  errors,
}: {
  packages: ThirdPartyPackage[];
  errors: Record<string, string[]>;
}) {
  const approvedId = useId();
  const [showApproved, setShowApproved] = useState(false);
  const [search, setSearch] = useState("");
  const [selection, setSelection] = useState<ReadonlySet<string>>(() => new Set());
  const {
    mutate: setApprovals,
    isPending,
    error,
  } = useMutation({
    mutationFn: ({ targets, approved }: { targets: ThirdPartyPackage[]; approved: boolean }) =>
      taurpc.setPackageApprovals(
        targets.map((pkg) => pkg.identity),
        approved,
      ),
    onSuccess: (_, { targets }) => updateSelection(targets, false),
  });

  const sorted = packages.toSorted((a, b) => a.name.localeCompare(b.name));
  const pending = sorted.filter((pkg) => !pkg.approved);
  const approved = sorted.filter((pkg) => pkg.approved);
  const failedCount = approved.filter((pkg) => errors[pkg.name]?.length).length;
  const query = search.trim().toLocaleLowerCase();
  function matches(pkg: ThirdPartyPackage) {
    return [pkg.name, pkg.path].some((value) => value.toLocaleLowerCase().includes(query));
  }
  const visiblePending = pending.filter(matches);
  const visibleApproved = showApproved ? approved.filter(matches) : [];
  const visible = [...visiblePending, ...visibleApproved];
  // Search and disclosures can hide selected packages. Bulk actions only affect visible selections.
  const selected = visible.filter((pkg) => selection.has(pkg.identity));
  const selectedPending = selected.filter((pkg) => !pkg.approved);
  const selectedApproved = selected.filter((pkg) => pkg.approved);
  const allSelected = visible.length > 0 && selected.length === visible.length;
  const canSelect = visible.length > 1 || selected.length > 0;
  const showSearch = search !== "" || pending.length + (showApproved ? approved.length : 0) > 4;
  const errorMessage =
    error &&
    (typeof error === "object" && "Message" in error ? String(error.Message) : String(error));

  function updateSelection(targets: ThirdPartyPackage[], checked: boolean) {
    setSelection((previous) => {
      const next = new Set(previous);
      for (const pkg of targets) {
        if (checked) next.add(pkg.identity);
        else next.delete(pkg.identity);
      }
      return next;
    });
  }

  function renderPackage(pkg: ThirdPartyPackage) {
    const displayPath = pkg.path.replace(
      /^.*?[\\/]\.chord[\\/]packages[\\/]/,
      "~/.chord/packages/",
    );
    return (
      <li
        key={pkg.identity}
        className="flex flex-wrap items-center gap-x-3 gap-y-2 border-t border-(--settings-separator) px-4 py-2.5"
      >
        {canSelect && (
          <Checkbox
            aria-label={`Select ${pkg.name}`}
            checked={selection.has(pkg.identity)}
            onCheckedChange={(checked) => updateSelection([pkg], checked)}
          />
        )}
        <div className="min-w-24 flex-1">
          <p className="wrap-anywhere text-[13px] font-medium">{pkg.name}</p>
          <p title={pkg.path} className="truncate text-xs text-(--settings-description)">
            {displayPath}
          </p>
          {pkg.approved &&
            errors[pkg.name]?.map((message) => (
              <p
                key={message}
                role="alert"
                className="mt-1 wrap-anywhere whitespace-pre-wrap text-xs text-destructive"
              >
                {message}
              </p>
            ))}
        </div>
        {pkg.approved ? (
          <Button
            size="sm"
            variant="ghost"
            disabled={isPending}
            aria-label={`Revoke approval for ${pkg.name}`}
            onClick={() => setApprovals({ targets: [pkg], approved: false })}
          >
            Revoke
          </Button>
        ) : (
          <ApproveDialog
            packages={[pkg]}
            disabled={isPending}
            label="Review…"
            ariaLabel={`Review ${pkg.name} for approval`}
            onApprove={() => setApprovals({ targets: [pkg], approved: true })}
          />
        )}
      </li>
    );
  }

  return (
    <section aria-label="Third-party packages" className="preference-disclosure">
      <div className="flex items-start gap-3 px-4 py-3">
        <span
          className={
            pending.length
              ? "flex size-8 shrink-0 items-center justify-center rounded-lg bg-amber-100 text-amber-800 dark:bg-amber-400/15 dark:text-amber-200"
              : "flex size-8 shrink-0 items-center justify-center rounded-lg bg-(--settings-key) text-(--settings-key-foreground)"
          }
          aria-hidden="true"
        >
          {pending.length ? <ShieldAlert size={16} /> : <Shield size={16} />}
        </span>
        <div className="min-w-0">
          <h3 className="text-[13px] font-semibold">Third-Party Packages</h3>
          <p className="text-xs text-(--settings-description)" role="status">
            {pending.length > 0
              ? `${pending.length} awaiting approval. Review before allowing code to run.`
              : approved.length > 0
                ? "Manage which packages can run code."
                : "No third-party packages installed."}
          </p>
        </div>
      </div>

      {errorMessage && (
        <p role="alert" className="wrap-anywhere px-4 pb-3 text-xs text-destructive">
          {errorMessage}
        </p>
      )}

      {(showSearch || canSelect) && (
        <div className="flex flex-wrap items-center gap-x-3 gap-y-2 border-t border-(--settings-separator) px-4 py-2">
          {canSelect && (
            <Checkbox
              aria-label="Select all visible third-party packages"
              checked={allSelected}
              isIndeterminate={selected.length > 0 && !allSelected}
              onCheckedChange={(checked) => updateSelection(visible, checked)}
              className="text-xs text-(--settings-description)"
            >
              {selected.length === 0 ? "Select all" : null}
            </Checkbox>
          )}
          {selected.length > 0 && (
            <div
              role="group"
              aria-label="Bulk actions"
              className="flex flex-wrap items-center gap-2"
            >
              <span className="text-xs text-(--settings-description)" role="status">
                {selected.length} selected
              </span>
              {selectedPending.length > 0 && (
                <ApproveDialog
                  packages={selectedPending}
                  disabled={isPending}
                  label={`Review selected (${selectedPending.length})…`}
                  onApprove={() => setApprovals({ targets: selectedPending, approved: true })}
                />
              )}
              {selectedApproved.length > 0 && (
                <Button
                  size="sm"
                  variant="ghost"
                  disabled={isPending}
                  onClick={() => setApprovals({ targets: selectedApproved, approved: false })}
                >
                  Revoke selected ({selectedApproved.length})
                </Button>
              )}
              <Button
                variant="ghost"
                size="icon-xs"
                aria-label="Clear package selection"
                onClick={() => setSelection(new Set())}
              >
                <X size={14} />
              </Button>
            </div>
          )}
          {showSearch && (
            <label className="chord-library-search">
              <Search size={14} aria-hidden="true" />
              <Input
                aria-label="Search third-party packages"
                placeholder="Search third-party packages"
                value={search}
                onChange={(event) => setSearch(event.target.value)}
              />
              {search && (
                <Button
                  variant="ghost"
                  size="icon-xs"
                  aria-label="Clear third-party package search"
                  onClick={() => setSearch("")}
                >
                  <X size={14} />
                </Button>
              )}
            </label>
          )}
        </div>
      )}

      {visiblePending.length > 0 && (
        <ul aria-label="Packages awaiting approval">{visiblePending.map(renderPackage)}</ul>
      )}
      {query && visible.length === 0 && (
        <p
          role="status"
          className="border-t border-(--settings-separator) px-4 py-3 text-xs text-(--settings-description)"
        >
          No third-party packages match your search.
        </p>
      )}

      <div className="flex flex-wrap items-start gap-x-4 border-t border-(--settings-separator) px-4 py-2">
        {approved.length > 0 && (
          <button
            type="button"
            aria-expanded={showApproved}
            aria-controls={approvedId}
            onClick={() => setShowApproved(!showApproved)}
            className="group flex min-h-6 cursor-pointer items-center gap-1.5 rounded-sm text-xs font-medium text-(--settings-description) hover:text-foreground focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring"
          >
            <ChevronRight
              size={13}
              className="transition-transform group-aria-expanded:rotate-90"
              aria-hidden="true"
            />
            Approved packages ({approved.length})
            {failedCount > 0 && (
              <span className="text-destructive">· {failedCount} with errors</span>
            )}
          </button>
        )}
        <details className="group open:w-full">
          <summary className="flex min-h-6 cursor-pointer list-none items-center gap-1.5 rounded-sm text-xs font-medium text-(--settings-description) hover:text-foreground focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring [&::-webkit-details-marker]:hidden">
            <ChevronRight
              size={13}
              className="transition-transform group-open:rotate-90"
              aria-hidden="true"
            />
            Install a third-party package
          </summary>
          <p className="max-w-[65ch] pt-1.5 pb-1 pl-[19px] text-xs leading-relaxed text-(--settings-description)">
            Extract the package into <code>~/.chord/packages/&lt;name&gt;</code>, then quit and
            restart Chord. Review the package here to enable it. Official packages live in{" "}
            <code>~/.chord/packages/@keychord/</code>.
          </p>
        </details>
      </div>
      <ul id={approvedId} aria-label="Approved packages" hidden={!showApproved || !approved.length}>
        {visibleApproved.map(renderPackage)}
        {query && !visibleApproved.length && visiblePending.length > 0 && (
          <li className="border-t border-(--settings-separator) px-4 py-3 text-xs text-(--settings-description)">
            No approved packages match your search.
          </li>
        )}
      </ul>
    </section>
  );
}
