import type { Chord, ChordHint, ChordReference } from '@chord/dev.improve.chord.lib.typeshare';
import { cn } from '@chord/com.npmjs.utils-cn';
import { Kbd } from '@chord/dev.improve.chord.components.ui.kbd';
import { StateQueries, useChordExecutionState, useChordInputState, useChordMacrosState, useChordPackageManagerState, useChordPanelState, useFrontmostState } from '@chord/dev.improve.chord.lib.state';
import { emit, listen } from '@tauri-apps/api/event';
import { blockDisplayEnds, getCommandLabel, getCommandPrefix, getRangeHint, isDeletionOf, normalizeChordTokens, normalizeToken, withQuoteTexts } from '#chord-tokens';
import { getEscapeSuggestions } from '#chord-suggestions';
import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'react';

const LETTER_TOKENS = Array.from({ length: 26 }, (_, index) =>
	String.fromCharCode('A'.charCodeAt(0) + index));
const MAX_KEY_SIZE = 32;
const NATIVE_SURFACE_RADIUS = 32;
const SHOW_DEVELOPMENT_LABEL = import.meta.env.DEV;
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
	return cn('border-sky-400/90 bg-sky-100 text-sky-950', block === 'open' && 'border-dashed bg-sky-50');
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
				isDimmed ? 'opacity-35' : 'opacity-100',
				'text-foreground/95',
			)}
		>
			<Kbd
				style={{
					height: `${keySize}px`,
					minWidth: `${keySize}px`,
					fontSize: `${Math.max(12, Math.round(keySize * 0.48))}px`,
				}}
				className={cn(
					'shrink-0 rounded-md border font-mono shadow-[inset_0_1px_0_rgba(255,255,255,0.35),0_1px_2px_rgba(0,0,0,0.18)]',
					token.length > 1 ? 'px-2' : 'px-0',
					isSelected
						? block
							? blockKeyClassName(block)
							: isFailed
								? 'border-red-400/90 bg-red-100 text-red-950 shadow-[inset_0_1px_0_rgba(255,255,255,0.5),0_0_0_1px_rgba(248,113,113,0.35),0_4px_10px_rgba(239,68,68,0.25)]'
								: 'border-emerald-400/90 bg-emerald-100 text-emerald-950 shadow-[inset_0_1px_0_rgba(255,255,255,0.5),0_0_0_1px_rgba(52,211,153,0.35),0_4px_10px_rgba(16,185,129,0.25)]'
						: 'border-border/80 bg-background/95 text-foreground',
				)}
			>
				{token}
			</Kbd>
			{error
				? <div role="alert" className="line-clamp-2 max-w-80 text-red-600 dark:text-red-400" style={{ fontSize: `${descriptionFontSize}px` }}>{error}</div>
				: <div style={{ fontSize: `${descriptionFontSize}px` }}>{description}</div>}
		</div>
	);
}

export function ChordsPage() {
	return (
		<StateQueries queries={[useChordInputState(), useChordPanelState(), useFrontmostState(), useChordPackageManagerState(), useChordExecutionState(), useChordMacrosState()]} quiet>
			{(chordInputData, chordPanelData, frontmostData, chordPackageManagerData, chordExecutionData, chordMacrosData) => (
				<ChordsPageContent chordInputData={chordInputData} chordPanelData={chordPanelData} frontmostData={frontmostData} chordPackageManagerData={chordPackageManagerData} chordExecutionData={chordExecutionData} chordMacrosData={chordMacrosData} />
			)}
		</StateQueries>
	);
}

export function ChordsPageContent({
	chordInputData,
	chordPanelData,
	frontmostData,
	chordPackageManagerData,
	chordExecutionData,
	chordMacrosData,
}: {
	chordInputData: NonNullable<ReturnType<typeof useChordInputState>['data']>
	chordPanelData: NonNullable<ReturnType<typeof useChordPanelState>['data']>
	frontmostData: NonNullable<ReturnType<typeof useFrontmostState>['data']>
	chordPackageManagerData: NonNullable<ReturnType<typeof useChordPackageManagerState>['data']>
	chordExecutionData: NonNullable<ReturnType<typeof useChordExecutionState>['data']>
	chordMacrosData: NonNullable<ReturnType<typeof useChordMacrosState>['data']>
}) {
	const chordInputState = chordInputData;
	const chordPanelState = chordPanelData;
	const { frontmostAppBundleId, filePickerActive } = frontmostData;
	const { packages } = chordPackageManagerData;
	const { loops, metaCommands, haltLoopTemplate } = chordExecutionData;
	const isFloating = chordPanelState.layout === 'floating-bottom';

	const [viewportHeight, setViewportHeight] = useState(() => window.innerHeight);
	const [surfaceVersion, setSurfaceVersion] = useState(0);
	const surfaceRef = useRef<HTMLDivElement>(null);
	const floatingBarRef = useRef<HTMLDivElement>(null);
	const floatingKeysRef = useRef<HTMLDivElement>(null);
	/** The widest the floating bar has been while the current buffer was typed. */
	const floatingWidthFloorRef = useRef<{ tokens: string[], width: number } | null>(null);

	const emitSurfaceRect = useCallback(() => {
		const surface = surfaceRef.current;
		if (!surface) {
			return;
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
	}, [isFloating]);

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

		for (const [relpath, file] of Object.entries(chordPackage.compiledChordsFiles)) {
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
	const baseHintSequences = Object.values(hintsByRawPattern).flatMap(hint =>
		'keys' in hint.pattern
			? [{ tokens: hint.pattern.keys.map(normalizeToken), description: hint.description }]
			: 'range' in hint.pattern
				? [{ tokens: hint.pattern.range.prefix.map(normalizeToken), description: '' }]
				: [],
	);

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
	// Typed quotes keep their text beside the keys; show it in place.
	const floatingDisplayTokens = withQuoteTexts(floatingTokens, floatingQuotes, showsBuffer && chordInputState.inserting);
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
	const commandLabel = getCommandLabel(commandPrefix);
	const builtinHints = [
		...metaCommands,
		...loops.map(loop => ({ sequence: haltLoopTemplate.replace('{id}', String(loop.id)), name: `Halt loop ${loop.id}: ${loop.input}` })),
	].map(hint => ({ tokens: [...hint.sequence].map(normalizeToken), description: hint.name }));
	const hintSequences = [
		...baseHintSequences.map(hint => ({ ...hint, tokens: [...commandPrefixTokens, ...hint.tokens] })),
		...(commandPrefixTokens.length === 0 ? builtinHints : []),
		...(commandPrefixTokens.length === 0 ? [{ tokens: ['1'], description: 'Loop chord' }] : []),
	];
	const rangeHints = baseRangeHints.flatMap(hint => getRangeHint(hint, commandPrefix) ?? []);
	const floatingHint = hintSequences.findLast(hint =>
		hint.tokens.length === floatingTokens.length
		&& hint.tokens.every((token, index) => floatingTokens[index] === token));
	const floatingDescription = chordInputState.description
		?? floatingHint?.description
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

	const keyColumns = Array.from(
		{
			length: shouldHighlightActiveChord
				? Math.max(1, currentPrefixLength)
				: Math.max(1, currentPrefixLength + 1),
		},
		(_, columnIndex) => {
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
			if (selectedToken) {
				activeTokens.add(selectedToken);
			}
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
			for (const range of matchingRanges) {
				for (const token of range.keys) activeTokens.delete(token);
			}

			const rows = sortTokens(activeTokens).map(token => {
				const sequenceKey = [...prefixTokens, token].join('').toLowerCase();
				const exactHint = matchingHints.findLast(
					hint => hint.tokens[columnIndex] === token && hint.tokens.length === columnIndex + 1,
				);
				const exactChord = matchingChords.find(
					chord => getChordKeys(chord)[columnIndex] === token && getChordKeys(chord).length === columnIndex + 1,
				);

				return {
					token,
					keys: [token],
					description: exactHint?.description ?? hintsByRawPattern[sequenceKey]?.description ?? exactChord?.name ?? '',
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
			const tokenOrder = sortTokens(rows.map(row => row.token));
			rows.sort((left, right) => tokenOrder.indexOf(left.token) - tokenOrder.indexOf(right.token));

			return {
				id: `column-${columnIndex}`,
				rows,
				selectedToken,
				hasSelection: Boolean(selectedToken),
			};
		},
	);

	// Typing never narrows the floating bar, even when the description shortens;
	// it may shrink again after Backspace or once the buffer clears.
	useLayoutEffect(() => {
		const bar = floatingBarRef.current;
		const surface = surfaceRef.current;
		if (!bar || !surface) {
			return;
		}

		const typedTokens = isFloating && showsBuffer && normalizedBufferTokens.length > 0 ? floatingDisplayTokens : null;
		const floor = floatingWidthFloorRef.current;
		bar.style.minWidth = typedTokens && floor && !isDeletionOf(typedTokens, floor.tokens)
			? `min(${floor.width}px, 100% - 1.5rem)`
			: '';
		floatingWidthFloorRef.current = typedTokens && { tokens: typedTokens, width: surface.getBoundingClientRect().width };
	});

	useLayoutEffect(() => {
		emitSurfaceRect();
	}, [currentPrefixLength, keyColumns.length, keySize, rowGap, descriptionFontSize, chordPanelState.isVisible, emitSurfaceRect]);

	useLayoutEffect(() => {
		const keys = floatingKeysRef.current;
		if (keys) {
			keys.scrollLeft = keys.scrollWidth;
		}
	}, [chordInputState.input, chordInputState.quotes, chordInputState.selectedInputEvent, isFloating]);

	return (
		<div className="relative size-full bg-transparent">
			<div ref={floatingBarRef} className={isFloating ? 'absolute bottom-3 left-1/2 w-max min-w-40 max-w-[calc(100%_-_1.5rem)] -translate-x-1/2' : 'absolute left-0 top-1/2 -translate-y-1/2'}>
				<div
					key={surfaceVersion}
					ref={surfaceRef}
					className={cn(
						'relative isolate overflow-hidden border border-white/30 bg-white/22 dark:border-white/10 dark:bg-zinc-950/24',
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
													'h-7 min-w-7 shrink-0 rounded-md border px-1.5 font-mono text-sm shadow-sm',
													block
														? blockKeyClassName(block)
														: showsBuffer
															? 'border-border/80 bg-background/95 text-foreground'
															: executionError
																? 'border-red-400/90 bg-red-100 text-red-950'
																: 'border-emerald-400/90 bg-emerald-100 text-emerald-950',
													// Separates adjacent blocks, and blocks from the keys typed after them.
													index + 1 < floatingDisplayTokens.length && floatingBlockEnds.includes(index + 1) && 'mr-1.5',
												)}
											>
											{insertText !== undefined && index === floatingDisplayTokens.length - 1 ? <InsertText text={token} /> : token}
											</Kbd>
										);
									}) : (
									<Kbd
										aria-label="No key pressed"
										className="h-7 min-w-7 shrink-0 rounded-md border border-border/80 bg-background/95 px-1.5 font-mono text-sm text-foreground shadow-sm"
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
						{commandLabel && (
							<div className="mb-3 text-xs text-foreground/70">
								{commandLabel}
							</div>
						)}
						{loops.length > 0 && (
							<div className="mb-3 flex max-h-24 flex-col gap-1 overflow-y-auto text-xs text-foreground/80">
								{loops.map(loop => (
									<div key={loop.id}>Loop {loop.id}: <code>{loop.input}</code> · Halt: <code>{haltLoopTemplate.replace('{id}', String(loop.id))}</code></div>
								))}
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
					<div className="relative flex items-start">
						<div className="flex items-start gap-6">
							{keyColumns.map((column, columnIndex) => (
								<div
									key={column.id}
									className={cn('flex flex-col items-start overflow-y-auto px-1 py-1', columnIndex > 0 && selectedBlockEnds.includes(columnIndex) && 'ml-3')}
									style={{ gap: `${rowGap}px`, maxHeight: `${availableHeight}px` }}
								>
									{SHOW_DEVELOPMENT_LABEL && column.id === 'column-0'
										? (
												<div className="-mb-1 text-[10px] font-semibold tracking-[0.28em] text-foreground/55">
													DEVELOPMENT
												</div>
											)
										: null}
									{column.rows.map(row => (
										<ChordKeyRow
											key={`${column.id}-${row.token}`}
											token={row.token}
											description={row.description}
											error={columnIndex === keyColumns.length - 1 && row.keys.includes(column.selectedToken) ? executionError : undefined}
											isSelected={row.keys.includes(column.selectedToken)}
											isFailed={executionError !== undefined}
											isDimmed={column.hasSelection && !row.keys.includes(column.selectedToken)}
											block={getColumnBlock(columnIndex)}
											keySize={keySize}
											descriptionFontSize={descriptionFontSize}
										/>
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
