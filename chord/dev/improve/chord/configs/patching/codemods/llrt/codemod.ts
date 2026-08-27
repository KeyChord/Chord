/**
 * JSSG codemod that patches `llrt_core/build.rs` to comment out the
 * `generate_sdk_client_endpoint_map` build-script call.
 *
 * Background. Upstream LLRT's `build.rs` always runs
 * `generate_sdk_client_endpoint_map(&out_dir)?`, which reads `../sdk.cfg` and
 * emits AWS SDK client endpoint metadata into `OUT_DIR`. Chord embeds LLRT as a
 * git dependency for chord scripting only — it does not ship the AWS Lambda SDK
 * bundle, so this step fails or adds unnecessary build work.
 *
 * Strategy. Find every `call_expression` whose callee is the identifier
 * `generate_sdk_client_endpoint_map`, walk up to the enclosing statement (the
 * `expression_statement` or block-level `try_expression` that carries the
 * trailing `?;`), and prefix that statement with `// ` while preserving its
 * leading indentation.
 *
 * Idempotency. Statements that are already line-commented are skipped, so
 * re-running against an already-patched `build.rs` emits no edit.
 *
 * If a future LLRT release renames or removes the call, the transform emits no
 * edit (returns `null`) — treat that as the signal to re-check upstream rather
 * than silently ship a broken patch.
 */

import type Rust from '@codemod.com/jssg-types/langs/rust';
import type { Edit, SgNode, Transform } from '@codemod.com/jssg-types/main';

const TARGET_CALL = 'generate_sdk_client_endpoint_map';
const COMMENTED_STATEMENT_REGEX = /^\s*\/\//;
const LEADING_INDENTATION_REGEX = /^(\s*)/;

const codemod: Transform<Rust> = async (root) => {
	const rootNode = root.root();
	const edits: Edit[] = [];

	const calls = rootNode.findAll({
		rule: {
			kind: 'call_expression',
			pattern: `${TARGET_CALL}($$$ARGS)`,
		},
	});

	for (const call of calls) {
		const statement = enclosingStatement(call);
		if (statement === null) {
			continue;
		}
		if (isCommentedOut(statement)) {
			continue;
		}
		edits.push(statement.replace(commentLine(statement.text())));
	}

	if (edits.length === 0) {
		return null;
	}
	return rootNode.commitEdits(edits);
};

/** Walk upward from `call` to the statement node that should be commented out. */
function enclosingStatement(call: SgNode<Rust>): SgNode<Rust> | null {
	let current: SgNode<Rust> | null = call;
	while (current !== null) {
		const kind = current.kind();
		if (kind === 'expression_statement') {
			return current;
		}
		if (kind === 'try_expression') {
			const parent = current.parent();
			if (parent?.kind() === 'expression_statement') {
				return parent;
			}
			return current;
		}
		current = current.parent();
	}
	return null;
}

/** True when the statement text is already prefixed with a line comment. */
function isCommentedOut(statement: SgNode<Rust>): boolean {
	return COMMENTED_STATEMENT_REGEX.test(statement.text());
}

/** Prefix each line of `text` with `// ` after its existing indentation. */
function commentLine(text: string): string {
	return text.replace(LEADING_INDENTATION_REGEX, '$1// ');
}

export default codemod;
