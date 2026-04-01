# Project: Chord

# About

Chord is an app that enables users to assign key sequences to computer actions, such as simulating shortcuts, running shell commands, and even executing JavaScript (using the [rquickjs](https://crates.io/crates/rquickjs) crate as well as the [LLRT runtime](https://github.com/awslabs/llrt)).

In contrast to shortcuts which are a combination of one or more modifier keys and a letter/number/symbol, chord key combinations are always a sequence of two or more letter/number/symbol keys.

# Architecture

Chord is built with Tauri and uses Rust for the app backend (located in `src-tauri/`) and TypeScript + React for the app frontend (located in `src/`).

## State

The source of truth for app state lives in the Rust backend; only UI/ephemeral state (e.g. search input) should live in React `useState`.

Rust shares state with React using "observables", which are provided by the [observable-property](https://crates.io/crates/observable-property) crate, all of which are located in `src-tauri/src/observables`). You should use an observable for any state you want accessible to the frontend. Under the hood, we simply call Tauri's `invoke` function to update it whenever the state in an observable changes, and keep it synced in React using a [custom hook](./src/utils/state.ts) that wraps `listen`.

On the Rust side, observable state can be accessed from anywhere via `handle.observable_state::<MyObservable>()`. However, they are always owned by state singletons: app-level structs which are registered using [Tauri's state management](https://v2.tauri.app/develop/state-management/) via [`app.manage::<MyStateType>`](./src-tauri/src/setup.rs). In `setup.rs`, we create a single instance of each of these state singletons, as well as a single instance of each observable.

> Note that observables are immutable (similar to React state), so updating them requires calling `.set_state` with a new instance of the inner State type.

## State singletons

All the state singletons are defined inside of the [app/](./src-tauri/src/app) folder in `src-tauri/`. We use [a macro](./src-tauri/src/app/mod.rs) to make all of them exposed on the `handle` directly (e.g. `handle.chord_package_manager()` instead of handle.state::<ChordPackageManager>()`).

## Terminology

- **Pathslug:** A relative path from the package root to the file in the package, e.g. `js/file.js`. Called a path slug because it's similar to a URL slug, but as a "path" and for a package name instead of a URL (e.g. `@keychord/pkg/js/file.js`).
- **Chords file:** A TOML file defining a set of chords.
- **App Chords file:** A _chords file_ for a specific app (i.e. is only active when that app is focused).
- **Raw Chords file:** The raw structure of a chords file, i.e. an unprocessed chords file that's only been passed to `toml::parse`
- **Parsed Chords file:** A chords file that's been parsed and normalized, but the imports have not yet been resolved. A chords file can be parsed in isolation just via the contents.
- **Compiled Chords file:** A chords file whose imports have been resolved and inlined. When compiling a chords file, the context of the containing package often needs to be provided (e.g. to resolve imports).
