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
import { Clock, FolderDown, Search, ShieldAlert, ShieldCheck, X } from "lucide-react";
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

  function renderPackage(pkg: ThirdPartyPackage) {
    const displayPath = pkg.path.replace(
      /^.*?[\\/]\.chord[\\/]packages[\\/]/,
      "~/.chord/packages/",
    );
    const failures = pkg.approved ? (errors[pkg.name] ?? []) : [];
    // Each package is its own card, like the Packages tab; load errors open inside it.
    return (
      <div key={pkg.identity} role="listitem" className="mac-section">
        <GroupCard
          leading={
            canSelect && (
              <Checkbox
                aria-label={`Select ${pkg.name}`}
                checked={selection.has(pkg.identity)}
                onCheckedChange={(checked) => updateSelection([pkg], checked)}
              />
            )
          }
          icon={
            pkg.approved ? (
              <ShieldCheck size={17} strokeWidth={1.6} />
            ) : (
              <Clock size={17} strokeWidth={1.6} />
            )
          }
          title={pkg.name}
          subtitle={
            <>
              {pkg.approved ? (
                failures.length > 0 ? (
                  <span className="text-destructive">Third-party · didn&apos;t load</span>
                ) : (
                  "Third-party · approved"
                )
              ) : (
                <span className="text-amber-700 dark:text-amber-300">
                  Third-party · awaiting approval
                </span>
              )}
              {" · "}
              <span title={pkg.path}>{displayPath}</span>
            </>
          }
          listLabel={`${pkg.name} errors`}
          actions={
            pkg.approved ? (
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
            )
          }
        >
          {failures.length > 0 &&
            failures.map((message) => (
              <p
                key={message}
                role="alert"
                className="mac-row wrap-anywhere whitespace-pre-wrap text-destructive"
              >
                {message}
              </p>
            ))}
        </GroupCard>
      </div>
    );
  }

  return (
    <section aria-label="Third-party packages">
      {/* The trust warning shows while something waits for approval, where it matters. */}
      {pending.length > 0 && (
        <div className="mac-callout-warning" role="note">
          <ShieldAlert size={22} strokeWidth={1.8} aria-hidden="true" />
          <div className="min-w-0">
            <h3 className="mac-callout-title">Third-Party Packages</h3>
            <p className="mac-callout-body">{PACKAGE_CODE_WARNING}</p>
            <p className="mac-callout-body font-medium" role="status">
              {`${pending.length} awaiting approval. Review before allowing code to run.`}
            </p>
          </div>
        </div>
      )}

      {errorMessage && (
        <p role="alert" className="mt-3 wrap-anywhere text-xs text-destructive">
          {errorMessage}
        </p>
      )}

      {(showSearch || canSelect) && (
        <div className="mac-list-tools">
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

      <div className="mac-card-list" role="list" aria-label="Third-party packages list">
        {visiblePending.map(renderPackage)}
        {visibleApproved.map(renderPackage)}
      </div>
      {query && visible.length === 0 && (
        <div className="mac-section">
          <GroupCard title="No packages match your search" />
        </div>
      )}

      <div className="mac-section">
        <GroupCard
          icon={<FolderDown size={17} strokeWidth={1.6} />}
          title="Install a third-party package"
          subtitle="Extract it into ~/.chord/packages, then restart Chord."
          defaultOpen={packages.length === 0}
          listLabel="How to install a third-party package"
        >
          <div className="mac-row">
            <p className="min-w-0 flex-1 pl-8 leading-relaxed">
              Extract the package into <code>~/.chord/packages/&lt;name&gt;</code>, then quit and
              restart Chord. It appears above, waiting for your review. Official packages live in{" "}
              <code>~/.chord/packages/@keychord/</code>.
            </p>
          </div>
        </GroupCard>
      </div>
    </section>
  );
}
