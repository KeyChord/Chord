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
import type { ThirdPartyPackage } from "@chord/dev.improve.chord.lib.typeshare";
import { Search, ShieldAlert, X } from "lucide-react";
import { useState } from "react";

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
      <Button
        size="sm"
        variant="outline"
        className="mac-button"
        disabled={disabled}
        aria-label={ariaLabel}
      >
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
  const visibleApproved = approved.filter(matches);
  const visible = [...visiblePending, ...visibleApproved];
  // Search can hide selected packages. Bulk actions only affect visible selections.
  const selected = visible.filter((pkg) => selection.has(pkg.identity));
  const selectedPending = selected.filter((pkg) => !pkg.approved);
  const selectedApproved = selected.filter((pkg) => pkg.approved);
  const allSelected = visible.length > 0 && selected.length === visible.length;
  const canSelect = visible.length > 1 || selected.length > 0;
  const showSearch = search !== "" || packages.length > 4;
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
      <li key={pkg.identity} className="mac-row">
        {canSelect && (
          <Checkbox
            aria-label={`Select ${pkg.name}`}
            checked={selection.has(pkg.identity)}
            onCheckedChange={(checked) => updateSelection([pkg], checked)}
          />
        )}
        <div className="mac-row-text">
          <span className="mac-row-title">{pkg.name}</span>
          <span title={pkg.path} className="mac-row-subtitle">
            {displayPath}
          </span>
          {pkg.approved &&
            errors[pkg.name]?.map((message) => (
              <span
                key={message}
                role="alert"
                className="mt-1 wrap-anywhere whitespace-pre-wrap text-xs text-destructive"
              >
                {message}
              </span>
            ))}
        </div>
        {pkg.approved ? (
          <button
            type="button"
            className="mac-button"
            disabled={isPending}
            aria-label={`Revoke approval for ${pkg.name}`}
            onClick={() => setApprovals({ targets: [pkg], approved: false })}
          >
            Revoke
          </button>
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
    <section aria-label="Third-party packages">
      <div className="mac-callout-warning" role="note">
        <ShieldAlert size={22} strokeWidth={1.8} aria-hidden="true" />
        <div className="min-w-0">
          <h3 className="mac-callout-title">Third-Party Packages</h3>
          <p className="mac-callout-body">{PACKAGE_CODE_WARNING}</p>
          <p className="mac-callout-body font-medium" role="status">
            {pending.length > 0
              ? `${pending.length} awaiting approval. Review before allowing code to run.`
              : approved.length > 0
                ? "Manage which packages can run code."
                : "No third-party packages installed."}
          </p>
        </div>
      </div>

      {errorMessage && (
        <p role="alert" className="mt-3 wrap-anywhere text-xs text-destructive">
          {errorMessage}
        </p>
      )}

      {(showSearch || canSelect) && (
        <div className="mt-5 flex flex-wrap items-center gap-x-3 gap-y-2 px-1">
          {canSelect && (
            <Checkbox
              aria-label="Select all visible third-party packages"
              checked={allSelected}
              isIndeterminate={selected.length > 0 && !allSelected}
              onCheckedChange={(checked) => updateSelection(visible, checked)}
              className="text-xs"
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
              <span className="mac-row-subtitle" role="status">
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
                <button
                  type="button"
                  className="mac-button"
                  disabled={isPending}
                  onClick={() => setApprovals({ targets: selectedApproved, approved: false })}
                >
                  Revoke selected ({selectedApproved.length})
                </button>
              )}
              <button
                type="button"
                className="mac-button"
                data-icon
                data-borderless
                aria-label="Clear package selection"
                onClick={() => setSelection(new Set())}
              >
                <X size={14} />
              </button>
            </div>
          )}
          {showSearch && (
            <label className="mac-search ml-auto">
              <Search size={13} aria-hidden="true" />
              <input
                aria-label="Search third-party packages"
                placeholder="Search"
                value={search}
                onChange={(event) => setSearch(event.target.value)}
              />
              {search && (
                <button
                  type="button"
                  aria-label="Clear third-party package search"
                  onClick={() => setSearch("")}
                >
                  <X size={13} />
                </button>
              )}
            </label>
          )}
        </div>
      )}

      {pending.length > 0 && (
        <div className="mac-section mt-5">
          <div className="mac-section-header">
            <h3 className="mac-section-title">Awaiting Approval</h3>
          </div>
          <ul className="mac-list" aria-label="Packages awaiting approval">
            {visiblePending.map(renderPackage)}
            {query && visiblePending.length === 0 && (
              <li className="mac-row-empty">No packages awaiting approval match your search.</li>
            )}
          </ul>
        </div>
      )}

      {approved.length > 0 && (
        <div className="mac-section mt-5">
          <div className="mac-section-header">
            <h3 className="mac-section-title">Approved packages ({approved.length})</h3>
            {failedCount > 0 && (
              <span className="mac-row-subtitle text-destructive">· {failedCount} with errors</span>
            )}
          </div>
          <ul className="mac-list" aria-label="Approved packages">
            {visibleApproved.map(renderPackage)}
            {query && visibleApproved.length === 0 && (
              <li className="mac-row-empty">No approved packages match your search.</li>
            )}
          </ul>
          <p className="mac-section-footer">
            Revoking turns a package off until you approve it again.
          </p>
        </div>
      )}

      <div className="mac-section mt-5">
        <div className="mac-section-header">
          <h3 className="mac-section-title">Install a third-party package</h3>
        </div>
        <div className="mac-list">
          <p className="px-3 py-2.5 text-[12px] leading-relaxed">
            Extract the package into <code>~/.chord/packages/&lt;name&gt;</code>, then quit and
            restart Chord. It appears above, waiting for your review. Official packages live in{" "}
            <code>~/.chord/packages/@keychord/</code>.
          </p>
        </div>
      </div>
    </section>
  );
}
