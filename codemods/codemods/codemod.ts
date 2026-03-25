import type { Transform } from 'codemod:ast-grep';
import type Rust from 'codemod:ast-grep/langs/rust';

// TODO: use this codemod to dynamically apply patch files
const transform: Transform<Rust> = async (root) => {
	const rootNode = root.root();

	const nodes = rootNode.findAll({
		rule: {
			pattern: 'generate_sdk_client_endpoint_map($$$ARGS);',
		},
	});

	if (nodes.length === 0) {
		return rootNode.text();
	}

	const edits = nodes.map(node => node.replace(node.text().replace(/^(\s*)/, '$1// ')));

	return rootNode.commitEdits(edits);
};

export default transform;
