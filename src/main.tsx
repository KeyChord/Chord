import ReactDOM from "react-dom/client";
import { attachConsole } from "@tauri-apps/plugin-log";
import { RouterProvider } from "@tanstack/react-router";
import { getRouter } from "./router.tsx";

if (import.meta.env.DEV) {
  void attachConsole();
}

const rootElement = document.getElementById("root");

if (rootElement && !rootElement.innerHTML) {
  const root = ReactDOM.createRoot(rootElement);
  root.render(<RouterProvider router={getRouter()} />);
}
