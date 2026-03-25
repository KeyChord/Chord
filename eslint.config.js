import antfu from '@antfu/eslint-config';

export default antfu(
	{
		type: 'app',
		formatters: true,
		react: true,
		ignores: [
			'**/target',
			'**/src-tauri',
		],
		stylistic: {
			indent: 'tab',
		},
	},
	{
		rules: {
			'style/semi': ['error', 'always'],
		},
	},
);
