import antfu from '@antfu/eslint-config';

export default antfu(
	{
		formatters: true,
		react: true,
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
