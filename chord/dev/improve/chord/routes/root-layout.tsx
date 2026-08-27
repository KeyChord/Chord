import { toast } from '@chord/com.npmjs.sonner';
import { MutationCache, QueryClient, QueryClientProvider } from '@chord/com.npmjs.tanstack__react-query';
import { Outlet } from '@chord/com.npmjs.tanstack__react-router';
import { Toaster } from '@chord/dev.improve.chord.components.ui.sonner';

function getMutationErrorMessage(error: unknown) {
	console.error(error);

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

	if (
		error
		&& typeof error === 'object'
		&& 'Message' in error
		&& typeof error.Message === 'string'
		&& error.Message.trim()
	) {
		return error.Message;
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

export function RootLayout() {
	return (
		<QueryClientProvider client={queryClient}>
			<Outlet />
			<Toaster position="top-right" />
		</QueryClientProvider>
	);
}
