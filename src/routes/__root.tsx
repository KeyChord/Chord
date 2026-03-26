import { Toaster } from '#/components/ui/sonner.tsx';
import { MutationCache, QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { createRootRoute, Outlet } from '@tanstack/react-router';
import { toast } from 'sonner';
import '../global.css';

export const Route = createRootRoute({
	component: RootComponent,
});

function getMutationErrorMessage(error: unknown) {
	if (typeof error === 'string' && error.trim()) {
		return error;
	}

	if (error instanceof Error && error.message.trim()) {
		return error.message;
	}

	if (
		error
		&& typeof error === 'object'
		&& 'message' in error
		&& typeof error.message === 'string'
		&& error.message.trim()
	) {
		return error.message;
	}

	return 'Something went wrong.';
}

const queryClient = new QueryClient({
	mutationCache: new MutationCache({
		onError: (error) => {
			toast.error(getMutationErrorMessage(error));
		},
	}),
});

function RootComponent() {
	return (
		<QueryClientProvider client={queryClient}>
			<Outlet />
			<Toaster position="top-right" />
		</QueryClientProvider>
	);
}
