/// <reference types="vite/client" />

interface TauriFsDirEntry {
	name?: string
	isDirectory?: boolean
	isFile?: boolean
	isSymlink?: boolean
}

interface TauriFsApi {
	exists: (path: string) => Promise<boolean>
	readDir: (path: string) => Promise<TauriFsDirEntry[]>
}

interface Window {
	__TAURI__?: {
		fs?: TauriFsApi
	}
}
