import ReactDOM from "react-dom/client";
import { attachConsole, debug } from "@tauri-apps/plugin-log";
import { RouterProvider, createRouter } from '@tanstack/react-router'
import { routeTree } from './routeTree.gen'
import { Route as SettingsRoute } from './routes/settings/index.tsx'
import { Route as ChordsRoute } from './routes/chords/index.tsx'
import { Route as RootRoute } from './routes/__root.tsx'
import { getCurrentWindow } from "@tauri-apps/api/window";

if (import.meta.env.DEV) {
  void attachConsole();
}

const settingsRouter = createRouter({
  routeTree: RootRoute._addFileChildren(SettingsRoute),
  defaultPreload: 'intent',
  scrollRestoration: true,
})

const chordsRouter = createRouter({
  routeTree: RootRoute._addFileChildren(ChordsRoute),
  defaultPreload: 'intent',
  scrollRestoration: true,
})

declare module '@tanstack/react-router' {
  interface Register {
    router: typeof createRouter<typeof routeTree>
  }
}

const rootElement = document.getElementById('root')

const currentWindowLabel = getCurrentWindow().label
debug(`Current window label: ${currentWindowLabel}`)

if (rootElement && !rootElement.innerHTML) {
  const root = ReactDOM.createRoot(rootElement)
  root.render(<RouterProvider router={currentWindowLabel === 'settings' ? settingsRouter : chordsRouter} />)
}
