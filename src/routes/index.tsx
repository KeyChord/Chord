import "./global.css";
import { createFileRoute, redirect, useRouteContext, useRouter, useRouterState } from '@tanstack/react-router'
import { getCurrentWindow } from "@tauri-apps/api/window";

export const Route = createFileRoute('/')({ component: App })

function App() {
  const windowLabel = getCurrentWindow().label;
  const router = useRouter()
  if (router.basepath) {
  }

  if (windowLabel === "chords") {
    return redirect('/chords')
  }

  return redirect('/settings')
}

