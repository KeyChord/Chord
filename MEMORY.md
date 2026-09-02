# Project Environment

- Chord is a Bun workspace monorepo with a Tauri 2 desktop app in `apps/chord-tauri`.
- The frontend is React 19, TypeScript, Vite, and Tailwind CSS 4; it is not React Native.
- Run the app from the repository root with `bun run dev`; Vite uses port 1420.
- Run the frontend build with `bun run build` and Rust tests from `apps/chord-tauri/src-tauri` with `cargo test`.
- The settings window is created in `apps/chord-tauri/src-tauri/src/app/settings/settings_ui.rs` with label `settings`.
- The settings layout lives in `packages/dev/improve/chord/routes/settings/settings-page.tsx`.
- Route adapters stay in `apps/chord-tauri/src/routes`; reusable UI belongs in workspace packages.
- The existing sidebar primitive is `@chord/dev.improve.chord.components.ui.sidebar`.
- Native macOS translucency is available through Tauri window effects; `window-vibrancy` 0.8 is also already present.
