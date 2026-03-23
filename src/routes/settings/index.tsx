import { createFileRoute } from "@tanstack/react-router";
import { TanStackRouterDevtoolsPanel } from "@tanstack/react-router-devtools";
import { TanStackDevtools } from "@tanstack/react-devtools";
import { Badge } from "#/components/ui/badge.tsx";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "#/components/ui/tabs.tsx";
import { Toaster } from "#/components/ui/sonner.tsx";
import { FirstRunOnboarding } from "#/components/settings/first-run-onboarding.tsx";
import { SettingsTab } from "#/components/settings/settings-tab.tsx";
import { ActiveChordsTab } from "#/components/settings/active-chords-tab.tsx";
import { GlobalShortcutsTab } from "#/components/settings/global-shortcuts-tab.tsx";
import { useSettingsPage } from "#/utils/use-settings-page.ts";

export const Route = createFileRoute("/settings/")({
  component: Settings,
});

function Settings() {
  const settingsPage = useSettingsPage();

  if (!settingsPage.onboarding.startupBusy && settingsPage.onboarding.showOnboarding) {
    return (
      <>
        <FirstRunOnboarding
          canFinish={settingsPage.onboarding.canFinish}
          isMacOS={settingsPage.onboarding.isMacOS}
          onContinue={() => {
            void settingsPage.onboarding.handleCompleteOnboarding();
          }}
          onSkip={() => {
            void settingsPage.onboarding.handleCompleteOnboarding();
          }}
          permissionSteps={settingsPage.onboarding.permissionSteps}
        />
        <Toaster position="top-right" />
      </>
    );
  }

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
          <div className="flex items-center gap-2">
            <Badge variant="outline">{settingsPage.summary.sourceCount} sources</Badge>
            <Badge variant="outline">{settingsPage.summary.chordCount} chords</Badge>
            <Badge variant="outline">{settingsPage.summary.shortcutCount} shortcuts</Badge>
          </div>
        </div>

        <Tabs defaultValue="settings" className="gap-4">
          <TabsList>
            <TabsTrigger value="settings">Settings</TabsTrigger>
            <TabsTrigger value="active-chords">Active Chords</TabsTrigger>
            <TabsTrigger value="global-shortcuts">Global Shortcuts</TabsTrigger>
          </TabsList>

          <TabsContent value="settings">
            <SettingsTab
              settings={settingsPage.settingsTab}
              appMetadataByBundleId={settingsPage.appMetadataByBundleId}
            />
          </TabsContent>

          <TabsContent value="active-chords">
            <ActiveChordsTab
              activeChords={settingsPage.activeChordsTab}
              appMetadataByBundleId={settingsPage.appMetadataByBundleId}
            />
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
