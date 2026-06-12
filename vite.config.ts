import { defineConfig } from 'vite-plus';

export default defineConfig({
	fmt: {
		ignorePatterns: ['apps/keychord-tauri/src-tauri/**'],
	},
	lint: {
		ignorePatterns: ['apps/keychord-tauri/src-tauri/**'],
	},
});
