import type {
  ChordsFileProblem,
  PersonalPackageFiles,
  PersonalPackages,
} from "@chord/dev.improve.chord.api.taurpc";
import type {
  ChordPackage,
  DesktopAppMetadata,
  PersonalPackageState,
} from "@chord/dev.improve.chord.lib.typeshare";
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
} from "@chord/dev.improve.chord.components.ui.alert-dialog";
import { Button } from "@chord/dev.improve.chord.components.ui.button";
import type { CodeEditorDiagnostic } from "@chord/dev.improve.chord.components.ui.code-editor";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@chord/dev.improve.chord.components.ui.dialog";
import { KeyLabel } from "@chord/dev.improve.chord.components.ui.kbd";
import { Select } from "@chord/dev.improve.chord.components.ui.select";
import { showNativeMenu } from "@chord/dev.improve.chord.lib.native-menu";
import {
  StateQueries,
  useChordPackageManagerState,
  useDesktopAppManagerState,
} from "@chord/dev.improve.chord.lib.state";
import {
  AppWindow,
  ChevronLeft,
  ChevronRight,
  Ellipsis,
  FileText,
  FolderOpen,
  Globe,
  Plus,
  Trash2,
} from "lucide-react";
import { lazy, Suspense, useEffect, useId, useMemo, useRef, useState } from "react";
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

export function PersonalChords() {
  const personalPackagesQuery = useQuery({
    queryKey: PERSONAL_PACKAGES_KEY,
    queryFn: () => taurpc.listPersonalPackages(),
  });
  const apps = useDesktopAppManagerState().data?.appsMetadata;
  return (
    <StateQueries queries={[personalPackagesQuery, useChordPackageManagerState()]}>
      {(listing, { packages, errors, personalPackages }) => (
        <PersonalChordsContent
          listing={listing}
          states={personalPackages}
          errors={errors}
          packages={packages}
          appIds={appIdsWithChords(packages)}
          apps={apps}
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
  packages = [],
  appIds,
  apps = {},
}: {
  listing: PersonalPackages;
  /** The packages as the last reload saw them. */
  states: PersonalPackageState[];
  /** Load errors from the last reload, by package name. */
  errors: Record<string, string[]>;
  packages?: ChordPackage[];
  appIds: string[];
  apps?: Record<string, DesktopAppMetadata>;
}) {
  const [selected, setSelected] = useState<FileRef | null>(null);
  // Drafts are kept per file so switching files doesn't discard unsaved edits.
  const [edits, setEdits] = useState<Record<string, Edit>>({});
  const [addingTo, setAddingTo] = useState<string | null>(null);
  const current = currentFile(listing.packages, selected);
  const existing = listing.packages.map((pkg) => pkg.dirname);

  // An open file takes over the pane, so the editor has room to work.
  if (current) {
    return (
      <FileEditor
        key={editKey(current)}
        file={current}
        apps={apps}
        edit={edits[editKey(current)] ?? null}
        onEdit={(update) => {
          const key = editKey(current);
          setEdits(({ [key]: previous, ...others }) => {
            const next = update(previous ?? null);
            return next ? { ...others, [key]: next } : others;
          });
        }}
        onClose={() => setSelected(null)}
      />
    );
  }

  return (
    <section aria-label="Local Chords">
      <div className="mb-5 flex items-center gap-2">
        <p className="mac-intro min-w-0 flex-1">
          Your own chords. They take priority over every installed package.
        </p>
        <OpenInFinderButton label="Open in Finder" />
        <NewPackageButton existing={existing} />
        {listing.packages.length > 0 && (
          <AddFileButton
            packages={listing.packages}
            defaultDirname={addingTo ?? DEFAULT_PACKAGE}
            appIds={appIds}
            open={addingTo !== null}
            onOpenChange={(open) => setAddingTo(open ? (addingTo ?? DEFAULT_PACKAGE) : null)}
            onOpen={(file) => {
              setAddingTo(null);
              setSelected(file);
            }}
          />
        )}
      </div>
      {listing.packages.length === 0 ? (
        <div className="mac-list">
          <p className="mac-row-empty">No packages yet. Create one to start writing chords.</p>
        </div>
      ) : (
        listing.packages.map((pkg) => {
          const state = states.find((candidate) => candidate.dirname === pkg.dirname);
          return (
            <PackageSection
              key={pkg.dirname}
              pkg={pkg}
              state={state}
              errors={state?.name ? (errors[state.name] ?? []) : []}
              loadedPackage={packages.find((loaded) => loaded.name === state?.name)}
              apps={apps}
              edits={edits}
              onSelect={setSelected}
              onAddFile={() => setAddingTo(pkg.dirname)}
            />
          );
        })
      )}
    </section>
  );
}

/** Only open an editor after Edit or Add File, and close it when its file is removed. */
function currentFile(packages: PersonalPackageFiles[], selected: FileRef | null): FileRef | null {
  if (
    selected &&
    packages.some(
      (pkg) => pkg.dirname === selected.dirname && pkg.files.includes(selected.pathslug),
    )
  ) {
    return selected;
  }
  return null;
}

function PackageSection({
  pkg,
  state,
  errors,
  loadedPackage,
  apps,
  edits,
  onSelect,
  onAddFile,
}: {
  pkg: PersonalPackageFiles;
  state: PersonalPackageState | undefined;
  errors: string[];
  loadedPackage: ChordPackage | undefined;
  apps: Record<string, DesktopAppMetadata>;
  edits: Record<string, Edit>;
  onSelect: (file: FileRef) => void;
  onAddFile: () => void;
}) {
  const problems = state?.error ? [state.error] : errors;
  const openFolder = useMutation({ mutationFn: () => taurpc.openPersonalPackage(pkg.dirname) });
  return (
    <section className="mac-section" aria-label={`${pkg.dirname} package`}>
      <div className="mac-section-header">
        <h3 className="mac-section-title truncate">{pkg.dirname}</h3>
        {state?.name && state.name !== pkg.dirname && (
          <span className="mac-row-subtitle">loads as {state.name}</span>
        )}
        <button
          type="button"
          className="mac-button ml-auto"
          data-icon
          data-borderless
          aria-label={`${pkg.dirname} actions`}
          title="Actions"
          onClick={(event) =>
            void showNativeMenu(
              [
                { label: "Add Chords File…", onSelect: onAddFile },
                "separator",
                { label: "Show in Finder", onSelect: () => openFolder.mutate() },
              ],
              event.currentTarget,
            )
          }
        >
          <Ellipsis size={15} aria-hidden="true" />
        </button>
      </div>
      {problems.length > 0 && <LoadErrors errors={problems} />}
      <ul className="mac-list" data-icons aria-label={`Chords files in ${pkg.dirname}`}>
        {pkg.files.length === 0 ? (
          <li className="mac-row-empty">No chords files in this package.</li>
        ) : (
          pkg.files.map((pathslug) => {
            const file = { dirname: pkg.dirname, pathslug };
            return (
              <li key={pathslug} className="contents">
                <FileRow
                  pathslug={pathslug}
                  apps={apps}
                  chordCount={loadedPackage?.compiledChordsFiles[pathslug]?.chords.length}
                  unsaved={editKey(file) in edits}
                  onSelect={() => onSelect(file)}
                />
              </li>
            );
          })
        )}
      </ul>
      <p className="mac-section-footer truncate font-mono" title={pkg.path}>
        {pkg.path}
      </p>
    </section>
  );
}

function OpenInFinderButton({ dirname, label }: { dirname?: string; label: string }) {
  const open = useMutation({ mutationFn: () => taurpc.openPersonalPackage(dirname ?? null) });
  return (
    <button
      type="button"
      className="mac-button"
      onClick={() => open.mutate()}
      disabled={open.isPending}
    >
      <FolderOpen size={14} aria-hidden="true" />
      {label}
    </button>
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
      <Button type="button" variant="outline" size="sm" className="mac-button">
        <Plus />
        New Package
      </Button>

      <DialogContent>
        <form
          className="mac-page space-y-4"
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
          <input
            className="mac-field w-full"
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
            <button
              type="submit"
              className="mac-button"
              data-default
              disabled={!trimmed || invalid || taken || create.isPending}
            >
              {create.isPending ? "Creating…" : "Create Package"}
            </button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

function LoadErrors({ errors }: { errors: string[] }) {
  return (
    <div
      role="alert"
      className="mb-2 space-y-2 rounded-[10px] border border-destructive/30 bg-destructive/5 p-3"
    >
      <p className="text-[13px] font-medium text-destructive">
        Some of these chords didn't load on the last reload.
      </p>
      {errors.map((error) => (
        <pre
          key={error}
          className="whitespace-pre-wrap break-words font-mono text-xs text-destructive"
        >
          {error}
        </pre>
      ))}
    </div>
  );
}

/** The icon for a chords file: the app's own icon when Chord knows it. */
function FileIcon({
  pathslug,
  apps,
}: {
  pathslug: string;
  apps: Record<string, DesktopAppMetadata>;
}) {
  const { appId } = describeChordsFile(pathslug);
  const icon = appId ? apps[appId]?.iconDataUrl : undefined;
  if (icon) {
    return (
      <span className="mac-icon" data-image aria-hidden="true">
        <img src={icon} alt="" />
      </span>
    );
  }
  const everyApp = pathslug === ALL_APPS_PATHSLUG;
  const Icon = appId ? AppWindow : everyApp ? Globe : FileText;
  return (
    <span
      className="mac-icon"
      data-tint={everyApp ? "blue" : appId ? "orange" : undefined}
      aria-hidden="true"
    >
      <Icon size={16} strokeWidth={2} />
    </span>
  );
}

function FileRow({
  pathslug,
  apps,
  chordCount,
  unsaved,
  onSelect,
}: {
  pathslug: string;
  apps: Record<string, DesktopAppMetadata>;
  chordCount?: number;
  unsaved: boolean;
  onSelect: () => void;
}) {
  const { title, appId } = describeChordsFile(pathslug);
  const appName = appId ? apps[appId]?.displayName?.trim() : undefined;
  return (
    <button type="button" className="mac-row" onClick={onSelect} aria-label={`Edit ${pathslug}`}>
      <FileIcon pathslug={pathslug} apps={apps} />
      <span className="mac-row-text">
        <span className="mac-row-title">{appName || title}</span>
        <span className="mac-row-subtitle font-mono">{pathslug}</span>
      </span>
      {unsaved && <span className="mac-row-detail">Unsaved</span>}
      {chordCount !== undefined && (
        <span className="mac-row-detail">
          {chordCount} chord{chordCount === 1 ? "" : "s"}
        </span>
      )}
      <ChevronRight className="mac-row-chevron" size={15} strokeWidth={2} aria-hidden="true" />
    </button>
  );
}

/** A sheet that adds a chords file for every app or one app, then opens it. */
function AddFileButton({
  packages,
  defaultDirname,
  appIds,
  open,
  onOpenChange,
  onOpen,
}: {
  packages: PersonalPackageFiles[];
  defaultDirname: string;
  appIds: string[];
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onOpen: (file: FileRef) => void;
}) {
  const queryClient = useQueryClient();
  const [chosenDirname, setDirname] = useState<string | null>(null);
  const [appId, setAppId] = useState("");
  const [invalid, setInvalid] = useState(false);
  const dirname =
    packages.find((pkg) => pkg.dirname === (chosenDirname ?? defaultDirname))?.dirname ??
    packages[0]?.dirname ??
    DEFAULT_PACKAGE;
  const files = packages.find((pkg) => pkg.dirname === dirname)?.files ?? [];
  const create = useMutation({
    mutationFn: (file: FileRef) =>
      taurpc.savePersonalChordsFile(
        file.dirname,
        file.pathslug,
        newChordsFileContents(appId),
        null,
      ),
    onSuccess: async (_, file) => {
      // Select the file once the list includes it, so the editor doesn't flash another file.
      await queryClient.invalidateQueries({ queryKey: PERSONAL_PACKAGES_KEY });
      setAppId("");
      onOpen(file);
    },
  });
  const suggestions = appIds.filter((id) => !files.includes(chordsFilePathslug(id)));
  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        onOpenChange(next);
        setDirname(null);
        create.reset();
      }}
    >
      <Button type="button" variant="outline" size="sm" className="mac-button">
        <Plus />
        Add File
      </Button>
      <DialogContent>
        <form
          className="mac-page space-y-4"
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
          <DialogHeader>
            <DialogTitle>Add Chords File</DialogTitle>
            <DialogDescription>
              Leave the app empty for chords that work in every app.
            </DialogDescription>
          </DialogHeader>
          <div className="grid grid-cols-[auto_1fr] items-center gap-x-3 gap-y-3">
            {packages.length > 1 && (
              <>
                <label className="text-right text-[13px]" htmlFor="personal-chords-package">
                  Package:
                </label>
                <Select
                  id="personal-chords-package"
                  aria-label="Package for the new chords file"
                  value={dirname}
                  onValueChange={setDirname}
                  disabled={create.isPending}
                  options={packages.map((pkg) => ({ value: pkg.dirname, label: pkg.dirname }))}
                  className="justify-self-start"
                />
              </>
            )}
            <label className="text-right text-[13px]" htmlFor="personal-chords-app-id">
              App:
            </label>
            <input
              id="personal-chords-app-id"
              className="mac-field w-full"
              aria-label="App bundle ID for a new chords file"
              aria-invalid={invalid || undefined}
              placeholder="com.apple.finder"
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
          </div>
          <datalist id="personal-chords-app-ids">
            {suggestions.map((id) => (
              <option key={id} value={id} />
            ))}
          </datalist>
          {invalid && (
            <p role="alert" className="text-xs text-destructive">
              Use a bundle ID made of letters, digits, hyphens, and periods, such as
              com.apple.finder.
            </p>
          )}
          {create.isError && <ErrorMessage error={create.error} />}
          <DialogFooter>
            <button type="submit" className="mac-button" data-default disabled={create.isPending}>
              {create.isPending ? "Adding…" : "Add File"}
            </button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
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
  apps,
  edit,
  onEdit,
  onClose,
}: {
  file: FileRef;
  apps: Record<string, DesktopAppMetadata>;
  edit: Edit | null;
  onEdit: (update: EditUpdate) => void;
  onClose: () => void;
}) {
  const queryClient = useQueryClient();
  const formRef = useRef<HTMLFormElement>(null);
  const formId = useId();
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
  const { title, appId } = describeChordsFile(file.pathslug);
  const displayTitle = (appId && apps[appId]?.displayName?.trim()) || title;
  const path = `${file.dirname}/${file.pathslug}`;
  const contents = fileQuery.data;
  const dirty = edit !== null && contents !== undefined && edit.draft !== edit.base;
  const changedOnDisk = edit !== null && contents !== undefined && contents !== edit.base;
  const pending = save.isPending || remove.isPending;

  const bar = (
    <div className="mac-detail-bar">
      <button type="button" className="mac-back" onClick={onClose} disabled={pending}>
        <ChevronLeft size={16} strokeWidth={2} aria-hidden="true" />
        Local Chords
      </button>
      <FileIcon pathslug={file.pathslug} apps={apps} />
      <div className="min-w-0 flex-1">
        <h2 className="mac-row-title text-[13px] font-semibold">
          {displayTitle}
          {dirty && <span className="mac-row-subtitle"> — Edited</span>}
        </h2>
        <p className="mac-row-subtitle font-mono" title={path}>
          {path}
        </p>
      </div>
      {contents !== undefined && (
        <>
          <DeleteFileButton path={path} disabled={pending} onDelete={() => remove.mutate()} />
          {dirty && (
            <button
              type="button"
              className="mac-button"
              onClick={() => {
                onEdit(() => null);
                save.reset();
                void fileQuery.refetch();
              }}
              disabled={pending}
            >
              Revert
            </button>
          )}
          <button
            type="submit"
            form={formId}
            className="mac-button"
            data-default
            disabled={!dirty || pending}
          >
            {save.isPending ? "Saving…" : "Save"}
          </button>
        </>
      )}
    </div>
  );

  if (contents === undefined) {
    return (
      <section aria-label={`Edit ${path}`}>
        {bar}
        <div className="mac-list p-4">
          {fileQuery.isError ? (
            <div role="alert" className="space-y-2">
              <ErrorMessage error={fileQuery.error} />
              <button type="button" className="mac-button" onClick={() => void fileQuery.refetch()}>
                Retry
              </button>
            </div>
          ) : (
            <p role="status" className="mac-row-subtitle">
              Loading {path}…
            </p>
          )}
        </div>
      </section>
    );
  }

  return (
    <section aria-label={`Edit ${path}`} className="mac-editor">
      {bar}
      <form
        id={formId}
        ref={formRef}
        aria-label={`Edit ${path}`}
        className="contents"
        onSubmit={(event) => {
          event.preventDefault();
          if (edit && dirty && !pending) save.mutate(edit);
        }}
      >
        {changedOnDisk && (
          <p role="alert" className="mb-2 text-xs text-destructive">
            This file changed on disk after you started editing, so saving won't overwrite it.
            Revert to load the new version.
          </p>
        )}
        <div className="mac-editor-surface">
          <Suspense fallback={<div role="status" aria-label="Loading editor" className="h-full" />}>
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
              className="h-full"
            />
          </Suspense>
        </div>
        <div aria-live="polite" className="mac-editor-status">
          {checked?.problem ? (
            <span className="text-destructive">
              {checked.problem.from !== null && (
                <>Line {lineOf(checked.contents, checked.problem.from)}: </>
              )}
              {checked.problem.message}
            </span>
          ) : (
            <span>
              <kbd>
                <KeyLabel label="⌘" />S
              </kbd>{" "}
              saves and reloads Chord.{" "}
              <kbd>
                <KeyLabel label="⌘" />F
              </kbd>{" "}
              finds and replaces.
            </span>
          )}
        </div>
        {save.isError && <ErrorMessage error={save.error} />}
        {remove.isError && <ErrorMessage error={remove.error} />}
      </form>
    </section>
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
      <Button
        type="button"
        size="icon-sm"
        variant="ghost"
        aria-label={`Delete ${path}`}
        disabled={disabled}
        className="mac-button text-muted-foreground hover:text-destructive"
        data-icon
        data-borderless
      >
        <Trash2 />
      </Button>

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
    <pre
      role="alert"
      className="whitespace-pre-wrap break-words font-mono text-xs text-destructive"
    >
      {message}
    </pre>
  );
}
