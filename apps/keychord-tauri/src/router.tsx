import { createRouter as createTanStackRouter } from "@keychord/com.npmjs.tanstack__react-router";
import { routeTree } from './routeTree.gen';

export function getRouter() {
	return createTanStackRouter({
		routeTree,
		scrollRestoration: true,
		defaultPreload: 'intent',
		defaultPreloadStaleTime: 0,
	});
}

declare module '@tanstack/react-router' {
	interface Register {
		router: ReturnType<typeof getRouter>
	}
}
