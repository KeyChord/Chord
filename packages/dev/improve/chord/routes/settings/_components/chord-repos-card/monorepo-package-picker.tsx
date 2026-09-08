import { useMutation, useQuery, useQueryClient } from "@chord/com.npmjs.tanstack__react-query";
import { taurpc } from "@chord/dev.improve.chord.api.taurpc";

export function MonorepoPackagePicker({ source, disabled = false }: { source: string; disabled?: boolean }) {
  const client = useQueryClient();
  const queryKey = ["monorepo-packages", source];
  const packages = useQuery({ queryKey, queryFn: () => taurpc.listMonorepoPackages(source) });
  const selection = useMutation({
    mutationFn: (names: string[]) => taurpc.setMonorepoPackages(source, names),
    onSettled: () => client.invalidateQueries({ queryKey }),
  });
  const error = selection.error ?? packages.error;
  return (
    <fieldset className="mt-3 space-y-2" disabled={disabled || selection.isPending || packages.isFetching}>
      <legend className="text-sm font-medium">Packages to use</legend>
      <p className="text-xs text-muted-foreground">Select packages to import. New packages are never selected automatically.</p>
      {packages.isLoading && <p className="text-sm text-muted-foreground">Loading packages...</p>}
      {packages.data?.length === 0 && <p className="text-sm text-muted-foreground">No packages found.</p>}
      {packages.data?.map(([name, checked]) => (
        <label key={name} className="flex items-center gap-2 text-sm">
          <input type="checkbox" checked={checked} onChange={(event) => {
            const names = packages.data.filter(([, enabled]) => enabled).map(([value]) => value);
            selection.mutate(event.target.checked ? [...names, name] : names.filter((value) => value !== name));
          }} />
          {name}
        </label>
      ))}
      {error && <p role="alert" className="text-sm text-destructive">{typeof error === "object" && "Message" in error ? String(error.Message) : String(error)}</p>}
    </fieldset>
  );
}
