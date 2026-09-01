import { SettingsLogPage } from "@chord/dev.improve.chord.routes.settings.log.index";
import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/settings/log/")({
  component: SettingsLogPage,
});
