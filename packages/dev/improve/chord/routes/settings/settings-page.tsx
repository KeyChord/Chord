import { TanStackDevtools } from "@chord/com.npmjs.tanstack__react-devtools";
import { useMutation } from "@chord/com.npmjs.tanstack__react-query";
import { Link, Outlet, useRouterState } from "@chord/com.npmjs.tanstack__react-router";
import { TanStackRouterDevtoolsPanel } from "@chord/com.npmjs.tanstack__react-router-devtools";
import { taurpc } from "@chord/dev.improve.chord.api.taurpc";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@chord/dev.improve.chord.components.ui.collapsible";
import {
  desktopPlatform,
  useAppUpdateState,
  useSettingsState,
} from "@chord/dev.improve.chord.lib.state";
import { LogStreamProvider } from "@chord/dev.improve.chord.routes.settings._components.log-stream";
import { PermissionsDialog } from "@chord/dev.improve.chord.routes.settings._components.permissions-dialog";
import { UpdateCard } from "@chord/dev.improve.chord.routes.settings._components.update-card";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  Command,
  ChevronRight,
  Compass,
  Keyboard,
  Braces,
  PanelLeft,
  Settings2,
  SquareTerminal,
  Terminal,
  TriangleAlert,
} from "lucide-react";
import { useEffect, useRef, useState } from "react";

/**
 * Each page's description shows under its title in the window header, so every page says what
 * it's for. Advanced pages run from everyday tools to the reset that can't be undone.
 */
const navigationGroups = [
  {
    label: "Configuration",
    collapsible: false,
    items: [
      {
        label: "General",
        to: "/settings/general",
        icon: Settings2,
        description: "Startup, permissions and how chord mode looks.",
      },
      {
        label: "Chords",
        to: "/settings/chords",
        icon: Keyboard,
        description: "Your packages, your own chords and third-party code.",
      },
      {
        label: "Browse",
        to: "/settings/browse",
        icon: Compass,
        description: "Find and install chord packages.",
      },
    ],
  },
  {
    label: "Advanced",
    collapsible: true,
    items: [
      {
        label: "Macros",
        to: "/settings/macros",
        icon: Braces,
        description: "Saved text and key presses you can insert while typing a chord.",
      },
      {
        label: "Global Shortcuts",
        to: "/settings/global-shortcuts",
        icon: Command,
        description: "Shortcuts packages reserve so Chord can trigger actions inside apps.",
      },
      {
        label: "Command Line",
        to: "/settings/command-line",
        icon: SquareTerminal,
        description: "Run chords and scripts from Terminal with the chord command.",
      },
      {
        label: "Log",
        to: "/settings/log",
        icon: Terminal,
        description: "What Chord has been doing, for tracking down problems.",
      },
      {
        label: "Danger Zone",
        to: "/settings/danger",
        icon: TriangleAlert,
        description: "Actions that can't be undone.",
      },
    ],
  },
] as const;

function handleTitleBarMouseDown(event: React.MouseEvent<HTMLElement>) {
  if (event.buttons !== 1) {
    return;
  }

  const window = getCurrentWindow();
  void (event.detail === 2 ? window.toggleMaximize() : window.startDragging());
}

function SettingsShell({ children }: { children: React.ReactNode }) {
  const pathname = useRouterState({
    select: (state) => state.location.pathname,
  });
  const isAdvancedPage = navigationGroups.some(
    (group) =>
      group.collapsible &&
      group.items.some((item) => pathname === item.to || pathname === `${item.to}/`),
  );
  const [advancedOpen, setAdvancedOpen] = useState(isAdvancedPage);
  const settings = useSettingsState();
  const appUpdate = useAppUpdateState();
  const toggleSidebarMutation = useMutation({ mutationFn: taurpc.toggleSettingsSidebar });
  const scrollContainerRef = useRef<HTMLDivElement>(null);
  // These pages fill the window and scroll inside themselves.
  const fillsWindow = ["/settings/log", "/settings/browse"].some(
    (path) => pathname === path || pathname === `${path}/`,
  );
  const currentPage = navigationGroups
    .map((group) => group.items.find((item) => pathname === item.to || pathname === `${item.to}/`))
    .find((item) => item !== undefined);

  useEffect(() => {
    scrollContainerRef.current?.scrollTo({ top: 0 });
    if (isAdvancedPage) {
      setAdvancedOpen(true);
    }
  }, [pathname, isAdvancedPage]);

  // Waiting for the saved state keeps a collapsed sidebar from opening and sliding shut.
  if (settings.isPending) {
    return null;
  }

  const isSidebarCollapsed = settings.data?.isSettingsSidebarCollapsed ?? false;
  const sidebarToggleLabel = isSidebarCollapsed ? "Show Sidebar" : "Hide Sidebar";
  const hasUpdateCard = appUpdate.data !== undefined && appUpdate.data.status !== "idle";

  return (
    <div className="settings-window-shell relative flex min-h-0 h-full w-full">
      {/* Beside the traffic lights, where it stays put whether the sidebar is open or not.
          Their centres sit 26px from the top (`traffic_light_position` in settings_ui.rs),
          so this 28px button starts 14px above that. */}
      <button
        type="button"
        aria-controls="settings-sidebar"
        aria-expanded={!isSidebarCollapsed}
        aria-label={sidebarToggleLabel}
        title={sidebarToggleLabel}
        className="absolute left-[84px] top-[12px] z-10 flex size-7 items-center justify-center rounded-lg text-sidebar-foreground/60 outline-none hover:bg-foreground/5 hover:text-sidebar-foreground focus-visible:ring-2 focus-visible:ring-ring"
        onClick={() => {
          toggleSidebarMutation.mutate();
        }}
      >
        <PanelLeft className="size-4" strokeWidth={1.8} />
      </button>

      <aside
        id="settings-sidebar"
        inert={isSidebarCollapsed}
        data-collapsed={isSidebarCollapsed || undefined}
        className="settings-window-sidebar flex w-[15.5rem] shrink-0 select-none flex-col overflow-hidden border-r border-black/[0.08] dark:border-white/[0.08] text-sidebar-foreground transition-[width] duration-200 ease-out motion-reduce:transition-none data-collapsed:w-0 data-collapsed:border-r-0"
      >
        <div className="h-[50px] shrink-0" onMouseDown={handleTitleBarMouseDown} />

        {/* Fixed width, so the links are clipped as the sidebar closes instead of reflowing. */}
        <nav
          aria-label="Settings"
          className={`min-h-0 w-[calc(15.5rem-1px)] flex-1 overflow-y-auto px-2.5 pt-1 ${hasUpdateCard ? "pb-24" : "pb-4"}`}
        >
          {navigationGroups.map((group) => (
            <Collapsible
              key={group.label}
              className="py-2"
              open={group.collapsible ? advancedOpen : true}
              onOpenChange={group.collapsible ? setAdvancedOpen : undefined}
            >
              {group.collapsible ? (
                <CollapsibleTrigger className="group/advanced flex h-6 w-full items-center px-2.5 py-0 text-[11px] font-semibold tracking-[0.01em] text-sidebar-foreground/40 hover:text-sidebar-foreground/70">
                  {group.label}
                  <ChevronRight className="ml-auto size-3 transition-transform group-aria-expanded/advanced:rotate-90" />
                </CollapsibleTrigger>
              ) : (
                <div className="flex h-6 items-center px-2.5 text-[11px] font-semibold tracking-[0.01em] text-sidebar-foreground/40">
                  {group.label}
                </div>
              )}
              <CollapsibleContent>
                <ul className="flex flex-col gap-0.5">
                  {group.items.map((item) => {
                    const isActive = pathname === item.to || pathname === `${item.to}/`;
                    const Icon = item.icon;

                    return (
                      <li key={item.to}>
                        <Link
                          to={item.to}
                          aria-current={isActive ? "page" : undefined}
                          data-active={isActive || undefined}
                          className="flex h-8 items-center gap-2 rounded-[8px] px-2.5 text-[13px] font-medium text-sidebar-foreground/85 outline-none hover:bg-foreground/5 focus-visible:ring-2 focus-visible:ring-ring data-active:bg-primary data-active:text-primary-foreground data-active:shadow-[inset_0_0_0_0.5px_rgba(0,0,0,0.08)] data-active:hover:bg-primary data-active:hover:text-primary-foreground"
                        >
                          <Icon className="size-4 shrink-0" strokeWidth={1.8} />
                          <span>{item.label}</span>
                        </Link>
                      </li>
                    );
                  })}
                </ul>
              </CollapsibleContent>
            </Collapsible>
          ))}
        </nav>
      </aside>

      <main className="settings-content-surface flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden bg-background">
        <header
          data-sidebar-collapsed={isSidebarCollapsed || undefined}
          className="flex h-[62px] shrink-0 items-center px-4 transition-[padding] duration-200 ease-out motion-reduce:transition-none data-sidebar-collapsed:pl-[120px]"
          onMouseDown={handleTitleBarMouseDown}
        >
          <div className="min-w-0 cursor-default select-none">
            <h1 className="settings-page-title">{currentPage?.label ?? "Chord"}</h1>
            {currentPage?.description && (
              <p className="settings-page-description truncate">{currentPage.description}</p>
            )}
          </div>
        </header>
        <div
          ref={scrollContainerRef}
          className={
            fillsWindow
              ? "settings-page-scroll min-h-0 flex-1 overflow-hidden bg-background"
              : "settings-page-scroll min-h-0 flex-1 overflow-y-auto bg-background"
          }
        >
          <div
            className={
              fillsWindow
                ? `settings-page-content h-full w-full px-6 pt-4 ${isSidebarCollapsed && hasUpdateCard ? "pb-28" : "pb-6"}`
                : `settings-page-content mx-auto w-full max-w-[960px] px-6 pt-4 ${isSidebarCollapsed && hasUpdateCard ? "pb-28" : "pb-10"}`
            }
          >
            {children}
          </div>
        </div>
      </main>

      {/* Over the sidebar's corner, so collapsing the sidebar doesn't hide it. */}
      <UpdateCard className="absolute bottom-3 left-3 z-10 w-[calc(15.5rem-1.5rem-1px)]" />
    </div>
  );
}

export function SettingsPage() {
  const [permissionDialogDismissed, setPermissionDialogDismissed] = useState(false);

  return (
    <LogStreamProvider>
      <SettingsShell>
        <Outlet />
      </SettingsShell>
      {desktopPlatform === "macos" && !permissionDialogDismissed ? (
        <PermissionsDialog
          onDismiss={() => {
            setPermissionDialogDismissed(true);
          }}
        />
      ) : null}
      {import.meta.env.DEV ? (
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
      ) : null}
    </LogStreamProvider>
  );
}
