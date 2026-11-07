import type { ChordMacroEntry, ChordMacroName } from "@chord/dev.improve.chord.lib.typeshare";
import { useMutation } from "@chord/com.npmjs.tanstack__react-query";
import { taurpc } from "@chord/dev.improve.chord.api.taurpc";
import { Badge } from "@chord/dev.improve.chord.components.ui.badge";
import { Button } from "@chord/dev.improve.chord.components.ui.button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@chord/dev.improve.chord.components.ui.card";
import { Input } from "@chord/dev.improve.chord.components.ui.input";
import { KeyLabel } from "@chord/dev.improve.chord.components.ui.kbd";
import { Textarea } from "@chord/dev.improve.chord.components.ui.textarea";
import { StateQueries, useChordMacrosState } from "@chord/dev.improve.chord.lib.state";
import { Pencil, Trash2 } from "lucide-react";
import { Fragment, useState, type ComponentProps } from "react";
import { sequenceLines, sequenceToText, textToSequence } from "#macro-sequence-text";

export function SettingsMacrosPage() {
  return (
    <StateQueries queries={[useChordMacrosState()]}>
      {(state) => <MacrosContent entries={state.entries} />}
    </StateQueries>
  );
}

export function MacrosContent({ entries }: { entries: ChordMacroEntry[] }) {
  const named = entries.filter((entry) => entry.target.type === "named");
  return (
    <div className="space-y-4">
      <Card size="sm">
        <CardHeader>
          <CardTitle>Named Macros</CardTitle>
          <CardDescription>
            Expand a macro with <code>{"\\name="}</code>. Names starting with 1–9, or with n, up,
            left, down, or right in macro text, need the leading <code>=</code> of{" "}
            <code>{"\\=name="}</code>. Names starting with a digit clear when Chord restarts; other
            names are saved. <EnterHint />
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <NamedMacroEditor />
          <div className="space-y-2 border-t pt-4">
            {named.length === 0 ? (
              <p className="text-sm text-muted-foreground">No named macros yet.</p>
            ) : (
              named.map((entry) => <NamedMacroRow key={JSON.stringify(entry)} entry={entry} />)
            )}
          </div>
        </CardContent>
      </Card>
      <Card size="sm">
        <CardHeader>
          <CardTitle>Temporary Buffers</CardTitle>
          <CardDescription>
            Use <code>{"\\1"}</code> through <code>{"\\9"}</code> to expand these buffers. They
            clear when Chord restarts. <EnterHint />
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-2">
          {Array.from({ length: 9 }, (_, index) => {
            const digit = index + 1;
            const entry = entries.find(
              (candidate) => candidate.target.type === "buffer" && candidate.target.value === digit,
            );
            return (
              <BufferRow key={`${digit}:${entry?.sequence ?? ""}`} digit={digit} entry={entry} />
            );
          })}
        </CardContent>
      </Card>
    </div>
  );
}

function NamedMacroEditor({ entry, onDone }: { entry?: ChordMacroEntry; onDone?: () => void }) {
  const [name, setName] = useState(entry?.target.type === "named" ? entry.target.value : "");
  const [text, setText] = useState(sequenceToText(entry?.sequence ?? ""));
  const save = useMutation({
    mutationFn: () =>
      taurpc.setChordMacro(
        { type: "named", value: name },
        textToSequence(text),
        entry?.target ?? null,
      ),
    onSuccess: () => {
      if (onDone) onDone();
      else {
        setName("");
        setText("");
      }
    },
  });
  return (
    <form
      className="space-y-2"
      onSubmit={(event) => {
        event.preventDefault();
        save.mutate();
      }}
    >
      <div className="grid gap-2 sm:grid-cols-[minmax(0,1fr)_minmax(0,1.5fr)_auto] sm:items-start">
        <label className="min-w-0 space-y-1 text-xs text-muted-foreground">
          <span>Name</span>
          <Input
            value={name}
            onChange={(event) => setName(event.target.value)}
            required
            placeholder="window-size"
            spellCheck={false}
            autoCapitalize="none"
            autoCorrect="off"
            aria-label={entry ? `Name for ${reference(entry.target)}` : "New macro name"}
            disabled={save.isPending}
          />
        </label>
        <label className="min-w-0 space-y-1 text-xs text-muted-foreground">
          <span>Sequence</span>
          <SequenceTextarea
            value={text}
            onValueChange={setText}
            placeholder="1720x864"
            aria-label={entry ? `Sequence for ${reference(entry.target)}` : "New macro sequence"}
            disabled={save.isPending}
          />
        </label>
        {/* Line up with the inputs below their labels while the sequence grows. */}
        <div className="flex items-center gap-2 sm:pt-5">
          {onDone && (
            <Button type="button" variant="outline" onClick={onDone} disabled={save.isPending}>
              Cancel
            </Button>
          )}
          <Button type="submit" disabled={!name || save.isPending}>
            {save.isPending ? "Saving…" : entry ? "Save" : "Add macro"}
          </Button>
        </div>
      </div>
      {name && (
        <p className="text-xs text-muted-foreground">
          <code>{reference({ type: "named", value: name })}</code> ·{" "}
          {/^[0-9]/.test(name)
            ? "Temporary — clears on restart"
            : "Persistent — saved across restarts"}
        </p>
      )}
      {save.isError && <MacroError error={save.error} />}
    </form>
  );
}

function NamedMacroRow({ entry }: { entry: ChordMacroEntry }) {
  const [editing, setEditing] = useState(false);
  const remove = useMutation({ mutationFn: () => taurpc.removeChordMacro(entry.target) });
  return (
    <div className="rounded-lg border bg-background/80 p-3">
      {editing ? (
        <NamedMacroEditor entry={entry} onDone={() => setEditing(false)} />
      ) : (
        <div className="flex items-center gap-3">
          <div className="min-w-0 flex-1 space-y-1">
            <div className="flex flex-wrap items-center gap-2">
              <code className="break-all text-sm font-medium">{reference(entry.target)}</code>
              <Badge variant="outline">{entry.persistent ? "Persistent" : "Temporary"}</Badge>
            </div>
            <SequencePreview sequence={entry.sequence} />
          </div>
          <Button
            type="button"
            size="icon-sm"
            variant="ghost"
            aria-label={`Edit ${reference(entry.target)}`}
            onClick={() => setEditing(true)}
            disabled={remove.isPending}
          >
            <Pencil />
          </Button>
          <Button
            type="button"
            size="icon-sm"
            variant="ghost"
            aria-label={`Delete ${reference(entry.target)}`}
            onClick={() => remove.mutate()}
            disabled={remove.isPending}
            className="text-muted-foreground hover:text-destructive"
          >
            <Trash2 />
          </Button>
        </div>
      )}
      {remove.isError && <MacroError error={remove.error} />}
    </div>
  );
}

function BufferRow({ digit, entry }: { digit: number; entry?: ChordMacroEntry }) {
  const [text, setText] = useState(sequenceToText(entry?.sequence ?? ""));
  const sequence = textToSequence(text);
  const target: ChordMacroName = { type: "buffer", value: digit };
  const save = useMutation({ mutationFn: () => taurpc.setChordMacro(target, sequence, null) });
  const clear = useMutation({ mutationFn: () => taurpc.removeChordMacro(target) });
  const pending = save.isPending || clear.isPending;
  const dirty = sequence !== (entry?.sequence ?? "");
  return (
    <form
      onSubmit={(event) => {
        event.preventDefault();
        save.mutate();
      }}
      className="space-y-1"
    >
      {/* Multiline buffers grow downward; the label and buttons stay on the first line. */}
      <div className="flex items-start gap-2">
        <label
          htmlFor={`macro-buffer-${digit}`}
          className="w-7 shrink-0 font-mono text-sm leading-8"
        >{`\\${digit}`}</label>
        <SequenceTextarea
          id={`macro-buffer-${digit}`}
          aria-label={`Buffer ${digit} sequence`}
          value={text}
          onValueChange={setText}
          placeholder="Empty"
          className="min-w-0 flex-1"
          disabled={pending}
        />
        <div className="flex h-8 shrink-0 items-center gap-2">
          {dirty && (
            <Button type="submit" size="sm" disabled={pending}>
              {save.isPending ? "Saving…" : "Save"}
            </Button>
          )}
          <Button
            type="button"
            variant="ghost"
            size="sm"
            aria-label={`Clear buffer ${digit}`}
            disabled={pending || !entry}
            onClick={() => clear.mutate()}
          >
            Clear
          </Button>
        </div>
      </div>
      {(save.isError || clear.isError) && <MacroError error={save.error ?? clear.error} />}
    </form>
  );
}

function EnterHint() {
  return (
    <>
      Each new line is an Enter key press;{" "}
      <kbd>
        <KeyLabel label="⌘" />
        <KeyLabel label="↩" />
      </kbd>{" "}
      saves.
    </>
  );
}

/**
 * Enter inserts a line break, which is saved as an Enter key press; ⌘Enter
 * submits the form, since Enter no longer does.
 */
function SequenceTextarea({
  value,
  onValueChange,
  className,
  ...props
}: Omit<ComponentProps<typeof Textarea>, "value" | "onChange" | "rows"> & {
  value: string;
  onValueChange: (value: string) => void;
}) {
  return (
    <Textarea
      {...props}
      value={value}
      onChange={(event) => onValueChange(event.target.value)}
      onKeyDown={(event) => {
        if (event.key === "Enter" && event.metaKey) {
          event.preventDefault();
          event.currentTarget.form?.requestSubmit();
        }
      }}
      // Sizes to the content where `field-sizing` is unsupported.
      rows={value.split("\n").length}
      spellCheck={false}
      autoCapitalize="none"
      autoCorrect="off"
      className={`min-h-8 resize-none py-1.25 font-mono ${className ?? ""}`}
    />
  );
}

/** Shows each Enter key press as ↩ at the end of its line, including a trailing one. */
function SequencePreview({ sequence }: { sequence: string }) {
  const lines = sequenceLines(sequence);
  return (
    <p className="whitespace-pre-wrap break-all font-mono text-xs text-muted-foreground">
      {sequence === ""
        ? "(empty sequence)"
        : lines.map((line, index) => (
            <Fragment key={index}>
              {line}
              {index < lines.length - 1 && (
                <>
                  <span title="Enter" className="text-muted-foreground/60">
                    ↩
                  </span>
                  {"\n"}
                </>
              )}
            </Fragment>
          ))}
    </p>
  );
}

/**
 * As in the chord overlay's suggestions: `\name=`, except names that `\1`–`\9` or, in
 * text, the special keys `\n`, `\up`, `\left`, `\down`, and `\right` would claim.
 */
function reference(target: ChordMacroName) {
  if (target.type === "buffer") return `\\${target.value}`;
  return /^(?:[1-9]|n|up|left|down|right)/.test(target.value)
    ? `\\=${target.value}=`
    : `\\${target.value}=`;
}

function MacroError({ error }: { error: unknown }) {
  const message =
    typeof error === "string"
      ? error
      : error instanceof Error
        ? error.message
        : error && typeof error === "object" && "Message" in error
          ? String(error.Message)
          : "Could not update this macro. Try again.";
  return (
    <p role="alert" className="text-xs text-destructive">
      {message}
    </p>
  );
}
