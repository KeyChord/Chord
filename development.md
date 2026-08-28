# Development

## Toolchain

Chord uses [proto](https://moonrepo.dev/proto) to install the versions of Bun and Rust pinned for the repository.

Install proto on macOS, Linux, or WSL:

```sh
bash <(curl -fsSL https://moonrepo.dev/install/proto.sh) 0.61.1 --yes
```

Restart your shell, then install the toolchain and project dependencies from the repository root:

```sh
proto install
bun install --frozen-lockfile
```

Start the desktop app with:

```sh
bun run dev
```

The project tool versions are declared in [`.prototools`](./.prototools). Rust is also pinned in [`rust-toolchain.toml`](./rust-toolchain.toml) because proto delegates Rust version selection to rustup.

## Stores

There can be many owners of Observables (e.g. the AppHandle needs to `.manage` it so we can read the current state when initializing a window, and certain structs should be able to own it in order to modify it).

## JavaScript engines

Chord runs JS handlers on Bun by default, embedded through the
[`rbun`](https://github.com/KeyChord/rbun) crate (checked out next to this
repo at `../rbun`; cargo feature `bun`, enabled by default). Bun is what lets
packages load native code with `bun:ffi`. The build links
`libbun_embed.dylib` from the rbun checkout, so build it once:

```sh
# one-time: build Bun and libbun_embed.dylib in the rbun checkout (~20 min cold)
(cd ../rbun && scripts/build-bun.sh)

bun run dev            # Bun engine (default)
bun run dev:quickjs    # QuickJS-only build (--no-default-features), no bun:ffi
```

QuickJS (`rquickjs` + LLRT) remains available as a fallback engine: choose it
in Settings → General → JavaScript Engine (persisted as `jsEngine` in the app
state store) or with `CHORD_JS_ENGINE=bun|quickjs`; the choice takes effect on
the next launch. The engine-specific code lives in `src-tauri/src/quickjs/`
and `src-tauri/src/bun_js/`; `src-tauri/src/js_engine.rs` selects between
them. Both expose the same `chord` module to packages.

The CLI honours the same environment variable:

```sh
CHORD_JS_ENGINE=bun target/debug/chord run ./script.ts
```
