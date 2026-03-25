import process from 'node:process';
import babel from '@rolldown/plugin-babel';
import tailwindcss from '@tailwindcss/vite';
import { tanstackRouter } from '@tanstack/router-plugin/vite';
import react, { reactCompilerPreset } from '@vitejs/plugin-react';
import { defineConfig } from 'vite-plus';

const host = process.env.TAURI_DEV_HOST;

// https://vitejs.dev/config/
export default defineConfig({
	plugins: [
		react(),
		babel({ presets: [reactCompilerPreset()] }),
		tailwindcss(),
		tanstackRouter({
			target: 'react',
		}),
	],
	lint: { ignorePatterns: ['src-tauri/**'] },
	fmt: { ignorePatterns: ['src-tauri/**'] },

	// Vite options tailored for Tauri development and only applied in `tauri_app dev` or `tauri_app build`
	//
	// 1. prevent vite from obscuring rust errors
	clearScreen: false,

	// 2. tauri_app expects a fixed port, fail if that port is not available
	server: {
		port: 1420,
		strictPort: true,
		host: host || false,
		hmr: host
			? {
					protocol: 'ws',
					host,
					port: 1421,
				}
			: undefined,
		watch: {
			// 3. tell vite to ignore watching `src-tauri_app`
			ignored: ['**/src-tauri_app/**'],
		},
	},
});
