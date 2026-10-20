import { SettingsPersonalPage } from "@chord/dev.improve.chord.routes.settings.personal.index";
import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/settings/personal/")({ component: SettingsPersonalPage });
