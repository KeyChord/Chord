import { TanStackDevtools } from "@chord/com.npmjs.tanstack__react-devtools";
import { Link, Outlet, useRouterState } from "@chord/com.npmjs.tanstack__react-router";
import { TanStackRouterDevtoolsPanel } from "@chord/com.npmjs.tanstack__react-router-devtools";
import {
  Collapsible,
  CollapsibleContent,
  CollapsibleTrigger,
} from "@chord/dev.improve.chord.components.ui.collapsible";
import { LogStreamProvider } from "@chord/dev.improve.chord.routes.settings._components.log-stream";
import { PermissionsDialog } from "@chord/dev.improve.chord.routes.settings._components.permissions-dialog";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  Command,
  ChevronRight,
  Compass,
  Keyboard,
  Braces,
  Settings2,
  SlidersHorizontal,
  Terminal,
  TriangleAlert,
} from "lucide-react";
import { useEffect, useRef, useState } from "react";

const navigationGroups = [
  {
    label: "Configuration",
    collapsible: false,
    items: [
      {
        label: "General",
        to: "/settings/general",
        icon: Settings2,
      },
      {
        label: "Chords",
        to: "/settings/chords",
        icon: Keyboard,
      },
      {
        label: "Browse",
        to: "/settings/browse",
        icon: Compass,
      },
      {
        label: "Configure",
        to: "/settings/configure",
        icon: SlidersHorizontal,
      },
    ],
  },
  {
    label: "Advanced",
    collapsible: true,
    items: [
      {
        label: "Log",
        to: "/settings/log",
        icon: Terminal,
      },
      {
        label: "Danger Zone",
        to: "/settings/danger",
        icon: TriangleAlert,
      },
      {
        label: "Macros",
        to: "/settings/macros",
        icon: Braces,
      },
      {
        label: "Global Shortcuts",
        to: "/settings/global-shortcuts",
        icon: Command,
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
  const scrollContainerRef = useRef<HTMLDivElement>(null);
  const isLogPage = pathname === "/settings/log" || pathname === "/settings/log/";
  const currentPage = navigationGroups
    .map((group) => group.items.find((item) => pathname === item.to || pathname === `${item.to}/`))
    .find((item) => item !== undefined);

  useEffect(() => {
    scrollContainerRef.current?.scrollTo({ top: 0 });
    if (isAdvancedPage) {
      setAdvancedOpen(true);
    }
  }, [pathname, isAdvancedPage]);

  return (
    <div className="settings-window-shell flex min-h-0 h-full w-full">
      <aside className="settings-window-sidebar flex w-[15.5rem] shrink-0 flex-col border-r border-black/[0.08] text-sidebar-foreground">
        <div className="h-[50px] shrink-0" onMouseDown={handleTitleBarMouseDown} />

        <nav aria-label="Settings" className="min-h-0 flex-1 overflow-y-auto px-2.5 pb-4 pt-1">
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
          className="flex h-[62px] shrink-0 items-center px-4"
          onMouseDown={handleTitleBarMouseDown}
        >
          <h1 className="settings-page-title cursor-default select-none">
            {currentPage?.label === "General" ? "Configuration" : (currentPage?.label ?? "Chord")}
          </h1>
        </header>
        <div
          ref={scrollContainerRef}
          className={
            isLogPage
              ? "settings-page-scroll min-h-0 flex-1 overflow-hidden bg-background"
              : "settings-page-scroll min-h-0 flex-1 overflow-y-auto bg-background"
          }
        >
          <div
            className={
              isLogPage
                ? "settings-page-content h-full w-full px-6 pb-6 pt-4"
                : "settings-page-content mx-auto w-full max-w-[960px] px-6 pb-10 pt-4"
            }
          >
            {children}
          </div>
        </div>
      </main>
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
      {!permissionDialogDismissed ? (
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
