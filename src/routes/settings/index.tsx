import { createFileRoute } from "@tanstack/react-router";
import { TanStackRouterDevtoolsPanel } from "@tanstack/react-router-devtools";
import { TanStackDevtools } from "@tanstack/react-devtools";
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "#/components/ui/tabs.tsx";
import { Toaster } from "#/components/ui/sonner.tsx";
import { SettingsTab } from "#/components/settings/settings-tab.tsx";
import { ActiveChordsTab } from "#/components/settings/active-chords-tab.tsx";
import { GlobalShortcutsTab } from "#/components/settings/global-shortcuts-tab.tsx";
import { FirstRunOnboarding } from "#/components/settings/first-run-onboarding.tsx";
import { taurpc } from "#/api/taurpc.ts";

export const Route = createFileRoute("/settings/")({
  component: Settings,
});

function Settings() {
  const [dismissedOnboarding, setDismissedOnboarding] = useState(false);
  const startupStatusQuery = useQuery({
    queryKey: ["startup-status"],
    queryFn: taurpc.getStartupStatus,
  });
  const shouldShowOnboarding =
    startupStatusQuery.data?.shouldShowOnboarding === true && !dismissedOnboarding;

  return (
    <>
      {startupStatusQuery.isLoading ? (
        <div className="flex min-h-full items-center justify-center bg-muted/30 px-5 py-4 text-sm text-muted-foreground">
          Loading settings...
        </div>
      ) : shouldShowOnboarding ? (
        <FirstRunOnboarding
          onSkip={() => {
            setDismissedOnboarding(true);
          }}
          onComplete={() => {
            setDismissedOnboarding(true);
            void startupStatusQuery.refetch();
          }}
        />
      ) : (
        <div className="min-h-full bg-muted/30 px-5 py-4 text-sm text-foreground">
          <div className="mx-auto flex max-w-[720px] flex-col gap-4">
            <div className="flex items-start justify-between gap-3">
              <div>
                <h1 className="text-[20px] font-semibold">Chords</h1>
                <p className="mt-1 text-muted-foreground">
                  Configure the tray app, manage chord sources, and inspect the active chord
                  registry.
                </p>
              </div>
            </div>

            <Tabs defaultValue="settings" className="gap-4">
              <TabsList>
                <TabsTrigger value="settings">Settings</TabsTrigger>
                <TabsTrigger value="active-chords">Active Chords</TabsTrigger>
                <TabsTrigger value="global-shortcuts">Global Shortcuts</TabsTrigger>
              </TabsList>

              <TabsContent value="settings">
                <SettingsTab />
              </TabsContent>

              <TabsContent value="active-chords">
                <ActiveChordsTab />
              </TabsContent>

              <TabsContent value="global-shortcuts">
                <GlobalShortcutsTab />
              </TabsContent>
            </Tabs>
          </div>
        </div>
      )}
      <Toaster position="top-right" />
      <TanStackDevtools
        config={{
          position: "bottom-right",
        }}
        plugins={[
          {
            name: "TanStack Router",
            render: <TanStackRouterDevtoolsPanel />,
          },
        ]}
      />
    </>
  );
}
