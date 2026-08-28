# Project: Chord

# About

Chord is an app that enables users to assign key sequences to computer actions, such as simulating shortcuts, running shell commands, and even executing JavaScript (using the [rquickjs](https://crates.io/crates/rquickjs) crate as well as the [LLRT runtime](https://github.com/awslabs/llrt); an experimental Bun engine via the [rbun](https://github.com/KeyChord/rbun) crate can be selected from the settings when built with `--features bun`, see `development.md`).

In contrast to shortcuts which are a combination of one or more modifier keys and a letter/number/symbol, chord key combinations are always a sequence of two or more letter/number/symbol keys.

# Monorepo layout

This repo is a Bun workspace monorepo that follows the [flatcap](https://github.com/leonsilicon/flatcap) convention:

- `apps/chord-tauri` — the Tauri desktop app. Its React source contains only desktop bootstrapping, generated router glue, route adapters, and global CSS; reusable UI and feature code belongs in workspace packages.
- `chord/com/npmjs/**` — npm wrapper packages named `@chord/com.npmjs.*`.
- `chord/dev/improve/chord/components/ui/**` — one isolated package per reusable UI primitive.
- `chord/dev/improve/chord/routes/**` — route packages (page UI + logic). Route-owned shared packages use internal path segments such as `routes/settings/_components/**`.
- `chord/dev/improve/chord/api/**`, `hooks/**`, `lib/**`, and `data/**` — narrowly scoped shared packages.
- `chord/dev/improve/chord/configs/**` — tooling packages (e.g. patching codemods)

Run app commands from the repo root with `bun run dev`, `bun run build`, etc., or `cd apps/chord-tauri` for direct access.

## Flatcap packages

- The top-level scope folder is `chord/`, so workspace package names use the `@chord` scope.
- Product packages live below `chord/dev/improve/chord/` and therefore start with `@chord/dev.improve.chord`.
- Package code is flat: code files live beside `package.json`. Package-owned non-code assets may live in an `@assets/` directory.
- Public entry files start with `+` and contain re-exports only. Use `+.ts` for the root export.
- Relative source imports are not allowed inside flatcap packages. Declare local `#...` subpath imports in `package.json`; import other packages by their workspace name.
- A path segment starting with `_` is internal and may only be imported by packages sharing the prefix before that segment. A path segment starting with `__` is private to its parent package.
- Each package declares its own direct runtime and workspace dependencies.

# Architecture

Chord is built with Tauri and uses Rust for the app backend (located in `apps/chord-tauri/src-tauri/`) and TypeScript + React for the app frontend (primarily in `chord/dev/improve/chord/`, with platform glue in `apps/chord-tauri/src/`).

## State

The source of truth for app state lives in the Rust backend; only UI/ephemeral state (e.g. search input) should live in React `useState`.

Rust shares state with React using "observables", which are provided by the [observable-property](https://crates.io/crates/observable-property) crate, all of which are located in `apps/chord-tauri/src-tauri/src/state/observables/`). You should use an observable for any state you want accessible to the frontend. Under the hood, we call Tauri's `invoke` function to update it whenever the state in an observable changes and keep it synced in React using the [`lib/state` package](./chord/dev/improve/chord/lib/state/state.ts), which wraps `listen`.

On the Rust side, observable state can be accessed from anywhere via `handle.observable_state::<MyObservable>()`. However, they are always owned by state singletons: app-level structs which are registered using [Tauri's state management](https://v2.tauri.app/develop/state-management/) via [`app.manage::<MyStateType>`](./apps/chord-tauri/src-tauri/src/setup.rs). In `setup.rs`, we create a single instance of each of these state singletons, as well as a single instance of each observable.

> Note that observables are immutable (similar to React state), so updating them requires calling `.set_state` with a new instance of the inner State type.

## State singletons

All the state singletons are defined inside of the [app/](./apps/chord-tauri/src-tauri/src/app) folder in `src-tauri/`. We use [a macro](./apps/chord-tauri/src-tauri/src/app/mod.rs) to make all of them exposed on the `handle` directly (e.g. `handle.chord_package_manager()` instead of handle.state::<ChordPackageManager>()`).

## JS engines and native code

Every handler is a JavaScript module. Chord runs them on Bun — the embedded runtime from the sibling [`rbun`](../rbun) crate (`src-tauri/src/bun_js/`, cargo feature `bun`, on by default; `libbun_embed.dylib` is bundled from `bundle.macOS.frameworks`) — which is the default and the only engine that can load Node-API add-ons with `process.dlopen`. QuickJS/LLRT (`src-tauri/src/quickjs/`) is the legacy engine, still runtime-selectable and the automatic choice for a `--no-default-features` build; `src-tauri/src/js_engine.rs` picks one per process. On Bun, package modules are imported straight from the package directory (`ChordJsPackage::root`), so `import.meta` is real and relative imports, `node:*`/`bun:*` and a package's `node_modules` resolve through Bun itself; the Rust resolver only serves the `chord` module.

Chord has no separate native-handler runtime. A package that needs native code ships a prebuilt Node-API add-on under `target/<triple>/<relpath>/<stem>.node` with NodeSwift's `libNodeAPI.dylib` beside it (e.g. `target/<triple>/menu/{menu.node,libNodeAPI.dylib}`; the triple comes from `build.rs` as `CHORD_TARGET_TRIPLE`; vendored packages nest as `target/@scope/name/target/...`) and loads it from its own JS with `process.dlopen`, locating it through the `chord` module's `resolveNativeModulePath(import.meta, relpath)` / `resolvePackageFile(import.meta, path)` (`models/native.rs` + `resolve_logical_package_path` in `chord_js_package.rs`). `@keychord/config` builds Swift add-ons with NodeSwift from `src/swift/`. See `scripting.md` for the author-facing contract and `chords/packages/chords-menu` for the reference package.

## Terminology

- **Pathslug:** A relative path from the package root to the file in the package, e.g. `js/file.js`. Called a path slug because it's similar to a URL slug, but as a "path" and for a package name instead of a URL (e.g. `@keychord/pkg/js/file.js`).
- **Chords file:** A TOML file defining a set of chords.
- **App Chords file:** A _chords file_ for a specific app (i.e. is only active when that app is focused).
- **Raw Chords file:** The raw structure of a chords file, i.e. an unprocessed chords file that's only been passed to `toml::parse`
- **Parsed Chords file:** A chords file that's been parsed and normalized, but the imports have not yet been resolved. A chords file can be parsed in isolation just via the contents.
- **Compiled Chords file:** A chords file whose imports have been resolved and inlined. When compiling a chords file, the context of the containing package often needs to be provided (e.g. to resolve imports).
