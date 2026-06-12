import antfu from '@antfu/eslint-config';

export default antfu(
	{
		type: 'app',
		formatters: true,
		react: true,
		ignores: [
			'**/target',
			'**/src-tauri',
			'**/*.gen.ts',
		],
		stylistic: {
			indent: 'tab',
		},
		rules: {
			'style/semi': ['error', 'always'],
			'antfu/no-top-level-await': 'off',
			'react-refresh/only-export-components': 'off',
		},
	},
);
