import type {
  ChordsFileProblem,
  PersonalPackageFiles,
  PersonalPackages,
} from "@chord/dev.improve.chord.api.taurpc";
import type { ChordPackage, PersonalPackageState } from "@chord/dev.improve.chord.lib.typeshare";
import { useMutation, useQuery, useQueryClient } from "@chord/com.npmjs.tanstack__react-query";
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
  AlertDialogTrigger,
} from "@chord/dev.improve.chord.components.ui.alert-dialog";
import { Button } from "@chord/dev.improve.chord.components.ui.button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@chord/dev.improve.chord.components.ui.card";
import type { CodeEditorDiagnostic } from "@chord/dev.improve.chord.components.ui.code-editor";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@chord/dev.improve.chord.components.ui.dialog";
import { Input } from "@chord/dev.improve.chord.components.ui.input";
import {
  NativeSelect,
  NativeSelectOption,
} from "@chord/dev.improve.chord.components.ui.native-select";
import { StateQueries, useChordPackageManagerState } from "@chord/dev.improve.chord.lib.state";
import {
  AppWindow,
  FileText,
  FolderOpen,
  Globe,
  Package,
  Plus,
  Trash2,
} from "lucide-react";
import { lazy, Suspense, useEffect, useMemo, useRef, useState } from "react";
import {
  ALL_APPS_PATHSLUG,
  chordsFileAppId,
  chordsFilePathslug,
  describeChordsFile,
  isValidAppId,
  isValidPackageDirname,
  newChordsFileContents,
} from "#personal-chords-file";

// CodeMirror loads when a file first opens, so it stays out of the chord panel's bundle.
const CodeEditor = lazy(() =>
  import("@chord/dev.improve.chord.components.ui.code-editor").then(({ CodeEditor }) => ({
    default: CodeEditor,
  })),
);

const PERSONAL_PACKAGES_KEY = ["personal-packages"];
const DEFAULT_PACKAGE = "default";

/** An unsaved edit. `base` is the file as it was when editing started; saving refuses to
 * overwrite changes made on disk since then. */
type Edit = { base: string; draft: string };
type EditUpdate = (current: Edit | null) => Edit | null;
type FileRef = { dirname: string; pathslug: string };

function fileKey({ dirname, pathslug }: FileRef) {
  return ["personal-chords-file", dirname, pathslug];
}

function editKey({ dirname, pathslug }: FileRef) {
  return `${dirname}/${pathslug}`;
}

export function SettingsPersonalPage() {
  const personalPackagesQuery = useQuery({
    queryKey: PERSONAL_PACKAGES_KEY,
    queryFn: () => taurpc.listPersonalPackages(),
  });
  return (
    <StateQueries queries={[personalPackagesQuery, useChordPackageManagerState()]}>
      {(listing, { packages, errors, personalPackages }) => (
        <PersonalChordsContent
          listing={listing}
          states={personalPackages}
          errors={errors}
          appIds={appIdsWithChords(packages)}
        />
      )}
    </StateQueries>
  );
}

/** Apps that loaded packages have chords for, offered when adding a file for an app. */
function appIdsWithChords(packages: ChordPackage[]) {
  const appIds = new Set<string>();
  for (const chordPackage of packages) {
    for (const pathslug of Object.keys(chordPackage.compiledChordsFiles)) {
      const appId = chordsFileAppId(pathslug);
      if (appId) appIds.add(appId);
    }
  }
  return [...appIds].sort();
}

export function PersonalChordsContent({
  listing,
  states,
  errors,
  appIds,
}: {
  listing: PersonalPackages;
  /** The packages as the last reload saw them. */
  states: PersonalPackageState[];
  /** Load errors from the last reload, by package name. */
  errors: Record<string, string[]>;
  appIds: string[];
}) {
  const [selected, setSelected] = useState<FileRef | null>(null);
  // Drafts are kept per file so switching files doesn't discard unsaved edits.
  const [edits, setEdits] = useState<Record<string, Edit>>({});
  const current = currentFile(listing.packages, selected);

  return (
    <div className="space-y-4">
      <Card size="sm">
        <CardHeader>
          <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
            <div className="space-y-1">
              <CardTitle>My Chords</CardTitle>
              <CardDescription>
                Every folder in <code>{listing.root}</code> is a chord package that Chord loads
                with your other packages. When one of these chords uses the same keys as another
                package's chord, this one runs. Chord creates <code>{DEFAULT_PACKAGE}</code> for
                you; saving a file reloads Chord.
              </CardDescription>
            </div>
            <div className="flex shrink-0 items-center gap-2">
              <OpenInFinderButton label="Open in Finder" />
              <NewPackageButton existing={listing.packages.map((pkg) => pkg.dirname)} />
            </div>
          </div>
        </CardHeader>
        <CardContent className="space-y-4 pt-0">
          {listing.packages.length === 0 ? (
            <p className="text-sm text-muted-foreground">
              No packages yet. Create one to start writing chords.
            </p>
          ) : (
            listing.packages.map((pkg) => {
              const state = states.find((candidate) => candidate.dirname === pkg.dirname);
              return (
                <PackageSection
                  key={pkg.dirname}
                  pkg={pkg}
                  state={state}
                  errors={state?.name ? (errors[state.name] ?? []) : []}
                  current={current}
                  edits={edits}
                  onSelect={setSelected}
                />
              );
            })
          )}
          {listing.packages.length > 0 && (
            <NewFileForm
              packages={listing.packages}
              defaultDirname={current?.dirname ?? DEFAULT_PACKAGE}
              appIds={appIds}
              onOpen={setSelected}
            />
          )}
        </CardContent>
      </Card>
      {current && (
        <FileEditor
          key={editKey(current)}
          file={current}
          edit={edits[editKey(current)] ?? null}
          onEdit={(update) => {
            const key = editKey(current);
            setEdits(({ [key]: previous, ...others }) => {
              const next = update(previous ?? null);
              return next ? { ...others, [key]: next } : others;
            });
          }}
        />
      )}
    </div>
  );
}

/** The chosen file while it exists, else the first file, preferring the default package. */
function currentFile(packages: PersonalPackageFiles[], selected: FileRef | null): FileRef | null {
  if (
    selected
    && packages.some((pkg) => pkg.dirname === selected.dirname && pkg.files.includes(selected.pathslug))
  ) {
    return selected;
  }
  const ordered = [
    ...packages.filter((pkg) => pkg.dirname === DEFAULT_PACKAGE),
    ...packages.filter((pkg) => pkg.dirname !== DEFAULT_PACKAGE),
  ];
  const first = ordered.find((pkg) => pkg.files.length > 0);
  return first?.files[0] ? { dirname: first.dirname, pathslug: first.files[0] } : null;
}

function PackageSection({
  pkg,
  state,
  errors,
  current,
  edits,
  onSelect,
}: {
  pkg: PersonalPackageFiles;
  state: PersonalPackageState | undefined;
  errors: string[];
  current: FileRef | null;
  edits: Record<string, Edit>;
  onSelect: (file: FileRef) => void;
}) {
  const problems = state?.error ? [state.error] : errors;
  return (
    <section className="space-y-2" aria-label={`${pkg.dirname} package`}>
      <div className="flex items-center gap-2">
        <Package className="size-4 shrink-0 text-muted-foreground" strokeWidth={1.8} />
        <h3 className="min-w-0 truncate text-sm font-semibold">{pkg.dirname}</h3>
        {state?.name && state.name !== pkg.dirname && (
          <span className="truncate text-xs text-muted-foreground">loads as {state.name}</span>
        )}
        <span className="flex-1" />
        <OpenInFinderButton dirname={pkg.dirname} label={`Open ${pkg.dirname} in Finder`} iconOnly />
      </div>
      {problems.length > 0 && <LoadErrors errors={problems} />}
      {pkg.files.length === 0 ? (
        <p className="text-xs text-muted-foreground">No chords files in this package.</p>
      ) : (
        <ul className="space-y-1" aria-label={`Chords files in ${pkg.dirname}`}>
          {pkg.files.map((pathslug) => {
            const file = { dirname: pkg.dirname, pathslug };
            return (
              <li key={pathslug}>
                <FileRow
                  pathslug={pathslug}
                  selected={current?.dirname === pkg.dirname && current.pathslug === pathslug}
                  unsaved={editKey(file) in edits}
                  onSelect={() => onSelect(file)}
                />
              </li>
            );
          })}
        </ul>
      )}
    </section>
  );
}

function OpenInFinderButton({
  dirname,
  label,
  iconOnly = false,
}: {
  dirname?: string;
  label: string;
  iconOnly?: boolean;
}) {
  const open = useMutation({ mutationFn: () => taurpc.openPersonalPackage(dirname ?? null) });
  return (
    <Button
      type="button"
      variant={iconOnly ? "ghost" : "outline"}
      size={iconOnly ? "icon-sm" : "sm"}
      aria-label={iconOnly ? label : undefined}
      title={iconOnly ? label : undefined}
      onClick={() => open.mutate()}
      disabled={open.isPending}
      className={iconOnly ? "text-muted-foreground" : undefined}
    >
      <FolderOpen />
      {!iconOnly && label}
    </Button>
  );
}

function NewPackageButton({ existing }: { existing: string[] }) {
  const queryClient = useQueryClient();
  const [open, setOpen] = useState(false);
  const [dirname, setDirname] = useState(existing.includes(DEFAULT_PACKAGE) ? "" : DEFAULT_PACKAGE);
  const trimmed = dirname.trim();
  const invalid = trimmed !== "" && !isValidPackageDirname(trimmed);
  const taken = existing.includes(trimmed);
  const create = useMutation({
    mutationFn: () => taurpc.createPersonalPackage(trimmed),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: PERSONAL_PACKAGES_KEY });
      setOpen(false);
      setDirname("");
    },
  });
  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        setOpen(next);
        create.reset();
      }}
    >
      <DialogTrigger asChild>
        <Button type="button" variant="outline" size="sm">
          <Plus />
          New Package
        </Button>
      </DialogTrigger>
      <DialogContent>
        <form
          className="space-y-4"
          onSubmit={(event) => {
            event.preventDefault();
            if (trimmed && !invalid && !taken) create.mutate();
          }}
        >
          <DialogHeader>
            <DialogTitle>New Package</DialogTitle>
            <DialogDescription>
              Creates a folder in your packages folder with a starter chords file.
            </DialogDescription>
          </DialogHeader>
          <Input
            aria-label="Package folder name"
            aria-invalid={invalid || taken || undefined}
            placeholder="work"
            value={dirname}
            onChange={(event) => setDirname(event.target.value)}
            spellCheck={false}
            autoCapitalize="none"
            autoCorrect="off"
            autoComplete="off"
            disabled={create.isPending}
          />
          {(invalid || taken) && (
            <p role="alert" className="text-xs text-destructive">
              {taken
                ? `${trimmed} already exists.`
                : "Start with a letter or digit, and use only letters, digits, hyphens, underscores, and periods."}
            </p>
          )}
          {create.isError && <ErrorMessage error={create.error} />}
          <DialogFooter>
            <Button type="submit" disabled={!trimmed || invalid || taken || create.isPending}>
              {create.isPending ? "Creating…" : "Create Package"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

function LoadErrors({ errors }: { errors: string[] }) {
  return (
    <div role="alert" className="space-y-2 rounded-lg border border-destructive/30 bg-destructive/5 p-3">
      <p className="text-sm font-medium text-destructive">
        Some of these chords didn't load on the last reload.
      </p>
      {errors.map((error) => (
        <pre key={error} className="whitespace-pre-wrap break-words font-mono text-xs text-destructive">
          {error}
        </pre>
      ))}
    </div>
  );
}

function FileRow({
  pathslug,
  selected,
  unsaved,
  onSelect,
}: {
  pathslug: string;
  selected: boolean;
  unsaved: boolean;
  onSelect: () => void;
}) {
  const { title, appId } = describeChordsFile(pathslug);
  const Icon = appId ? AppWindow : pathslug === ALL_APPS_PATHSLUG ? Globe : FileText;
  return (
    <button
      type="button"
      onClick={onSelect}
      aria-current={selected ? "true" : undefined}
      className={`flex w-full items-center gap-3 rounded-lg border px-3 py-2 text-left transition-colors ${
        selected ? "border-primary/40 bg-primary/5" : "bg-background/80 hover:bg-muted/50"
      }`}
    >
      <Icon className="size-4 shrink-0 text-muted-foreground" strokeWidth={1.8} />
      <span className="min-w-0 flex-1">
        <span className="block truncate text-sm font-medium">{title}</span>
        <span className="block truncate font-mono text-xs text-muted-foreground">{pathslug}</span>
      </span>
      {unsaved && <span className="shrink-0 text-xs text-muted-foreground">Unsaved</span>}
    </button>
  );
}

function NewFileForm({
  packages,
  defaultDirname,
  appIds,
  onOpen,
}: {
  packages: PersonalPackageFiles[];
  defaultDirname: string;
  appIds: string[];
  onOpen: (file: FileRef) => void;
}) {
  const queryClient = useQueryClient();
  const [chosenDirname, setDirname] = useState<string | null>(null);
  const [appId, setAppId] = useState("");
  const [invalid, setInvalid] = useState(false);
  const dirname
    = packages.find((pkg) => pkg.dirname === (chosenDirname ?? defaultDirname))?.dirname
      ?? packages[0]?.dirname
      ?? DEFAULT_PACKAGE;
  const files = packages.find((pkg) => pkg.dirname === dirname)?.files ?? [];
  const create = useMutation({
    mutationFn: (file: FileRef) =>
      taurpc.savePersonalChordsFile(file.dirname, file.pathslug, newChordsFileContents(appId), null),
    onSuccess: async (_, file) => {
      // Select the file once the list includes it, so the editor doesn't flash another file.
      await queryClient.invalidateQueries({ queryKey: PERSONAL_PACKAGES_KEY });
      setAppId("");
      onOpen(file);
    },
  });
  const suggestions = appIds.filter((id) => !files.includes(chordsFilePathslug(id)));
  return (
    <form
      className="space-y-1 border-t pt-3"
      onSubmit={(event) => {
        event.preventDefault();
        if (appId.trim() !== "" && !isValidAppId(appId)) {
          setInvalid(true);
          return;
        }
        const file = { dirname, pathslug: chordsFilePathslug(appId) };
        if (files.includes(file.pathslug)) {
          setAppId("");
          onOpen(file);
        } else {
          create.mutate(file);
        }
      }}
    >
      <div className="flex flex-col gap-2 sm:flex-row">
        {packages.length > 1 && (
          <NativeSelect
            aria-label="Package for the new chords file"
            value={dirname}
            onChange={(event) => setDirname(event.target.value)}
            disabled={create.isPending}
          >
            {packages.map((pkg) => (
              <NativeSelectOption key={pkg.dirname} value={pkg.dirname}>
                {pkg.dirname}
              </NativeSelectOption>
            ))}
          </NativeSelect>
        )}
        <Input
          aria-label="App bundle ID for a new chords file"
          aria-invalid={invalid || undefined}
          placeholder="App bundle ID, such as com.apple.finder (empty for every app)"
          list="personal-chords-app-ids"
          value={appId}
          onChange={(event) => {
            setAppId(event.target.value);
            setInvalid(false);
          }}
          spellCheck={false}
          autoCapitalize="none"
          autoCorrect="off"
          autoComplete="off"
          disabled={create.isPending}
        />
        <datalist id="personal-chords-app-ids">
          {suggestions.map((id) => (
            <option key={id} value={id} />
          ))}
        </datalist>
        <Button type="submit" variant="outline" disabled={create.isPending} className="shrink-0">
          <Plus />
          {create.isPending ? "Adding…" : "Add File"}
        </Button>
      </div>
      {invalid && (
        <p role="alert" className="text-xs text-destructive">
          Use a bundle ID made of letters, digits, hyphens, and periods, such as com.apple.finder.
        </p>
      )}
      {create.isError && <ErrorMessage error={create.error} />}
    </form>
  );
}

/**
 * Checks the editor's contents with the same rules Chord loads files with, on every change.
 * Only the reply for the latest contents is kept, so replies arriving out of order can't show
 * a stale problem.
 */
function useChordsFileProblem(contents: string) {
  const [checked, setChecked] = useState<{
    contents: string;
    problem: ChordsFileProblem | null;
  } | null>(null);
  const latest = useRef(contents);
  useEffect(() => {
    latest.current = contents;
    taurpc.checkChordsFile(contents).then(
      (problem) => {
        if (latest.current === contents) setChecked({ contents, problem });
      },
      // Saving reports the same problems, so a failed check only loses the early warning.
      () => {},
    );
  }, [contents]);
  return checked;
}

function FileEditor({
  file,
  edit,
  onEdit,
}: {
  file: FileRef;
  edit: Edit | null;
  onEdit: (update: EditUpdate) => void;
}) {
  const queryClient = useQueryClient();
  const formRef = useRef<HTMLFormElement>(null);
  const fileQuery = useQuery({
    queryKey: fileKey(file),
    queryFn: () => taurpc.readPersonalChordsFile(file.dirname, file.pathslug),
  });
  const save = useMutation({
    mutationFn: (next: Edit) =>
      taurpc.savePersonalChordsFile(file.dirname, file.pathslug, next.draft, next.base),
    onSuccess: (_, saved) => {
      queryClient.setQueryData(fileKey(file), saved.draft);
      // Typing may have continued while the save was in flight.
      onEdit((current) =>
        !current || current.draft === saved.draft ? null : { ...current, base: saved.draft },
      );
    },
  });
  const remove = useMutation({
    mutationFn: () => taurpc.deletePersonalChordsFile(file.dirname, file.pathslug),
    onSuccess: async () => {
      onEdit(() => null);
      // The editor closes once the list drops the file; only then forget its contents.
      await queryClient.invalidateQueries({ queryKey: PERSONAL_PACKAGES_KEY });
      queryClient.removeQueries({ queryKey: fileKey(file) });
    },
  });
  const value = edit?.draft ?? fileQuery.data ?? "";
  const checked = useChordsFileProblem(value);
  const diagnostics = useMemo<CodeEditorDiagnostic[]>(() => {
    const problem = checked?.problem;
    return problem && problem.from !== null
      ? [{ from: problem.from, to: problem.to ?? problem.from, message: problem.message }]
      : [];
  }, [checked]);
  const { title } = describeChordsFile(file.pathslug);
  const path = `${file.dirname}/${file.pathslug}`;

  if (fileQuery.data === undefined) {
    return (
      <Card size="sm">
        <CardContent>
          {fileQuery.isError ? (
            <div role="alert" className="space-y-2">
              <ErrorMessage error={fileQuery.error} />
              <Button type="button" variant="outline" size="sm" onClick={() => void fileQuery.refetch()}>
                Retry
              </Button>
            </div>
          ) : (
            <p role="status" className="text-sm text-muted-foreground">
              Loading {path}…
            </p>
          )}
        </CardContent>
      </Card>
    );
  }

  const contents = fileQuery.data;
  const dirty = edit !== null && edit.draft !== edit.base;
  const changedOnDisk = edit !== null && contents !== edit.base;
  const pending = save.isPending || remove.isPending;
  return (
    <Card size="sm">
      <form
        ref={formRef}
        className="contents"
        onSubmit={(event) => {
          event.preventDefault();
          if (edit && dirty && !pending) save.mutate(edit);
        }}
      >
        <CardHeader>
          <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
            <div className="min-w-0 space-y-1">
              <CardTitle>{title}</CardTitle>
              <CardDescription className="truncate font-mono text-xs">{path}</CardDescription>
            </div>
            <div className="flex shrink-0 items-center gap-2">
              {dirty && (
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={() => {
                    onEdit(() => null);
                    save.reset();
                    void fileQuery.refetch();
                  }}
                  disabled={pending}
                >
                  Revert
                </Button>
              )}
              <Button type="submit" size="sm" disabled={!dirty || pending}>
                {save.isPending ? "Saving…" : "Save"}
              </Button>
              <DeleteFileButton path={path} disabled={pending} onDelete={() => remove.mutate()} />
            </div>
          </div>
        </CardHeader>
        <CardContent className="space-y-2 pt-0">
          {changedOnDisk && (
            <p role="alert" className="text-xs text-destructive">
              This file changed on disk after you started editing, so saving won't overwrite it.
              Revert to load the new version.
            </p>
          )}
          <Suspense
            fallback={
              <div
                role="status"
                aria-label="Loading editor"
                className="h-[26rem] rounded-lg border bg-muted/30"
              />
            }
          >
            <CodeEditor
              aria-label={`Contents of ${path}`}
              language="toml"
              value={value}
              onChange={(draft) => {
                onEdit((current) => {
                  const base = current?.base ?? contents;
                  return draft === base ? null : { base, draft };
                });
              }}
              onSave={() => formRef.current?.requestSubmit()}
              diagnostics={diagnostics}
              readOnly={remove.isPending}
              className="h-[26rem]"
            />
          </Suspense>
          <p aria-live="polite" className="min-h-4 text-xs">
            {checked?.problem ? (
              <span className="text-destructive">
                {checked.problem.from !== null && <>Line {lineOf(checked.contents, checked.problem.from)}: </>}
                {checked.problem.message}
              </span>
            ) : (
              <span className="text-muted-foreground">
                <kbd>⌘S</kbd> saves and reloads Chord. <kbd>⌘F</kbd> finds and replaces.
              </span>
            )}
          </p>
          {save.isError && <ErrorMessage error={save.error} />}
          {remove.isError && <ErrorMessage error={remove.error} />}
        </CardContent>
      </form>
    </Card>
  );
}

function lineOf(contents: string, offset: number) {
  return contents.slice(0, offset).split("\n").length;
}

function DeleteFileButton({
  path,
  disabled,
  onDelete,
}: {
  path: string;
  disabled: boolean;
  onDelete: () => void;
}) {
  return (
    <AlertDialog>
      <AlertDialogTrigger asChild>
        <Button
          type="button"
          size="icon-sm"
          variant="ghost"
          aria-label={`Delete ${path}`}
          disabled={disabled}
          className="text-muted-foreground hover:text-destructive"
        >
          <Trash2 />
        </Button>
      </AlertDialogTrigger>
      <AlertDialogContent size="sm">
        <AlertDialogHeader>
          <AlertDialogTitle>Delete this file?</AlertDialogTitle>
          <AlertDialogDescription>
            <code>{path}</code> and its chords are removed, and Chord reloads without them.
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel>Cancel</AlertDialogCancel>
          <AlertDialogAction variant="destructive" onClick={onDelete}>
            Delete
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}

function ErrorMessage({ error }: { error: unknown }) {
  const message =
    typeof error === "string"
      ? error
      : error instanceof Error
        ? error.message
        : error && typeof error === "object" && "Message" in error
          ? String(error.Message)
          : "Something went wrong. Try again.";
  return (
    <pre role="alert" className="whitespace-pre-wrap break-words font-mono text-xs text-destructive">
      {message}
    </pre>
  );
}
