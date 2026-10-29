import { toast, Toaster } from "@chord/dev.improve.chord.components.ui.toast";
import {
  MutationCache,
  QueryClient,
  QueryClientProvider,
} from "@chord/com.npmjs.tanstack__react-query";
import { Outlet } from "@chord/com.npmjs.tanstack__react-router";

function getMutationErrorMessage(error: unknown) {
  if (typeof error === "string" && error.trim()) {
    return error;
  }

  if (error instanceof Error && error.message.trim()) {
    return error.message;
  }

  if (
    error &&
    typeof error === "object" &&
    "message" in error &&
    typeof error.message === "string" &&
    error.message.trim()
  ) {
    return error.message;
  }

  if (
    error &&
    typeof error === "object" &&
    "Message" in error &&
    typeof error.Message === "string" &&
    error.Message.trim()
  ) {
    return error.Message;
  }

  return "Something went wrong.";
}

const queryClient = new QueryClient({
  mutationCache: new MutationCache({
    onError: (error) => {
      toast.danger(getMutationErrorMessage(error));
    },
  }),
});

export function RootLayout() {
  return (
    <QueryClientProvider client={queryClient}>
      <Outlet />
      <Toaster />
    </QueryClientProvider>
  );
}
