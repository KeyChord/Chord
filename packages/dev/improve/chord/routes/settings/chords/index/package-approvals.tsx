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
import { GroupCard } from "@chord/dev.improve.chord.components.ui.group-card";
import { FolderDown, Search, ShieldAlert, X } from "lucide-react";
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

  /** One package as a row of the third-party card: select, name and status, then its action. */
  function renderPackage(pkg: ThirdPartyPackage) {
    const displayPath = pkg.path.replace(
      /^.*?[\\/]\.chord[\\/]packages[\\/]/,
      "~/.chord/packages/",
    );
    const failures = pkg.approved ? (errors[pkg.name] ?? []) : [];
    return (
      <div key={pkg.identity} role="listitem" className="mac-row">
        {canSelect && (
          <span className="mac-icon">
            <Checkbox
              aria-label={`Select ${pkg.name}`}
              checked={selection.has(pkg.identity)}
              onCheckedChange={(checked) => updateSelection([pkg], checked)}
            />
          </span>
        )}
        <span className="mac-row-text">
          <span className="mac-row-title">{pkg.name}</span>
          <span className="mac-row-subtitle">
            {pkg.approved ? (
              failures.length > 0 ? (
                <span className="text-destructive">Approved · didn&apos;t load</span>
              ) : (
                "Approved"
              )
            ) : (
              <span className="text-amber-700 dark:text-amber-300">Awaiting approval</span>
            )}
            {" · "}
            <span title={pkg.path}>{displayPath}</span>
          </span>
          {failures.map((message) => (
            <span
              key={message}
              role="alert"
              className="mac-row-subtitle wrap-anywhere whitespace-pre-wrap text-destructive"
            >
              {message}
            </span>
          ))}
        </span>
        {pkg.approved ? (
          <button
            type="button"
            className="mac-button"
            disabled={isPending}
            aria-label={`Revoke approval for ${pkg.name}`}
            title="Turns the package off until you approve it again."
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
      </div>
    );
  }

  // One card holds everything third-party: its header says what these are and carries Select
  // all and search; the packages and how to add one are its rows.
  return (
    <section aria-label="Third-party packages">
      {errorMessage && (
        <p role="alert" className="mt-3 wrap-anywhere text-xs text-destructive">
          {errorMessage}
        </p>
      )}

      <div className="mac-section">
        <GroupCard
          // Yellow throughout, so it reads apart from Chord's own packages: this code runs
          // with your permissions.
          className="mac-group-warning"
          icon={<ShieldAlert size={17} strokeWidth={1.7} />}
          title="Third-party packages"
          subtitle={
            pending.length > 0 ? (
              <span role="status">
                {pending.length} awaiting approval. Approve only packages you trust.
              </span>
            ) : (
              "Their code runs with your permissions. Approve only packages you trust."
            )
          }
          count={packages.length}
          listLabel="Third-party packages list"
          actions={
            <>
              {showSearch && (
                <label className="mac-search">
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
              {canSelect && (
                <Checkbox
                  aria-label="Select all visible third-party packages"
                  checked={allSelected}
                  isIndeterminate={selected.length > 0 && !allSelected}
                  onCheckedChange={(checked) => updateSelection(visible, checked)}
                >
                  Select all
                </Checkbox>
              )}
            </>
          }
        >
          {selected.length > 0 && (
            <div role="group" aria-label="Bulk actions" className="mac-row mac-bulk-row">
              <span className="mac-row-subtitle flex-1" role="status">
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
          {visiblePending.map(renderPackage)}
          {visibleApproved.map(renderPackage)}
          {query && visible.length === 0 && (
            <p role="listitem" className="mac-row mac-row-muted">
              No third-party packages match your search.
            </p>
          )}
          <p role="listitem" className="mac-row mac-row-muted mac-install-row">
            <span className="mac-icon" aria-hidden="true">
              <FolderDown size={16} strokeWidth={1.6} />
            </span>
            <span className="min-w-0 flex-1">
              Install a third-party package: extract it into{" "}
              <code>~/.chord/packages/&lt;name&gt;</code>, then restart Chord. It appears here,
              waiting for your review.
            </span>
          </p>
        </GroupCard>
      </div>
    </section>
  );
}
