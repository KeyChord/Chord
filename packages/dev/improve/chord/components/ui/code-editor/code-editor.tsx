import { cn } from '@chord/com.npmjs.utils-cn';
import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands';
import {
	bracketMatching,
	HighlightStyle,
	indentOnInput,
	StreamLanguage,
	syntaxHighlighting,
} from '@codemirror/language';
import { toml } from '@codemirror/legacy-modes/mode/toml';
import { lintGutter, setDiagnostics } from '@codemirror/lint';
import { highlightSelectionMatches, searchKeymap } from '@codemirror/search';
import { Annotation, Compartment, EditorState, Prec } from '@codemirror/state';
import {
	drawSelection,
	EditorView,
	highlightActiveLine,
	highlightActiveLineGutter,
	highlightSpecialChars,
	keymap,
	lineNumbers,
} from '@codemirror/view';
import { tags } from '@lezer/highlight';
import { useEffect, useLayoutEffect, useRef } from 'react';

export interface CodeEditorDiagnostic {
	/** Offsets in UTF-16 code units, as JavaScript string indices; equal for a point. */
	from: number
	to: number
	message: string
	severity?: 'error' | 'warning' | 'info'
}

const languages = {
	toml: StreamLanguage.define(toml),
};

/** Marks document replacements that come from the `value` prop rather than from typing. */
const fromProp = Annotation.define<boolean>();

// Colours come from the window's surface tokens, so the editor reads like a Mac text view: a
// light gutter with readable line numbers and a faint highlight on the current line.
const theme = EditorView.theme({
	'&': {
		height: '100%',
		fontSize: '12.5px',
		color: 'var(--foreground)',
		backgroundColor: 'var(--mac-list, var(--surface, var(--background)))',
		overflow: 'hidden',
	},
	'&.cm-focused': { outline: 'none' },
	'.cm-scroller': {
		fontFamily: 'ui-monospace, "SF Mono", SFMono-Regular, Menlo, monospace',
		lineHeight: '1.65',
	},
	'.cm-content': { padding: '10px 0', caretColor: 'var(--foreground)' },
	'.cm-line': { padding: '0 14px 0 10px' },
	'.cm-gutters': {
		backgroundColor: 'transparent',
		color: 'var(--mac-tertiary, color-mix(in oklab, var(--foreground) 35%, transparent))',
		border: 'none',
		borderRight: '0.5px solid var(--mac-separator, var(--border))',
	},
	'.cm-lineNumbers .cm-gutterElement': { padding: '0 10px 0 14px', minWidth: '36px' },
	'.cm-activeLine': {
		backgroundColor: 'color-mix(in oklab, var(--accent, #0a7aff) 6%, transparent)',
	},
	'.cm-activeLineGutter': {
		backgroundColor: 'transparent',
		color: 'var(--mac-label, var(--foreground))',
	},
	'.cm-selectionBackground, &.cm-focused .cm-selectionBackground': {
		backgroundColor: 'color-mix(in oklab, var(--accent, #0a7aff) 22%, transparent)',
	},
	'.cm-tooltip': {
		border: '0.5px solid var(--border)',
		borderRadius: '8px',
		backgroundColor: 'var(--surface, var(--background))',
		color: 'var(--foreground)',
		boxShadow: '0 6px 20px rgb(0 0 0 / 15%)',
	},
	'.cm-panels': {
		backgroundColor: 'var(--background)',
		color: 'var(--foreground)',
		borderColor: 'var(--mac-separator, var(--border))',
	},
});

const highlightStyle = HighlightStyle.define([
	// Keys, which in a chords file are mostly the key sequences you type.
	{ tag: tags.propertyName, color: 'oklch(0.48 0.17 255)' },
	{ tag: tags.string, color: 'oklch(0.5 0.13 150)' },
	// Table headers such as `[chords]`, and booleans.
	{ tag: tags.atom, color: 'oklch(0.5 0.2 310)', fontWeight: '600' },
	{ tag: tags.number, color: 'oklch(0.55 0.15 50)' },
	{ tag: tags.comment, color: 'var(--muted-foreground)', fontStyle: 'italic' },
]);

export function CodeEditor({
	value,
	onChange,
	onSave,
	language,
	diagnostics,
	readOnly = false,
	className,
	'aria-label': ariaLabel,
}: {
	value: string
	onChange: (value: string) => void
	/** ⌘S; the browser's own save shortcut never fires inside the editor. */
	onSave?: () => void
	language?: keyof typeof languages
	/** Replaces the editor's diagnostics whenever this array changes identity. They are
	 * positions in the document as it is when they are set; edits afterwards move them along. */
	diagnostics?: CodeEditorDiagnostic[]
	readOnly?: boolean
	className?: string
	'aria-label'?: string
}) {
	const hostRef = useRef<HTMLDivElement>(null);
	const viewRef = useRef<EditorView | null>(null);
	const callbacks = useRef({ onChange, onSave });
	const readOnlyCompartment = useRef(new Compartment());
	const initial = useRef({ value, language, readOnly, ariaLabel });

	useLayoutEffect(() => {
		callbacks.current = { onChange, onSave };
	});

	useLayoutEffect(() => {
		const host = hostRef.current;
		if (!host) return;
		const { value, language, readOnly, ariaLabel } = initial.current;
		const view = new EditorView({
			parent: host,
			state: EditorState.create({
				doc: value,
				extensions: [
					Prec.highest(
						keymap.of([
							{
								key: 'Mod-s',
								preventDefault: true,
								run: () => {
									callbacks.current.onSave?.();
									return true;
								},
							},
						]),
					),
					lineNumbers(),
					highlightActiveLineGutter(),
					highlightSpecialChars(),
					history(),
					drawSelection(),
					indentOnInput(),
					bracketMatching(),
					highlightActiveLine(),
					highlightSelectionMatches(),
					lintGutter(),
					// Escape, then Tab, moves focus out of the editor.
					keymap.of([...defaultKeymap, ...historyKeymap, ...searchKeymap, indentWithTab]),
					language ? languages[language] : [],
					syntaxHighlighting(highlightStyle),
					theme,
					readOnlyCompartment.current.of(EditorState.readOnly.of(readOnly)),
					EditorView.contentAttributes.of(ariaLabel ? { 'aria-label': ariaLabel } : {}),
					EditorView.updateListener.of((update) => {
						if (
							update.docChanged
							&& !update.transactions.some(transaction => transaction.annotation(fromProp))
						) {
							callbacks.current.onChange(update.state.doc.toString());
						}
					}),
				],
			}),
		});
		viewRef.current = view;
		return () => {
			view.destroy();
			viewRef.current = null;
		};
	}, []);

	// Revert or a reload replaces the document; typing already matches `value`.
	useLayoutEffect(() => {
		const view = viewRef.current;
		if (view && view.state.doc.toString() !== value) {
			view.dispatch({
				changes: { from: 0, to: view.state.doc.length, insert: value },
				annotations: fromProp.of(true),
			});
		}
	}, [value]);

	useEffect(() => {
		viewRef.current?.dispatch({
			effects: readOnlyCompartment.current.reconfigure(EditorState.readOnly.of(readOnly)),
		});
	}, [readOnly]);

	useEffect(() => {
		const view = viewRef.current;
		if (!view || !diagnostics) return;
		const length = view.state.doc.length;
		const clamp = (offset: number) => Math.max(0, Math.min(offset, length));
		view.dispatch(
			setDiagnostics(
				view.state,
				diagnostics.map(diagnostic => ({
					from: clamp(diagnostic.from),
					to: clamp(Math.max(diagnostic.from, diagnostic.to)),
					message: diagnostic.message,
					severity: diagnostic.severity ?? 'error',
				})),
			),
		);
	}, [diagnostics]);

	return <div ref={hostRef} data-slot="code-editor" className={cn('min-h-0', className)} />;
}
