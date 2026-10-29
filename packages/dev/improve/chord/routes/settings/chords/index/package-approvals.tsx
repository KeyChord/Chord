import { useMutation } from "@chord/com.npmjs.tanstack__react-query";
import type { ColumnDef, Row } from "@chord/com.npmjs.tanstack__react-table";
import {
  columnFilteringFeature,
  createFilteredRowModel,
  globalFilteringFeature,
  rowSelectionFeature,
  Subscribe,
  tableFeatures,
  useTable,
} from "@chord/com.npmjs.tanstack__react-table";
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
import { Badge } from "@chord/dev.improve.chord.components.ui.badge";
import { Button } from "@chord/dev.improve.chord.components.ui.button";
import { Checkbox } from "@chord/dev.improve.chord.components.ui.checkbox";
import { Input } from "@chord/dev.improve.chord.components.ui.input";
import type { ThirdPartyPackage } from "@chord/dev.improve.chord.lib.typeshare";
import { Search, ShieldAlert, X } from "lucide-react";
import { useMemo } from "react";

export const PACKAGE_CODE_WARNING =
  "Chord scripts and native add-ons can run arbitrary code with your macOS permissions. Only approve packages from people you trust or after auditing their code.";

const approvalFeatures = tableFeatures({
  rowSelectionFeature,
  columnFilteringFeature,
  globalFilteringFeature,
  filteredRowModel: createFilteredRowModel(),
});

type ApprovalRow = Row<typeof approvalFeatures, ThirdPartyPackage>;

function countPackages(count: number) {
  return `${count} Package${count === 1 ? "" : "s"}`;
}

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
      <Button size="sm" disabled={disabled || packages.length === 0} aria-label={ariaLabel}>
        {label}
      </Button>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>
            Approve {single ? single.name : countPackages(packages.length)}?
          </AlertDialogTitle>
          <AlertDialogDescription>{PACKAGE_CODE_WARNING}</AlertDialogDescription>
        </AlertDialogHeader>
        <ul className="my-3 max-h-60 space-y-2 overflow-y-auto">
          {packages.map((pkg) => (
            <li key={pkg.identity}>
              {!single && <span className="block break-words text-sm">{pkg.name}</span>}
              <code className="block break-all text-xs text-muted-foreground">{pkg.path}</code>
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

function RowCheckbox({ row }: { row: ApprovalRow }) {
  // The core row is stable. Subscribe here so the React Compiler sees selection changes.
  return (
    <Subscribe source={row.table.atoms.rowSelection} selector={(selection) => !!selection[row.id]}>
      {(selected) => (
        <Checkbox
          aria-label={`Select ${row.original.name}`}
          checked={selected}
          onCheckedChange={(checked) => row.toggleSelected(checked)}
        />
      )}
    </Subscribe>
  );
}

export function PackageApprovals({
  packages,
  errors,
}: {
  packages: ThirdPartyPackage[];
  errors: Record<string, string[]>;
}) {
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
  });
  const data = useMemo(
    () =>
      packages.toSorted(
        (a, b) => Number(a.approved) - Number(b.approved) || a.name.localeCompare(b.name),
      ),
    [packages],
  );
  const pendingCount = data.filter((pkg) => !pkg.approved).length;
  const columns = useMemo<ColumnDef<typeof approvalFeatures, ThirdPartyPackage>[]>(
    () => [
      // The header checkbox is rendered with the table so it follows data as well as state.
      { id: "select", cell: ({ row }) => <RowCheckbox row={row} /> },
      {
        accessorKey: "name",
        header: "Package",
        cell: ({ row }) => {
          const pkg = row.original;
          return (
            <div className="min-w-0 space-y-1">
              <span className="block break-words text-sm font-medium">{pkg.name}</span>
              <code className="block break-all text-xs text-muted-foreground">{pkg.path}</code>
              {pkg.approved &&
                errors[pkg.name]?.map((message) => (
                  <p
                    key={message}
                    role="alert"
                    className="whitespace-pre-wrap break-words text-sm text-destructive"
                  >
                    {message}
                  </p>
                ))}
            </div>
          );
        },
      },
      {
        accessorKey: "approved",
        header: "Status",
        cell: ({ row }) =>
          row.original.approved ? (
            <Badge variant="secondary" title="Can run code">
              Approved
            </Badge>
          ) : (
            <Badge variant="outline" title="Approval required">
              Blocked
            </Badge>
          ),
      },
      {
        id: "actions",
        header: () => <span className="sr-only">Actions</span>,
        cell: ({ row }) => {
          const pkg = row.original;
          return pkg.approved ? (
            <Button
              size="sm"
              variant="outline"
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
              label="Approve…"
              ariaLabel={`Approve ${pkg.name}`}
              onApprove={() => setApprovals({ targets: [pkg], approved: true })}
            />
          );
        },
      },
    ],
    [errors, isPending, setApprovals],
  );
  const table = useTable({
    features: approvalFeatures,
    data,
    columns,
    getRowId: (pkg) => pkg.identity,
    globalFilterFn: (row, _columnId, value: string) =>
      [row.original.name, row.original.path].some((label) =>
        label.toLocaleLowerCase().includes(value.trim().toLocaleLowerCase()),
      ),
  });
  const visibleRows = table.getRowModel().rows;
  const search = String(table.state.globalFilter ?? "");
  // Hidden rows keep their selection, but bulk actions only apply to what the search shows.
  const selected = table.getFilteredSelectedRowModel().rows.map((row) => row.original);
  const selectedPending = selected.filter((pkg) => !pkg.approved);
  const selectedApproved = selected.filter((pkg) => pkg.approved);
  const allSelected = visibleRows.length > 0 && selected.length === visibleRows.length;
  const errorMessage = error
    ? typeof error === "object" && "Message" in error
      ? String(error.Message)
      : String(error)
    : null;

  function bulkUpdate(targets: ThirdPartyPackage[], approved: boolean) {
    setApprovals({ targets, approved }, { onSuccess: () => table.resetRowSelection() });
  }

  return (
    <section aria-label="Third-party packages" className="preference-disclosure px-5 py-4">
      <div className="flex items-center gap-2">
        <ShieldAlert className="size-4 text-muted-foreground" aria-hidden="true" />
        <h3 className="text-sm font-medium">Third-Party Packages</h3>
        <span className="ml-auto text-xs text-muted-foreground" role="status">
          {pendingCount > 0
            ? `${pendingCount} awaiting approval`
            : "No packages awaiting approval."}
        </span>
      </div>
      <p className="mt-3 text-sm text-muted-foreground">{PACKAGE_CODE_WARNING}</p>
      <p className="mb-4 mt-2 text-xs leading-relaxed text-muted-foreground">
        Download and extract the package into <code>~/.chord/packages/&lt;name&gt;</code>, then quit
        and restart Chord. Review and approve it here to enable it. Official packages live in{" "}
        <code>~/.chord/packages/@keychord/</code>.
      </p>
      {errorMessage && (
        <p role="alert" className="mb-4 break-words text-sm text-destructive">
          {errorMessage}
        </p>
      )}
      {data.length > 0 && (
        <>
          <div className="chord-library-toolbar">
            <label className="chord-library-search">
              <Search size={16} aria-hidden="true" />
              <Input
                aria-label="Search third-party packages"
                placeholder="Search packages"
                value={search}
                onChange={(event) => table.setGlobalFilter(event.target.value)}
              />
              {search && (
                <Button
                  variant="ghost"
                  type="button"
                  size="icon-xs"
                  aria-label="Clear search"
                  onClick={() => table.setGlobalFilter("")}
                >
                  <X size={14} />
                </Button>
              )}
            </label>
            {selected.length > 0 && (
              <div
                role="group"
                aria-label="Bulk actions"
                className="flex flex-wrap items-center gap-2"
              >
                <span className="text-xs text-muted-foreground" role="status">
                  {selected.length} selected
                </span>
                <ApproveDialog
                  packages={selectedPending}
                  disabled={isPending}
                  label={`Approve ${countPackages(selectedPending.length)}…`}
                  onApprove={() => bulkUpdate(selectedPending, true)}
                />
                <Button
                  size="sm"
                  variant="outline"
                  disabled={isPending || selectedApproved.length === 0}
                  onClick={() => bulkUpdate(selectedApproved, false)}
                >
                  Revoke {countPackages(selectedApproved.length)}
                </Button>
                <Button
                  variant="ghost"
                  type="button"
                  size="icon-xs"
                  aria-label="Clear selection"
                  onClick={() => table.resetRowSelection()}
                >
                  <X size={14} />
                </Button>
              </div>
            )}
            <span className="ml-auto whitespace-nowrap text-xs text-muted-foreground">
              {countPackages(visibleRows.length).toLowerCase()}
            </span>
          </div>
          <div className="chord-library-scroll">
            <table className="chord-library-table package-approvals-table">
              <caption className="sr-only">
                Third-party packages, their approval status, and approval actions
              </caption>
              <thead>
                {table.getHeaderGroups().map((group) => (
                  <tr key={group.id}>
                    {group.headers.map((header) => (
                      <th key={header.id} scope="col">
                        {header.column.id === "select" ? (
                          <Checkbox
                            aria-label="Select all packages"
                            checked={allSelected}
                            isIndeterminate={selected.length > 0 && !allSelected}
                            onCheckedChange={(checked) => table.toggleAllRowsSelected(checked)}
                          />
                        ) : header.isPlaceholder ? null : (
                          <table.FlexRender header={header} />
                        )}
                      </th>
                    ))}
                  </tr>
                ))}
              </thead>
              <tbody>
                {visibleRows.map((row) => (
                  <tr key={row.id} data-selected={!!table.state.rowSelection[row.id]}>
                    {row.getAllCells().map((cell) => (
                      <td key={cell.id}>
                        <table.FlexRender cell={cell} />
                      </td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
          {visibleRows.length === 0 && (
            <p className="py-6 text-center text-sm text-muted-foreground">
              No packages match your search.
            </p>
          )}
        </>
      )}
    </section>
  );
}
