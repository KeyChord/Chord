import type { Chord, ChordExecutionState, ChordHint, ChordInputState, ChordMacrosState, ChordPanelState, ChordReference, FrontmostState, HintPanelState } from '@chord/dev.improve.chord.lib.typeshare';
import { cn } from '@chord/com.npmjs.utils-cn';
import { Kbd, KeyLabel } from '@chord/dev.improve.chord.components.ui.kbd';
import { useHintPanelState } from '@chord/dev.improve.chord.lib.state/hint-panel';
import { emit, listen } from '@tauri-apps/api/event';
import { blockDisplayEnds, getCommandPrefix, getRangeHint, isDeletionOf, normalizeChordTokens, normalizeToken, withQuoteTexts } from '#chord-tokens';
import { getEscapeSuggestions } from '#chord-suggestions';
import { type ReactNode, useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react';

const LETTER_TOKENS = Array.from({ length: 26 }, (_, index) =>
	String.fromCharCode('A'.charCodeAt(0) + index));
const MAX_KEY_SIZE = 32;
const NATIVE_SURFACE_RADIUS = 32;
/** The widest the sidebar card gets, as `INDICATOR_WIDTH` in `chord_mode_panel.rs`. */
const SIDEBAR_MAX_WIDTH = 960;
/** Descriptions past this width end in an ellipsis, so wrapped columns stay even. */
const DESCRIPTION_MAX_WIDTH = 240;
/** Columns the sidebar shows in full; earlier keys collapse into the trail above them. */
const MAX_FULL_COLUMNS = 3;
/** The most columns one long list of options wraps into before the rest is counted. */
const MAX_WRAPPED_COLUMNS = 3;
/** Loops listed above the keys; the rest are counted. */
const MAX_LISTED_LOOPS = 3;
// How far each card's box-shadow reaches past its edge: offset plus radius. Chord sizes
// the panel's window to the card and that reach, since WebKit keeps pixels for the
// whole window, transparent or not. (Tailwind reads comments, so no class names here.)
const SIDEBAR_SHADOW_REACH = 18 + 60; // the sidebar card's x offset plus radius, to the right
const FLOATING_SHADOW_REACH = 12 - 4; // the floating card's radius less y offset, above
const SINGLE_LETTER_TOKEN_REGEX = /^[A-Z]$/;
const MAX_ESCAPE_SUGGESTIONS = 10;

function ChordEscapeSuggestions({ result }: { result: NonNullable<ReturnType<typeof getEscapeSuggestions>> }) {
	const visible = result.suggestions.slice(0, MAX_ESCAPE_SUGGESTIONS);
	return (
		<div className="w-full min-w-72 max-w-lg border-t border-foreground/10 pt-3 text-left" aria-live="polite">
			<p className="mb-2 text-xs font-medium text-foreground/60">Suggestions{result.active && ' · Tab inserts the highlighted one'}</p>
			{visible.length > 0 ? <ul aria-label="Escape suggestions" className="space-y-2">
				{visible.map(suggestion => {
					const isActive = suggestion.reference === result.active;
					return <li key={suggestion.reference} aria-current={isActive || undefined} className="flex min-w-0 items-baseline gap-3 text-sm">
						<code className={cn('max-w-64 shrink-0 truncate rounded-md border px-2 py-0.5', isActive ? 'border-foreground/60 bg-background font-semibold' : 'border-border/70 bg-background/80')} title={suggestion.reference}>{suggestion.reference}</code>
						<span className={cn('min-w-0 truncate text-foreground/75', suggestion.kind === 'macro' && 'font-mono')} title={suggestion.description}>{suggestion.description}</span>
						{isActive && <Kbd className="ml-auto shrink-0 rounded border border-border/80 px-1 text-[10px] text-foreground/70">Tab</Kbd>}
					</li>;
				})}
			</ul> : <p className="text-sm text-foreground/60">No matching macros or special keys.</p>}
			{result.suggestions.length > visible.length && <p className="mt-2 text-xs text-foreground/60">{result.suggestions.length - visible.length} more · Keep typing to narrow the list</p>}
		</div>
	);
}

/** The open quote's text with a caret; whitespace stays visible. */
function InsertText({ text }: { text: string }) {
	return (
		<span className="whitespace-pre-wrap break-all">
			{text}
			<span aria-hidden className="ml-px inline-block h-[1em] w-px translate-y-[0.15em] animate-pulse bg-current" />
		</span>
	);
}

function clamp(value: number, min: number, max: number) {
	return Math.min(Math.max(value, min), max);
}

function sortTokens(tokens: Iterable<string>) {
	const tokenSet = new Set(tokens);
	const letterTokens = LETTER_TOKENS.filter(token => tokenSet.has(token));
	const otherTokens = [...tokenSet]
		.filter(token => !SINGLE_LETTER_TOKEN_REGEX.test(token))
		.sort((left, right) => left.localeCompare(right));

	return [...letterTokens, ...otherTokens];
}

/** Keys typed with Shift held, which stay after every run until Backspace removes them. */
type BlockState = 'solid' | 'open';

/** Class names for a key in a block; an open block is still being typed with Shift held. */
function blockKeyClassName(block: BlockState) {
	return cn(
		'border-sky-400/90 bg-sky-100 text-sky-950 dark:border-sky-300/80 dark:bg-sky-500/40 dark:text-white',
		block === 'open' && 'border-dashed bg-sky-50 dark:bg-sky-500/10',
	);
}

const IDLE_KEY_CLASS_NAME = 'border-black/15 bg-white text-neutral-900 dark:border-white/25 dark:bg-white/[0.16] dark:text-white';
const RAN_KEY_CLASS_NAME = 'border-emerald-400/90 bg-emerald-100 text-emerald-950 dark:border-emerald-300/80 dark:bg-emerald-500/45 dark:text-white';
const FAILED_KEY_CLASS_NAME = 'border-red-400/90 bg-red-100 text-red-950 dark:border-red-300/80 dark:bg-red-500/45 dark:text-white';

/**
 * Hides the part of a list that does not fit behind a fade at that edge, the way macOS
 * lists end, instead of a scroll bar the panel could never be scrolled with.
 */
function fadeMask(start: boolean, end: boolean, direction: 'bottom' | 'right') {
	if (!start && !end) {
		return undefined;
	}
	const image = `linear-gradient(to ${direction}, ${start ? 'transparent, black 2.5rem' : 'black'}, ${end ? 'black calc(100% - 2.5rem), transparent' : 'black'})`;
	return { maskImage: image, WebkitMaskImage: image };
}

/** Splits `items` into at most `maxColumns` columns of about the same length, filled top to bottom. */
function wrapIntoColumns<T>(items: T[], rowsPerColumn: number, maxColumns: number) {
	const columnCount = clamp(Math.ceil(items.length / rowsPerColumn), 1, maxColumns);
	const perColumn = Math.ceil(items.length / columnCount);
	return Array.from({ length: columnCount }, (_, index) => items.slice(index * perColumn, (index + 1) * perColumn))
		.filter(column => column.length > 0);
}

function ChordKeyRow({
	token,
	description = '',
	error,
	isSelected = false,
	isFailed = false,
	isDimmed = false,
	block,
	keySize,
	descriptionFontSize,
}: {
	token: string
	description?: string
	/** Shown in place of the description. */
	error?: string
	isSelected?: boolean
	/** Selected keys belong to input that failed to execute. */
	isFailed?: boolean
	isDimmed?: boolean
	/** The selected key belongs to a block. */
	block?: BlockState
	keySize: number
	descriptionFontSize: number
}) {
	return (
		<div
			className={cn(
				'flex shrink-0 items-center gap-3',
				isDimmed ? 'opacity-40 dark:opacity-50' : 'opacity-100',
				'text-foreground',
			)}
		>
			<Kbd
				style={{
					height: `${keySize}px`,
					minWidth: `${keySize}px`,
					fontSize: `${Math.max(12, Math.round(keySize * 0.48))}px`,
				}}
				className={cn(
					'shrink-0 rounded-md border shadow-[inset_0_1px_0_rgba(255,255,255,0.35),0_1px_2px_rgba(0,0,0,0.18)] dark:shadow-[inset_0_1px_0_rgba(255,255,255,0.08),0_1px_2px_rgba(0,0,0,0.4)]',
					token.length > 1 ? 'px-2' : 'px-0',
					isSelected
						? block
							? blockKeyClassName(block)
							: isFailed
								? cn(FAILED_KEY_CLASS_NAME, 'shadow-[inset_0_1px_0_rgba(255,255,255,0.5),0_0_0_1px_rgba(248,113,113,0.35),0_4px_10px_rgba(239,68,68,0.25)]')
								: cn(RAN_KEY_CLASS_NAME, 'shadow-[inset_0_1px_0_rgba(255,255,255,0.5),0_0_0_1px_rgba(52,211,153,0.35),0_4px_10px_rgba(16,185,129,0.25)]')
						: IDLE_KEY_CLASS_NAME,
				)}
			>
				<KeyLabel label={token} />
			</Kbd>
			{error
				? <div role="alert" className="line-clamp-2 max-w-80 text-red-600 dark:text-red-400" style={{ fontSize: `${descriptionFontSize}px` }}>{error}</div>
				: <div className="min-w-0 truncate" style={{ fontSize: `${descriptionFontSize}px`, maxWidth: DESCRIPTION_MAX_WIDTH }}>{description}</div>}
		</div>
	);
}

export function ChordsPage() {
	return (
		<HintPanelStates>
			{(chordInputData, chordPanelData, frontmostData, hintPanelData, chordExecutionData, chordMacrosData, defaultGlobalPrefixForPlaceholders) => (
				<ChordsPageContent chordInputData={chordInputData} chordPanelData={chordPanelData} frontmostData={frontmostData} hintPanelData={hintPanelData} chordExecutionData={chordExecutionData} chordMacrosData={chordMacrosData} defaultGlobalPrefixForPlaceholders={defaultGlobalPrefixForPlaceholders} />
			)}
		</HintPanelStates>
	);
}

/**
 * Renders once every state the panel draws from has loaded. The panel takes no clicks, so
 * unlike `StateQueries` it offers no retry; leaving it out also keeps HeroUI's button, and
 * the libraries it brings, out of the panel's page.
 */
function HintPanelStates({ children }: {
	children: (
		chordInput: ChordInputState,
		chordPanel: ChordPanelState,
		frontmost: FrontmostState,
		hintPanel: HintPanelState,
		chordExecution: ChordExecutionState,
		chordMacros: ChordMacrosState,
		defaultGlobalPrefixForPlaceholders: string,
	) => ReactNode
}) {
	const queries = [
		useHintPanelState('chord-input'),
		useHintPanelState('chord-panel'),
		useHintPanelState('frontmost'),
		useHintPanelState('hint-panel'),
		useHintPanelState('chord-execution'),
		useHintPanelState('chord-macros'),
		useHintPanelState('settings'),
	] as const;
	if (queries.some(query => query.isError && query.data === undefined)) {
		return <div role="alert" className="p-4 text-sm text-muted-foreground">Could not load app state.</div>;
	}
	const [chordInput, chordPanel, frontmost, hintPanel, chordExecution, chordMacros, settings] = queries;
	if (!chordInput.data || !chordPanel.data || !frontmost.data || !hintPanel.data || !chordExecution.data || !chordMacros.data || !settings.data) {
		return null;
	}
	return children(chordInput.data, chordPanel.data, frontmost.data, hintPanel.data, chordExecution.data, chordMacros.data, settings.data.defaultGlobalPrefixForPlaceholders);
}

export function ChordsPageContent({
	chordInputData,
	chordPanelData,
	frontmostData,
	hintPanelData,
	chordExecutionData,
	chordMacrosData,
	defaultGlobalPrefixForPlaceholders,
}: {
	chordInputData: ChordInputState
	chordPanelData: ChordPanelState
	frontmostData: FrontmostState
	hintPanelData: HintPanelState
	chordExecutionData: ChordExecutionState
	chordMacrosData: ChordMacrosState
	defaultGlobalPrefixForPlaceholders?: string
}) {
	const chordInputState = chordInputData;
	const chordPanelState = chordPanelData;
	const { overlayAppBundleId } = frontmostData;
	const frontmostAppBundleId = overlayAppBundleId ?? frontmostData.frontmostAppBundleId;
	const filePickerActive = !overlayAppBundleId && frontmostData.filePickerActive;
	const { packages } = hintPanelData;
	const { loops, metaCommands, haltLoopTemplate } = chordExecutionData;
	const isFloating = chordPanelState.layout === 'floating-bottom';

	const [viewportHeight, setViewportHeight] = useState(() => window.innerHeight);
	const [surfaceVersion, setSurfaceVersion] = useState(0);
	const surfaceRef = useRef<HTMLDivElement>(null);
	const floatingBarRef = useRef<HTMLDivElement>(null);
	const floatingKeysRef = useRef<HTMLDivElement>(null);
	/** The widest the floating bar has been while the current buffer was typed. */
	const floatingWidthFloorRef = useRef<{ tokens: string[], width: number } | null>(null);
	const reportedExtentRef = useRef('');

	const emitSurfaceRect = useCallback(() => {
		const surface = surfaceRef.current;
		const bar = floatingBarRef.current;
		if (!surface || !bar) {
			return;
		}

		// The window the card needs: the sidebar card's width from the left edge, and the
		// floating card's height from the bottom, each with its box-shadow's reach. Layout
		// boxes ignore the transforms that slide the card out of view, so a hidden card
		// keeps its room.
		const extent = JSON.stringify({
			layout: chordPanelState.layout,
			width: Math.ceil(bar.offsetLeft + bar.offsetWidth + SIDEBAR_SHADOW_REACH),
			height: Math.ceil(window.innerHeight - bar.offsetTop + FLOATING_SHADOW_REACH),
		});
		if (extent !== reportedExtentRef.current) {
			reportedExtentRef.current = extent;
			void emit('chorder-panel-extent', JSON.parse(extent));
		}

		const rect = surface.getBoundingClientRect();
		void emit('chorder-surface-rect', {
			x: rect.left,
			y: window.innerHeight - rect.bottom,
			width: rect.width,
			height: rect.height,
			radius: isFloating ? 16 : NATIVE_SURFACE_RADIUS,
			flushLeft: !isFloating,
		});
	}, [isFloating, chordPanelState.layout]);

	useEffect(() => {
		const handleResize = () => {
			setViewportHeight(window.innerHeight);
		};

		window.addEventListener('resize', handleResize);
		return () => {
			window.removeEventListener('resize', handleResize);
		};
	}, []);

	useEffect(() => {
		const unlistenPromise = listen('chorder-will-show', () => {
			setSurfaceVersion(version => version + 1);
		});

		return () => {
			void unlistenPromise.then(unlisten => unlisten?.());
		};
	}, []);

	useEffect(() => {
		void emit('chorder-window-ready');
	}, []);

	useLayoutEffect(() => {
		if (surfaceVersion === 0) {
			return;
		}

		emitSurfaceRect();
		void emit('chorder-surface-ready');
	}, [surfaceVersion, emitSurfaceRect]);

	useEffect(() => {
		const surface = surfaceRef.current;
		if (!surface) {
			return;
		}

		const observer = new ResizeObserver(() => {
			emitSurfaceRect();
		});
		observer.observe(surface);

		return () => {
			observer.disconnect();
		};
	}, [surfaceVersion, emitSurfaceRect]);

	const activeAppChords: Chord[] = [];
	const finderChords: Chord[] = [];
	const finderHints: ChordHint[] = [];
	const hintsByRawPattern: Record<string, ChordHint> = {};
	const globalChords: ChordReference[] = [];
	const repeatCountHints = new Set<ChordHint>();

	for (const chordPackage of packages) {
		globalChords.push(...chordPackage.globalChords);

		for (const [relpath, file] of Object.entries(chordPackage.chordsFiles)) {
			const bundleId = relpath.split('/').slice(1, -1).join('.');
			for (const hint of file.chordHints) {
				if (chordPackage.name === '@keychord/chords-chord'
					&& 'range' in hint.pattern
					&& hint.pattern.range.prefix.length === 0
					&& hint.pattern.range.keys.every(key => /^\d$/.test(normalizeToken(key)))) {
					repeatCountHints.add(hint);
				}
				// bad check for global
				if (hint.rawPattern[0]?.toUpperCase() === hint.rawPattern[0]) {
					hintsByRawPattern[hint.rawPattern] = hint;
				}
			}

			if (filePickerActive && bundleId === 'com.apple.finder') {
				finderChords.push(...file.chords);
				finderHints.push(...file.chordHints);
			}

			if (bundleId === frontmostAppBundleId && !(filePickerActive && bundleId === 'com.apple.finder')) {
				for (const hint of file.chordHints) {
					hintsByRawPattern[hint.rawPattern] = hint;
				}

				for (const chord of file.chords) {
					activeAppChords.push(chord);
				}
			}
		}
	}

	// The native resolver visits the picker scope before app and global scopes.
	for (const hint of finderHints.toReversed()) {
		hintsByRawPattern[hint.rawPattern] = hint;
	}
	const activeChords: Chord[] = [...finderChords, ...activeAppChords, ...globalChords.map(c => c.chord)];
	// A range hint offers its prefix as an option with no text of its own. Those
	// entries go first so `findLast` reaches the key's own hint, as with `=` and `="1-9"`.
	const baseHintSequences = Object.values(hintsByRawPattern).flatMap(hint =>
		'keys' in hint.pattern
			? [{ tokens: hint.pattern.keys.map(normalizeToken), description: hint.description }]
			: 'range' in hint.pattern
				? [{ tokens: hint.pattern.range.prefix.map(normalizeToken), description: '' }]
				: [],
	).toSorted((left, right) => Number(Boolean(left.description)) - Number(Boolean(right.description)));

	const baseRangeHints = Object.values(hintsByRawPattern).flatMap(hint =>
		'range' in hint.pattern
			? [
					{
						prefix: hint.pattern.range.prefix.map(normalizeToken),
						keys: hint.pattern.range.keys.map(normalizeToken),
						description: hint.description,
						isRepeatCount: repeatCountHints.has(hint),
					},
				]
			: [],
	);

	const normalizedBufferTokens = normalizeChordTokens(chordInputState.input);
	const escapeSuggestions = getEscapeSuggestions(normalizedBufferTokens, chordMacrosData.entries);
	const normalizedActiveChordTokens = normalizeChordTokens(chordInputState.selectedInputEvent?.input ?? []);
	// Blocks stay in the buffer after a run, so only keys after them count as typing.
	const blockLength = chordInputState.blocks.at(-1) ?? 0;
	const isTyping = normalizedBufferTokens.length > blockLength;
	const startsWithBlocks = (tokens: string[]) => normalizedBufferTokens.slice(0, blockLength).every((token, index) => tokens[index] === token);
	// Failed input stays selected until the next key press, in place of an executed chord.
	const executionError = !isTyping && chordInputState.error
		? chordInputState.error.charAt(0).toUpperCase() + chordInputState.error.slice(1)
		: undefined;

	const shouldHighlightActiveChord
		= executionError !== undefined
			&& normalizedActiveChordTokens.length > 0;
	const selectedTokens = shouldHighlightActiveChord
		? normalizedActiveChordTokens
		: normalizedBufferTokens;
	const showsBuffer = isTyping || !chordInputState.selectedInputEvent;
	const floatingTokens = showsBuffer
		? normalizedBufferTokens
		: normalizedActiveChordTokens;
	const floatingQuotes = showsBuffer ? chordInputState.quotes : chordInputState.selectedInputEvent?.quotes ?? [];
	const floatingBlockEnds = startsWithBlocks(floatingTokens)
		? blockDisplayEnds(floatingTokens, floatingQuotes, chordInputState.blocks)
		: [];
	const getFloatingBlock = (index: number): BlockState | undefined =>
		index < (floatingBlockEnds.at(-1) ?? 0)
			? 'solid'
			: showsBuffer && chordInputState.openBlock && index >= (floatingBlockEnds.at(-1) ?? 0)
				? 'open'
				: undefined;
	const selectedBlockEnds = startsWithBlocks(selectedTokens) ? chordInputState.blocks : [];
	const getColumnBlock = (columnIndex: number): BlockState | undefined =>
		columnIndex < (selectedBlockEnds.at(-1) ?? 0)
			? 'solid'
			: !shouldHighlightActiveChord && chordInputState.openBlock && columnIndex >= blockLength
				? 'open'
				: undefined;
	const insertText = chordInputState.inserting ? chordInputState.quotes.at(-1) ?? '' : undefined;
	const currentPrefixLength = selectedTokens.length;
	const commandPrefix = getCommandPrefix(selectedTokens);
	const commandPrefixTokens = commandPrefix.tokens;
	const defaultPlaceholderToken = defaultGlobalPrefixForPlaceholders
		? normalizeToken(defaultGlobalPrefixForPlaceholders)
		: undefined;
	const builtinHints = [
		...metaCommands,
		...loops.map(loop => ({ sequence: haltLoopTemplate.replace('{id}', String(loop.id)), name: `Halt loop ${loop.id}: ${loop.input}` })),
	].map(hint => ({ tokens: [...hint.sequence].map(normalizeToken), description: hint.name }));
	const hintSequences = [
		...baseHintSequences.map(hint => ({ ...hint, tokens: [...commandPrefixTokens, ...hint.tokens] })),
		...(commandPrefixTokens.length === 0 ? builtinHints : []),
	];
	const rangeHints = baseRangeHints.flatMap(hint => getRangeHint(hint, commandPrefix) ?? []);
	const floatingHint = hintSequences.findLast(hint =>
		hint.tokens.length === floatingTokens.length
		&& hint.tokens.every((token, index) => floatingTokens[index] === token));
	const isFloatingPlaceholderPrefix = floatingTokens.length === commandPrefixTokens.length + 1
		&& floatingTokens.at(-1) === defaultPlaceholderToken;
	const floatingDescription = chordInputState.description
		?? (isFloatingPlaceholderPrefix ? floatingHint?.description || 'More' : floatingHint?.description)
		?? (floatingTokens.length > 0 ? '...' : 'Ready for a chord');

	const maxVisibleRows = 20;
	const availableHeight = Math.max(viewportHeight - 96, 240);
	const idealKeySize = availableHeight / (maxVisibleRows + Math.max(maxVisibleRows - 1, 0) * 0.18);
	const keySize = clamp(Math.floor(idealKeySize), 22, MAX_KEY_SIZE);
	const rowGap = clamp(
		Math.floor((availableHeight - keySize * maxVisibleRows) / Math.max(maxVisibleRows - 1, 1)),
		4,
		10,
	);
	const descriptionFontSize = clamp(Math.round(keySize * 0.42), 11, 16);

	const columnCount = Math.max(1, currentPrefixLength + (shouldHighlightActiveChord ? 0 : 1));
	const typedChordStart = Math.max(blockLength, commandPrefixTokens.length);
	const keyColumns = [];
	for (let columnIndex = 0; columnIndex < columnCount;) {
		const prefixTokens = selectedTokens.slice(0, columnIndex);
		const selectedToken = selectedTokens[columnIndex];
		const getChordKeys = (chord: Chord) => 'keys' in chord.trigger ? [...commandPrefixTokens, ...normalizeChordTokens(chord.trigger.keys)] : [];

		const matchingChords = activeChords.filter(chord =>
			prefixTokens.every((token, tokenIndex) => getChordKeys(chord)[tokenIndex] === token),
		);
		const activeTokens = new Set(
			matchingChords
				.map(chord => getChordKeys(chord)[columnIndex])
				.filter((token): token is string => Boolean(token)),
		);
		// Explicit hints are options even when their handler uses a regex trigger.
		const matchingHints = hintSequences.filter(hint =>
			prefixTokens.every((token, tokenIndex) => hint.tokens[tokenIndex] === token),
		);
		for (const hint of matchingHints) {
			const token = hint.tokens[columnIndex];
			if (token) {
				activeTokens.add(token);
			}
		}

		const matchingRanges = rangeHints.filter(
			hint =>
				hint.prefix.length === columnIndex &&
				prefixTokens.every((token, index) => hint.prefix[index] === token),
		);
		const unmatchedToken = selectedToken && !activeTokens.has(selectedToken)
			&& !matchingRanges.some(range => range.keys.includes(selectedToken))
			? selectedToken
			: undefined;
		let nextColumnIndex = columnIndex + 1;
		// Once a letter has no matching continuation, the rest of its letter run is free text.
		if (isTyping && columnIndex >= typedChordStart && unmatchedToken && SINGLE_LETTER_TOKEN_REGEX.test(unmatchedToken)) {
			while (SINGLE_LETTER_TOKEN_REGEX.test(selectedTokens[nextColumnIndex] ?? '')) nextColumnIndex++;
		}
		const selectedLabel = nextColumnIndex === columnIndex + 1
			? selectedToken
			: selectedTokens.slice(columnIndex, nextColumnIndex).join('');
		if (selectedToken) {
			activeTokens.add(selectedToken);
		}
		for (const range of matchingRanges) {
			for (const token of range.keys) activeTokens.delete(token);
		}

		const rows = [...activeTokens].map(token => {
			const sequenceKey = [...prefixTokens, token].join('').toLowerCase();
			const exactHint = matchingHints.findLast(
				hint => hint.tokens[columnIndex] === token && hint.tokens.length === columnIndex + 1,
			);
			const exactChord = matchingChords.find(
				chord => getChordKeys(chord)[columnIndex] === token && getChordKeys(chord).length === columnIndex + 1,
			);
			const isDefaultPlaceholderPrefix = columnIndex === commandPrefixTokens.length && token === defaultPlaceholderToken;
			const description = exactHint?.description ?? hintsByRawPattern[sequenceKey]?.description ?? exactChord?.name;

			return {
				token,
				keys: [token],
				description: description || (isDefaultPlaceholderPrefix ? exactChord?.name || 'More' : ''),
			};
		});

		for (const range of matchingRanges) {
			const token = range.label;
			// Later range hints take precedence when they use the same label.
			const existingIndex = rows.findIndex(row => row.token === token);
			if (existingIndex !== -1) rows.splice(existingIndex, 1);
			rows.push({
				token,
				keys: range.keys,
				description: range.description,
			});
		}
		const tokenOrder = sortTokens(rows.map(row => row.token).filter(token => token !== unmatchedToken));
		if (unmatchedToken) tokenOrder.unshift(unmatchedToken);
		rows.sort((left, right) => tokenOrder.indexOf(left.token) - tokenOrder.indexOf(right.token));

		keyColumns.push({
			index: columnIndex,
			rows,
			selectedToken,
			selectedLabel,
		});
		columnIndex = nextColumnIndex;
	}

	// The sidebar shows every option without a scroll bar. The current key's options wrap
	// into more columns once they outgrow the panel's height; earlier keys show the options
	// around the one chosen, faded where the list goes on; and keys before those collapse
	// into a trail of keycaps, so a long chord never pushes the columns off the card.
	const headerHeight = loops.length > 0 ? (Math.min(loops.length, MAX_LISTED_LOOPS) + Number(loops.length > MAX_LISTED_LOOPS)) * 20 + 12 : 0;
	const rowsPerColumn = Math.max(4, Math.floor((availableHeight - headerHeight + rowGap) / (keySize + rowGap)));
	const lastColumnWraps = clamp(Math.ceil((keyColumns.at(-1)?.rows.length ?? 0) / rowsPerColumn), 1, MAX_WRAPPED_COLUMNS);
	const fullColumnCount = Math.max(1, MAX_FULL_COLUMNS - (lastColumnWraps - 1));
	const collapsedColumns = keyColumns.slice(0, Math.max(0, keyColumns.length - fullColumnCount));
	const visibleColumns = keyColumns.slice(collapsedColumns.length).map((column, index, columns) => {
		const isLast = index === columns.length - 1;
		const selectedIndex = column.rows.findIndex(row => row.keys.includes(column.selectedToken));
		if (isLast) {
			const capacity = rowsPerColumn * MAX_WRAPPED_COLUMNS;
			let rows = column.rows;
			if (rows.length > capacity) {
				// The last place goes to the count of the rest; the chosen key always shows.
				rows = rows.slice(0, capacity - 1);
				if (selectedIndex >= rows.length) {
					rows[rows.length - 1] = column.rows[selectedIndex];
				}
			}
			return {
				...column,
				isLast,
				wrapped: wrapIntoColumns(rows, rowsPerColumn, MAX_WRAPPED_COLUMNS),
				hiddenCount: column.rows.length - rows.length,
				clippedStart: false,
				clippedEnd: false,
			};
		}
		const start = clamp(selectedIndex - Math.floor(rowsPerColumn / 2), 0, Math.max(0, column.rows.length - rowsPerColumn));
		const rows = column.rows.slice(start, start + rowsPerColumn);
		return {
			...column,
			isLast,
			wrapped: [rows],
			hiddenCount: 0,
			clippedStart: start > 0,
			clippedEnd: start + rows.length < column.rows.length,
		};
	});

	// Typed quotes keep their text beside the keys; show it in place.
	const floatingKeycaps = isTyping ? keyColumns.flatMap(column => column.selectedLabel ? [column.selectedLabel] : []) : floatingTokens;
	const floatingDisplayTokens = withQuoteTexts(floatingKeycaps, floatingQuotes, showsBuffer && chordInputState.inserting);

	// Typing never narrows the floating bar, even when the description shortens;
	// it may shrink again after Backspace or once the buffer clears.
	useLayoutEffect(() => {
		const bar = floatingBarRef.current;
		const surface = surfaceRef.current;
		if (!bar || !surface) {
			return;
		}

		const typedTokens = isFloating && showsBuffer && normalizedBufferTokens.length > 0
			? withQuoteTexts(normalizedBufferTokens, floatingQuotes, chordInputState.inserting)
			: null;
		const floor = floatingWidthFloorRef.current;
		bar.style.minWidth = typedTokens && floor && !isDeletionOf(typedTokens, floor.tokens)
			? `min(${floor.width}px, 100% - 1.5rem)`
			: '';
		floatingWidthFloorRef.current = typedTokens && { tokens: typedTokens, width: surface.getBoundingClientRect().width };
	});

	useLayoutEffect(() => {
		emitSurfaceRect();
	}, [currentPrefixLength, keyColumns.length, keySize, rowGap, descriptionFontSize, chordPanelState.isVisible, emitSurfaceRect]);

	// A long chord keeps its latest keys in view; the start fades out on the left to show
	// there is more before it.
	useLayoutEffect(() => {
		const keys = floatingKeysRef.current;
		if (keys) {
			keys.scrollLeft = keys.scrollWidth;
			const mask = fadeMask(keys.scrollLeft > 0, false, 'right');
			keys.style.maskImage = mask?.maskImage ?? '';
			keys.style.webkitMaskImage = mask?.WebkitMaskImage ?? '';
		}
	});

	return (
		<div className="relative size-full bg-transparent">
			<div ref={floatingBarRef} className={isFloating ? 'absolute bottom-3 left-1/2 w-max min-w-40 max-w-[calc(100%_-_1.5rem)] -translate-x-1/2' : 'absolute left-0 top-1/2 w-max -translate-y-1/2'} style={isFloating ? undefined : { maxWidth: SIDEBAR_MAX_WIDTH }}>
				<div
					key={surfaceVersion}
					ref={surfaceRef}
					className={cn(
						'relative isolate overflow-hidden border border-black/[0.08] bg-white/60 dark:border-white/15 dark:bg-black/55',
						isFloating
							? 'rounded-2xl px-3 py-2.5 shadow-[0_4px_12px_rgba(15,23,42,0.18)] dark:shadow-[0_4px_12px_rgba(0,0,0,0.34)]'
							: 'rounded-r-[2rem] rounded-l-none border-l-0 px-5 py-5 pl-7 shadow-[18px_20px_60px_rgba(15,23,42,0.18),inset_0_1px_0_rgba(255,255,255,0.42)] dark:shadow-[18px_20px_60px_rgba(0,0,0,0.34),inset_0_1px_0_rgba(255,255,255,0.1)]',
					)}
					style={{
						// Move the native vibrancy surface offscreen too when toggled off.
						transform: chordPanelState.isVisible ? undefined : isFloating ? 'translateY(calc(100% + 40px))' : 'translateX(calc(-100% - 40px))',
						opacity: chordPanelState.isVisible ? 1 : 0,
					}}
				>
					{isFloating || escapeSuggestions ? (
						<div className="flex flex-col items-center gap-1.5" aria-live="polite">
							<div ref={floatingKeysRef} className="w-full overflow-x-hidden py-0.5">
								<div className="mx-auto flex w-max min-w-full justify-center gap-1.5">
									{floatingDisplayTokens.length > 0 ? floatingDisplayTokens.map((token, index) => {
										const block = getFloatingBlock(index);
										return (
											<Kbd
												key={`${index}-${token}`}
												className={cn(
													'h-7 min-w-7 shrink-0 rounded-md border px-1.5 text-sm shadow-sm',
													block
														? blockKeyClassName(block)
														: showsBuffer
															? IDLE_KEY_CLASS_NAME
															: executionError
																? FAILED_KEY_CLASS_NAME
																: RAN_KEY_CLASS_NAME,
													// Separates adjacent blocks, and blocks from the keys typed after them.
													index + 1 < floatingDisplayTokens.length && floatingBlockEnds.includes(index + 1) && 'mr-1.5',
												)}
											>
											{insertText !== undefined && index === floatingDisplayTokens.length - 1 ? <InsertText text={token} /> : <KeyLabel label={token} />}
											</Kbd>
										);
									}) : (
									<Kbd
										aria-label="No key pressed"
										className={cn('h-7 min-w-7 shrink-0 rounded-md border px-1.5 text-sm shadow-sm', IDLE_KEY_CLASS_NAME)}
									/>
								)}
								</div>
							</div>
							{executionError
								? <div role="alert" className="line-clamp-2 w-full text-center text-xs leading-4 text-red-600 dark:text-red-400">{executionError}</div>
								: <div className="line-clamp-2 w-full text-center text-xs leading-4 text-foreground/90">{floatingDescription}</div>}
							{escapeSuggestions && <ChordEscapeSuggestions result={escapeSuggestions} />}
						</div>
					) : <>
						{loops.length > 0 && (
							<div className="mb-3 flex max-w-md flex-col gap-1 text-xs text-foreground/80">
								{loops.slice(0, MAX_LISTED_LOOPS).map(loop => (
									<div key={loop.id} className="truncate">Loop {loop.id}: <code>{loop.input}</code> · Halt: <code>{haltLoopTemplate.replace('{id}', String(loop.id))}</code></div>
								))}
								{loops.length > MAX_LISTED_LOOPS && <div className="text-foreground/60">{`${loops.length - MAX_LISTED_LOOPS} more running`}</div>}
							</div>
						)}
					{insertText !== undefined ? (
						<div aria-label="Insert mode" className="flex max-w-96 flex-col gap-2">
							<p className="text-xs font-medium text-foreground/70">Insert mode</p>
							<div className="min-h-8 rounded-md border border-border/80 bg-background/95 px-2 py-1 font-mono text-sm text-foreground shadow-sm">
								<InsertText text={insertText} />
							</div>
							<p className="text-xs text-foreground/60">Caps+Space closes the quote · Esc cancels</p>
						</div>
					) : (
					<div className="relative flex flex-col items-start">
						{collapsedColumns.length > 0 && (
							<div aria-label="Earlier keys" className="mb-3 flex max-w-full items-center gap-1 overflow-hidden text-xs text-foreground/60">
								{collapsedColumns.map(column => {
									const block = getColumnBlock(column.index);
									return (
										<Kbd
											key={column.index}
											className={cn('h-6 min-w-6 shrink-0 rounded-md border px-1 text-xs shadow-sm', block ? blockKeyClassName(block) : IDLE_KEY_CLASS_NAME)}
										>
											<KeyLabel label={column.selectedLabel ?? ''} />
										</Kbd>
									);
								})}
								<span aria-hidden className="ml-0.5 text-foreground/40">›</span>
							</div>
						)}
						<div className="flex items-start gap-6">
							{visibleColumns.map(column => (
								<div
									key={column.index}
									className={cn('flex items-start gap-x-6 px-1 py-1', column.index > 0 && selectedBlockEnds.includes(column.index) && 'ml-3')}
									style={fadeMask(column.clippedStart, column.clippedEnd, 'bottom')}
								>
									{column.wrapped.map((rows, wrappedIndex) => (
										<div key={wrappedIndex} className="flex min-w-0 flex-col items-start" style={{ gap: `${rowGap}px` }}>
											{rows.map(row => (
												<ChordKeyRow
													key={row.token}
													token={row.token === column.selectedToken ? column.selectedLabel : row.token}
													description={row.description}
													error={column.isLast && row.keys.includes(column.selectedToken) ? executionError : undefined}
													isSelected={row.keys.includes(column.selectedToken)}
													isFailed={executionError !== undefined}
													isDimmed={Boolean(column.selectedToken) && !row.keys.includes(column.selectedToken)}
													block={getColumnBlock(column.index)}
													keySize={keySize}
													descriptionFontSize={descriptionFontSize}
												/>
											))}
											{wrappedIndex === column.wrapped.length - 1 && column.hiddenCount > 0 && (
												<div className="text-foreground/60" style={{ fontSize: `${descriptionFontSize}px`, lineHeight: `${keySize}px` }}>
													{`${column.hiddenCount} more · keep typing`}
												</div>
											)}
										</div>
									))}
								</div>
							))}
						</div>
					</div>
					)}
					</>}
				</div>
			</div>
		</div>
	);
}
