import { defineConfig } from 'vite-plus';

export default defineConfig({
	fmt: {
		ignorePatterns: ['apps/chord-tauri/src-tauri/**'],
	},
	lint: {
		categories: {
			correctness: 'error',
		},
		ignorePatterns: [
			'**/dist/**',
			'**/*.gen.*',
			'apps/chord-tauri/src-tauri/**',
		],
		plugins: ['eslint', 'typescript', 'react', 'import', 'unicorn', 'promise'],
		rules: {
			'@typescript-eslint/array-type': ['error', { default: 'array' }],
			'no-console': ['error', { allow: ['error', 'warn'] }],
			'no-debugger': 'error',
			'no-unused-vars': [
				'error',
				{
					args: 'none',
					varsIgnorePattern: '^_',
				},
			],
			'no-var': 'error',
			'prefer-const': 'error',
			'react-hooks/exhaustive-deps': 'error',
			'react-hooks/rules-of-hooks': 'error',
			'react/react-in-jsx-scope': 'off',
			'react/set-state-in-effect': 'off',
			'unicorn/consistent-function-scoping': 'off',
		},
	},
});
