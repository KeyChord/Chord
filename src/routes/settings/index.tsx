import { createFileRoute } from "@tanstack/react-router";
import { TanStackRouterDevtoolsPanel } from "@tanstack/react-router-devtools";
import { TanStackDevtools } from "@tanstack/react-devtools";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "#/components/ui/tabs.tsx";
import { Toaster } from "#/components/ui/sonner.tsx";
import { SettingsTab } from "#/components/settings/settings-tab.tsx";
import { ActiveChordsTab } from "#/components/settings/active-chords-tab.tsx";
import { GlobalShortcutsTab } from "#/components/settings/global-shortcuts-tab.tsx";

export const Route = createFileRoute("/settings/")({
  component: Settings,
});

function Settings() {
  return (
    <div className="min-h-full bg-muted/30 px-5 py-4 text-sm text-foreground">
      <div className="mx-auto flex max-w-[720px] flex-col gap-4">
        <div className="flex items-start justify-between gap-3">
          <div>
            <h1 className="text-[20px] font-semibold">Chords</h1>
            <p className="mt-1 text-muted-foreground">
              Configure the tray app, manage chord sources, and inspect the active chord registry.
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
            <GlobalShortcutsTab
              globalShortcuts={settingsPage.globalShortcutsTab}
              appMetadataByBundleId={settingsPage.appMetadataByBundleId}
            />
          </TabsContent>
        </Tabs>
      </div>
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
    </div>
  );
}
