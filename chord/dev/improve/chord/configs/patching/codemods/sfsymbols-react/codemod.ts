/**
 * JSSG codemod that patches `@bradleyhodges/sfsymbols-react` to remove the
 * spurious `next` runtime dependency from `package.json`.
 *
 * The package is a plain React icon wrapper and never imports Next.js, but its
 * manifest incorrectly lists `next` as a dependency. That pulls the entire
 * Next.js tree into Chord's Tauri + Vite app for no reason.
 *
 * Strategy. Find every `"next": …` pair inside the top-level `dependencies`
 * object and delete it together with its trailing comma (or the comma before
 * it when it is the last entry).
 *
 * Idempotency. When `next` is already absent, the transform emits no edit.
 */

import type Json from '@codemod.com/jssg-types/langs/json';
import type { Edit, SgNode, Transform } from '@codemod.com/jssg-types/main';

const codemod: Transform<Json> = async (root) => {
	const rootNode = root.root();
	const edits: Edit[] = [];

	const nextPairs = rootNode.findAll({
		rule: {
			kind: 'pair',
			has: {
				field: 'key',
				pattern: '"next"',
			},
		},
	});

	for (const pair of nextPairs) {
		if (!isInDependencies(pair)) {
			continue;
		}
		edits.push(...removePairWithComma(pair));
	}

	if (edits.length === 0) {
		return null;
	}
	return rootNode.commitEdits(edits);
};

function isInDependencies(pair: SgNode<Json>): boolean {
	let current: SgNode<Json> | null = pair.parent();
	while (current !== null) {
		if (current.kind() === 'pair') {
			const key = current.field('key');
			if (key?.text() === '"dependencies"') {
				return true;
			}
		}
		current = current.parent();
	}
	return false;
}

function removePairWithComma(pair: SgNode<Json>): Edit[] {
	const next = pair.next();
	if (next?.kind() === ',') {
		return [pair.replace(''), next.replace('')];
	}

	const previous = pair.prev();
	if (previous?.kind() === ',') {
		return [previous.replace(''), pair.replace('')];
	}

	return [pair.replace('')];
}

export default codemod;
