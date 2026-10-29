import { useMutation, useQuery, useQueryClient } from "@chord/com.npmjs.tanstack__react-query";
import { taurpc } from "@chord/dev.improve.chord.api.taurpc";
import { Checkbox } from "@chord/dev.improve.chord.components.ui.checkbox";

export function MonorepoPackagePicker({
  source,
  disabled = false,
}: {
  source: string;
  disabled?: boolean;
}) {
  const client = useQueryClient();
  const queryKey = ["monorepo-packages", source];
  const packages = useQuery({ queryKey, queryFn: () => taurpc.listMonorepoPackages(source) });
  const selection = useMutation({
    mutationFn: (names: string[]) => taurpc.setMonorepoPackages(source, names),
    onSettled: () => client.invalidateQueries({ queryKey }),
  });
  const error = selection.error ?? packages.error;
  return (
    <fieldset
      className="mt-3 space-y-2"
      disabled={disabled || selection.isPending || packages.isFetching}
    >
      <legend className="text-sm font-medium">Packages to use</legend>
      <p className="text-xs text-muted-foreground">
        Select packages to import. Chord's core bindings are always enabled.
      </p>
      {packages.isLoading && <p className="text-sm text-muted-foreground">Loading packages...</p>}
      {packages.data?.length === 0 && (
        <p className="text-sm text-muted-foreground">No packages found.</p>
      )}
      {packages.data?.map(([name, checked]) => (
        <Checkbox
          key={name}
          checked={checked}
          disabled={
            disabled ||
            selection.isPending ||
            packages.isFetching ||
            name === "@keychord/chords-chord"
          }
          onCheckedChange={(nextChecked) => {
            const names = packages.data.filter(([, enabled]) => enabled).map(([value]) => value);
            selection.mutate(
              nextChecked ? [...names, name] : names.filter((value) => value !== name),
            );
          }}
        >
          {name}
          {name === "@keychord/chords-chord" ? " (Required)" : ""}
        </Checkbox>
      ))}
      {error && (
        <p role="alert" className="whitespace-pre-wrap break-words text-sm text-destructive">
          {typeof error === "object" && "Message" in error ? String(error.Message) : String(error)}
        </p>
      )}
    </fieldset>
  );
}
