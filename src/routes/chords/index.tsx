import { Kbd } from '#/components/ui/kbd.tsx';
import { useChorderState, useChordPackageManagerState, useFrontmostState } from '#/utils/state.ts';
import { cn } from '#/utils/style.ts';
import { createFileRoute } from '@tanstack/react-router';
import { emit, listen } from '@tauri-apps/api/event';
import getPrettyKey from 'pretty-key';
import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import type { Chord, ChordHint, ChordReference } from '../../types/generated.ts';
import path from 'pathe'

export const Route = createFileRoute('/chords/')({
	component: Chords,
});

const LETTER_TOKENS = Array.from({ length: 26 }, (_, index) =>
	String.fromCharCode('A'.charCodeAt(0) + index));
const MAX_KEY_SIZE = 32;
const NATIVE_SURFACE_RADIUS = 32;
const INDICATOR_TRANSITION_MS = 240;
const HIDDEN_X_OFFSET_PX = 40;
const SHOW_DEVELOPMENT_LABEL = import.meta.env.DEV;
const SINGLE_LETTER_TOKEN_REGEX = /^[A-Z]$/;

function clamp(value: number, min: number, max: number) {
	return Math.min(Math.max(value, min), max);
}

function easeOutCubic(value: number) {
	return 1 - (1 - value) ** 3;
}

function normalizePrettyKey(token: string) {
	if (token === '_') {
		return '-';
	}

	return token;
}

function normalizeToken(token: string) {
	const pretty = normalizePrettyKey(getPrettyKey(token));
	return pretty.length === 1 ? pretty.toUpperCase() : pretty;
}

function parseSequence(sequence: string) {
	return Array.from(sequence).map(normalizeToken);
}

function sortTokens(tokens: Iterable<string>) {
	const tokenSet = new Set(tokens);
	const letterTokens = LETTER_TOKENS.filter(token => tokenSet.has(token));
	const otherTokens = [...tokenSet]
		.filter(token => !SINGLE_LETTER_TOKEN_REGEX.test(token))
		.sort((left, right) => left.localeCompare(right));

	return [...letterTokens, ...otherTokens];
}

function findMatchingBrace(sequence: string, start: number) {
	let depth = 0;

	for (let index = start; index < sequence.length; index += 1) {
		const char = sequence[index];
		if (char === '{') {
			depth += 1;
		}
		else if (char === '}') {
			depth -= 1;
			if (depth === 0) {
				return index;
			}
		}
	}

	return undefined;
}

function expandBraceVariants(inner: string) {
	if (inner.includes(',')) {
		return inner.split(',');
	}

	const rangeParts = inner.split('..');
	if (rangeParts.length !== 2) {
		throw new Error('unsupported brace expression');
	}

	const [start, end] = rangeParts;
	const startNumber = Number.parseInt(start, 10);
	const endNumber = Number.parseInt(end, 10);

	if (Number.isFinite(startNumber) && Number.isFinite(endNumber)) {
		const step = startNumber <= endNumber ? 1 : -1;
		const width = Math.max(start.length, end.length);
		const variants: string[] = [];

		for (let value = startNumber; ; value += step) {
			variants.push(value.toString().padStart(width, '0'));
			if (value === endNumber) {
				break;
			}
		}

		return variants;
	}

	if (start.length !== 1 || end.length !== 1) {
		throw new Error('unsupported brace range');
	}

	const startCode = start.charCodeAt(0);
	const endCode = end.charCodeAt(0);
	const step = startCode <= endCode ? 1 : -1;
	const variants: string[] = [];

	for (let value = startCode; ; value += step) {
		variants.push(String.fromCharCode(value));
		if (value === endCode) {
			break;
		}
	}

	return variants;
}

function expandDescriptionSequence(sequence: string): string[] {
	const start = sequence.indexOf('{');
	if (start === -1) {
		return [sequence];
	}

	const end = findMatchingBrace(sequence, start);
	if (end === undefined) {
		throw new Error('unclosed brace expression');
	}

	const prefix = sequence.slice(0, start);
	const inner = sequence.slice(start + 1, end);
	const suffix = sequence.slice(end + 1);
	const variants = expandBraceVariants(inner);
	const suffixes = expandDescriptionSequence(suffix);

	return variants.flatMap(variant =>
		suffixes.map(suffixValue => `${prefix}${variant}${suffixValue}`),
	);
}

function ChordKeyRow({
	token,
	description = '',
	isSelected = false,
	isDimmed = false,
	keySize,
	descriptionFontSize,
}: {
	token: string
	description?: string,
	isSelected?: boolean
	isDimmed?: boolean
	keySize: number
	descriptionFontSize: number
}) {
	return (
		<div
			className={cn(
				'flex items-center gap-3 transition-all',
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
					'rounded-md border px-0 font-mono shadow-[inset_0_1px_0_rgba(255,255,255,0.35),0_1px_2px_rgba(0,0,0,0.18)]',
					isSelected
						? 'border-emerald-400/90 bg-emerald-100 text-emerald-950 shadow-[inset_0_1px_0_rgba(255,255,255,0.5),0_0_0_1px_rgba(52,211,153,0.35),0_4px_10px_rgba(16,185,129,0.25)]'
						: 'border-border/80 bg-background/95 text-foreground',
				)}
			>
				{token}
			</Kbd>
			<div style={{ fontSize: `${descriptionFontSize}px` }}>{description}</div>
		</div>
	);
}

export function Chords() {
	const state = useChorderState();
	const { frontmostAppBundleId } = useFrontmostState();
  const { packages } = useChordPackageManagerState();

	const [viewportHeight, setViewportHeight] = useState(() => window.innerHeight);
	const [surfaceVersion, setSurfaceVersion] = useState(0);
	const [indicatorProgress, setIndicatorProgress] = useState(() =>
		state.isIndicatorVisible ? 1 : 0,
	);
	const surfaceRef = useRef<HTMLDivElement>(null);
	const animationFrameRef = useRef<number | null>(null);
	const indicatorProgressRef = useRef(indicatorProgress);

	const emitSurfaceRect = () => {
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
			radius: NATIVE_SURFACE_RADIUS,
		});
	};

	useEffect(() => {
		indicatorProgressRef.current = indicatorProgress;
	}, [indicatorProgress]);

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

	useEffect(() => {
		if (animationFrameRef.current !== null) {
			window.cancelAnimationFrame(animationFrameRef.current);
			animationFrameRef.current = null;
		}

		const startProgress = indicatorProgressRef.current;
		const targetProgress = state.isIndicatorVisible ? 1 : 0;
		if (Math.abs(targetProgress - startProgress) < 0.001) {
			animationFrameRef.current = window.requestAnimationFrame(() => {
				indicatorProgressRef.current = targetProgress;
				setIndicatorProgress(targetProgress);
				animationFrameRef.current = null;
			});
			return;
		}

		const startedAt = performance.now();

		const tick = (now: number) => {
			const elapsed = now - startedAt;
			const t = clamp(elapsed / INDICATOR_TRANSITION_MS, 0, 1);
			const nextProgress
				= startProgress + (targetProgress - startProgress) * easeOutCubic(t);

			indicatorProgressRef.current = nextProgress;
			setIndicatorProgress(nextProgress);

			if (t < 1) {
				animationFrameRef.current = window.requestAnimationFrame(tick);
			}
			else {
				animationFrameRef.current = null;
			}
		};

		animationFrameRef.current = window.requestAnimationFrame(tick);

		return () => {
			if (animationFrameRef.current !== null) {
				window.cancelAnimationFrame(animationFrameRef.current);
				animationFrameRef.current = null;
			}
		};
	}, [state.isIndicatorVisible]);

	useLayoutEffect(() => {
		if (surfaceVersion === 0) {
			return;
		}

		emitSurfaceRect();
		void emit('chorder-surface-ready');
	}, [surfaceVersion]);

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
	}, [surfaceVersion]);

	const activeAppChords: Chord[] = [];
	const hintsByRawPattern: Record<string, ChordHint> = {};
  const globalChords: ChordReference[] = []

	for (const chordPackage of packages) {
    globalChords.push(...chordPackage.globalChords);

    for (const [relpath, file] of Object.entries(chordPackage.compiledChordsFiles)) {
      const bundleId = relpath.split('/').slice(1, -1).join('.')
      for (const hint of file.chordHints) {
        // bad check for global
        if (hint.rawPattern[0]?.toUpperCase() === hint.rawPattern[0]) {
          hintsByRawPattern[hint.rawPattern] = hint
        }
      }

      if (bundleId === frontmostAppBundleId) {
        for (const hint of file.chordHints) {
          hintsByRawPattern[hint.rawPattern] = hint;
        }

        for (const chord of file.chords) {
          activeAppChords.push(chord)
        }
      }
    }
	}

  const activeChords: Chord[] = [...activeAppChords, ...globalChords.map(c => c.chord)]

	const normalizedBufferTokens = state.keyBuffer.map(normalizeToken);
	const normalizedActiveChordTokens = state.activeChordKeys?.map(normalizeToken) ?? [];

	const shouldHighlightActiveChord
		= state.isShiftPressed
			&& normalizedBufferTokens.length === 0
			&& normalizedActiveChordTokens.length > 0;
	const selectedTokens = shouldHighlightActiveChord
		? normalizedActiveChordTokens
		: normalizedBufferTokens;
	const currentPrefixLength = selectedTokens.length;

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
      const getChordKeys = (chord: Chord) => 'keys' in chord.trigger ? chord.trigger.keys.map(key => getPrettyKey(key)) : []

			const matchingChords = activeChords.filter(chord =>
				prefixTokens.every((token, tokenIndex) => getChordKeys(chord)[tokenIndex] === token),
			);
			const activeTokens = new Set(
				matchingChords
					.map(chord => getChordKeys(chord)[columnIndex])
					.filter((token): token is string => Boolean(token)),
			);

			const rows = sortTokens(activeTokens).map((token) => {
				const sequenceKey = [...prefixTokens, token].join('').toLowerCase()
				const exactChord = matchingChords.find(
					chord => getChordKeys(chord)[columnIndex] === token && getChordKeys(chord).length === columnIndex + 1,
				);

				return {
					token,
					description: hintsByRawPattern[sequenceKey]?.description ?? exactChord?.name ?? '',
				};
			});

			return {
				id: `column-${columnIndex}`,
				rows,
				selectedToken: selectedTokens[columnIndex],
				hasSelection: Boolean(selectedTokens[columnIndex]),
			};
		},
	);

	useLayoutEffect(() => {
		emitSurfaceRect();
	}, [currentPrefixLength, keyColumns.length, keySize, rowGap, descriptionFontSize, indicatorProgress]);

	const hiddenFraction = 1 - indicatorProgress;
	const indicatorTransform = `translateX(calc(-${hiddenFraction * 100}% - ${hiddenFraction * HIDDEN_X_OFFSET_PX}px))`;

	return (
		<div className="relative size-full bg-transparent">
			<div className="absolute left-0 top-1/2 -translate-y-1/2">
				<div
					key={surfaceVersion}
					ref={surfaceRef}
					className={cn(
						'relative isolate overflow-hidden rounded-r-[2rem] rounded-l-none border border-l-0 px-5 py-5 pl-7',
						'border-white/30 bg-white/22 shadow-[18px_20px_60px_rgba(15,23,42,0.18),inset_0_1px_0_rgba(255,255,255,0.42)]',
						'dark:border-white/10 dark:bg-zinc-950/24 dark:shadow-[18px_20px_60px_rgba(0,0,0,0.34),inset_0_1px_0_rgba(255,255,255,0.1)]',
					)}
					style={{
						transform: indicatorTransform,
						opacity: indicatorProgress,
					}}
				>
					<div className="relative flex items-start">
						<div className="flex items-start gap-6">
							{keyColumns.map(column => (
								<div
									key={column.id}
									className="flex flex-col items-start justify-center"
									style={{ gap: `${rowGap}px` }}
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
											isSelected={column.selectedToken === row.token}
											isDimmed={column.hasSelection && column.selectedToken !== row.token}
											keySize={keySize}
											descriptionFontSize={descriptionFontSize}
										/>
									))}
								</div>
							))}
						</div>
					</div>
				</div>
			</div>
		</div>
	);
}
