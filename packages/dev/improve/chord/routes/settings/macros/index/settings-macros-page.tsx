import type { ChordMacroEntry, ChordMacroName } from "@chord/dev.improve.chord.lib.typeshare";
import { useMutation } from "@chord/com.npmjs.tanstack__react-query";
import { taurpc } from "@chord/dev.improve.chord.api.taurpc";
import { GroupCard } from "@chord/dev.improve.chord.components.ui.group-card";
import { KeyLabel } from "@chord/dev.improve.chord.components.ui.kbd";
import { StateQueries, useChordMacrosState } from "@chord/dev.improve.chord.lib.state";
import { Braces, Hash, Pencil, Trash2 } from "lucide-react";
import { Fragment, useState, type ComponentProps } from "react";
import { sequenceLines, sequenceToText, textToSequence } from "#macro-sequence-text";

export function SettingsMacrosPage() {
  return (
    <StateQueries queries={[useChordMacrosState()]}>
      {(state) => <MacrosContent entries={state.entries} />}
    </StateQueries>
  );
}

/**
 * Two cards, like the rest of Settings: named macros, each a row you can edit or delete with
 * a form to add one at the end, and the nine temporary buffers as rows of their own.
 */
export function MacrosContent({ entries }: { entries: ChordMacroEntry[] }) {
  const named = entries.filter((entry) => entry.target.type === "named");
  return (
    <div className="mac-page">
      <div className="mac-section">
        <GroupCard
          icon={<Braces size={17} strokeWidth={1.6} />}
          title="Named macros"
          subtitle={
            <>
              Type <code>{"\\name="}</code> while typing a chord to insert one. Names starting with
              a digit clear when Chord restarts.
            </>
          }
          count={named.length}
          listLabel="Named macros"
        >
          {named.map((entry) => (
            <NamedMacroRow key={JSON.stringify(entry)} entry={entry} />
          ))}
          <div className="mac-row mac-form-row" role="listitem">
            <NamedMacroEditor />
          </div>
        </GroupCard>
      </div>
      <div className="mac-section">
        <GroupCard
          icon={<Hash size={17} strokeWidth={1.6} />}
          title="Temporary buffers"
          subtitle={
            <>
              Type <code>{"\\1"}</code> to <code>{"\\9"}</code> to insert one. They clear when Chord
              restarts.
            </>
          }
          listLabel="Temporary buffers"
        >
          {Array.from({ length: 9 }, (_, index) => {
            const digit = index + 1;
            const entry = entries.find(
              (candidate) => candidate.target.type === "buffer" && candidate.target.value === digit,
            );
            return (
              <BufferRow key={`${digit}:${entry?.sequence ?? ""}`} digit={digit} entry={entry} />
            );
          })}
          <p className="mac-row mac-row-muted mac-card-note">
            <EnterHint />
          </p>
        </GroupCard>
      </div>
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
      className="mac-macro-form"
      onSubmit={(event) => {
        event.preventDefault();
        save.mutate();
      }}
    >
      <div className="mac-macro-fields">
        <input
          className="mac-field"
          value={name}
          onChange={(event) => setName(event.target.value)}
          required
          placeholder={entry ? "Name" : "New macro name"}
          spellCheck={false}
          autoCapitalize="none"
          autoCorrect="off"
          aria-label={entry ? `Name for ${reference(entry.target)}` : "New macro name"}
          disabled={save.isPending}
        />
        <SequenceTextarea
          value={text}
          onValueChange={setText}
          placeholder="Text to insert"
          aria-label={entry ? `Sequence for ${reference(entry.target)}` : "New macro sequence"}
          disabled={save.isPending}
        />
        <div className="mac-macro-actions">
          {onDone && (
            <button type="button" className="mac-button" onClick={onDone} disabled={save.isPending}>
              Cancel
            </button>
          )}
          <button
            type="submit"
            className="mac-button"
            data-default
            disabled={!name || save.isPending}
          >
            {save.isPending ? "Saving…" : entry ? "Save" : "Add"}
          </button>
        </div>
      </div>
      {name && (
        <p className="mac-row-subtitle">
          <code>{reference({ type: "named", value: name })}</code> ·{" "}
          {/^[0-9]/.test(name) ? "Clears when Chord restarts" : "Kept across restarts"}
        </p>
      )}
      {save.isError && <MacroError error={save.error} />}
    </form>
  );
}

function NamedMacroRow({ entry }: { entry: ChordMacroEntry }) {
  const [editing, setEditing] = useState(false);
  const remove = useMutation({ mutationFn: () => taurpc.removeChordMacro(entry.target) });
  if (editing) {
    return (
      <div className="mac-row mac-form-row" role="listitem">
        <NamedMacroEditor entry={entry} onDone={() => setEditing(false)} />
      </div>
    );
  }
  return (
    <div className="mac-row" role="listitem">
      <span className="mac-row-text">
        <code className="mac-row-title font-mono">{reference(entry.target)}</code>
        <SequencePreview sequence={entry.sequence} />
        {remove.isError && <MacroError error={remove.error} />}
      </span>
      <span className="mac-row-detail">{entry.persistent ? "Kept" : "Until restart"}</span>
      <button
        type="button"
        className="mac-button"
        data-icon
        data-borderless
        aria-label={`Edit ${reference(entry.target)}`}
        title="Edit"
        onClick={() => setEditing(true)}
        disabled={remove.isPending}
      >
        <Pencil size={14} aria-hidden="true" />
      </button>
      <button
        type="button"
        className="mac-button"
        data-icon
        data-borderless
        aria-label={`Delete ${reference(entry.target)}`}
        title="Delete"
        onClick={() => remove.mutate()}
        disabled={remove.isPending}
      >
        <Trash2 size={14} aria-hidden="true" />
      </button>
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
      role="listitem"
      className="mac-row mac-buffer-row"
      onSubmit={(event) => {
        event.preventDefault();
        save.mutate();
      }}
    >
      {/* Multiline buffers grow downward; the label and buttons stay on the first line. */}
      <label htmlFor={`macro-buffer-${digit}`} className="mac-buffer-label">{`\\${digit}`}</label>
      <div className="min-w-0 flex-1">
        <SequenceTextarea
          id={`macro-buffer-${digit}`}
          aria-label={`Buffer ${digit} sequence`}
          value={text}
          onValueChange={setText}
          placeholder="Empty"
          disabled={pending}
        />
        {(save.isError || clear.isError) && <MacroError error={save.error ?? clear.error} />}
      </div>
      {dirty && (
        <button type="submit" className="mac-button" data-default disabled={pending}>
          {save.isPending ? "Saving…" : "Save"}
        </button>
      )}
      <button
        type="button"
        className="mac-button"
        data-borderless
        aria-label={`Clear buffer ${digit}`}
        disabled={pending || !entry}
        onClick={() => clear.mutate()}
      >
        Clear
      </button>
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
}: Omit<ComponentProps<"textarea">, "value" | "onChange" | "rows"> & {
  value: string;
  onValueChange: (value: string) => void;
}) {
  return (
    <textarea
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
      className={`mac-field mac-textarea ${className ?? ""}`}
    />
  );
}

/** Shows each Enter key press as ↩ at the end of its line, including a trailing one. */
function SequencePreview({ sequence }: { sequence: string }) {
  const lines = sequenceLines(sequence);
  return (
    <p className="mac-row-subtitle whitespace-pre-wrap break-all font-mono">
      {sequence === ""
        ? "(empty sequence)"
        : lines.map((line, index) => (
            <Fragment key={index}>
              {line}
              {index < lines.length - 1 && (
                <>
                  <span title="Enter" className="opacity-60">
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
    <p role="alert" className="mac-row-subtitle text-destructive">
      {message}
    </p>
  );
}
