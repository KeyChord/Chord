import { createFileRoute } from "@tanstack/react-router";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { redirect } from "@tanstack/react-router";

export const Route = createFileRoute("/")({
  loader: () => {
    const label = getCurrentWindow().label;

    if (label === "settings") {
      throw redirect({ to: "/settings" });
    }

    throw redirect({ to: "/chords" });
  },
});
