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

### Runtime log levels

While the development runner is open, focus the `tauri` pane and enter a log command:

```text
log debug
log info
log trace
log toggle
log status
```

Level changes take effect immediately and are persisted as `logLevel` in the development app's
state store. `RUST_LOG` remains a startup override and takes precedence over the persisted level.

The project tool versions are declared in [`.prototools`](./.prototools). Rust is also pinned in [`rust-toolchain.toml`](./rust-toolchain.toml) because proto delegates Rust version selection to rustup.

## Stores

There can be many owners of Observables (e.g. the AppHandle needs to `.manage` it so we can read the current state when initializing a window, and certain structs should be able to own it in order to modify it).

## JavaScript engines

Chord runs JS handlers on Bun by default, embedded through the
[`rbun`](https://github.com/KeyChord/rbun) crate (checked out next to this
repo at `../rbun`; cargo feature `bun`, enabled by default). Bun is what lets
packages load native code through Node-API add-ons with `process.dlopen`. The build links
`libbun_embed.dylib` from the rbun checkout, so build it once:

```sh
# one-time: build Bun and libbun_embed.dylib in the rbun checkout (~20 min cold)
(cd ../rbun && bun install && bun dev/improve/rbun/configs/bun/build/_build-bun.ts)

bun run dev            # Bun engine (default)
bun run dev:quickjs    # QuickJS-only build (--no-default-features), no native add-ons
```

QuickJS (`rquickjs` + LLRT) is the **legacy** engine (it cannot load Node-API
add-ons). Select it
in Settings → General → JavaScript Engine (persisted as `jsEngine` in the app
state store), with `CHORD_JS_ENGINE=bun|quickjs`, or per CLI invocation with
`chord run --engine quickjs <file>`; the app-level choice takes effect on the
next launch. The engine-specific code lives in `src-tauri/src/quickjs/`
and `src-tauri/src/bun_js/`; `src-tauri/src/js_engine.rs` selects between
them. Both expose the same `chord` module to packages.

The CLI honours the same environment variable, and takes an explicit flag
that overrides it:

```sh
CHORD_JS_ENGINE=bun target/debug/chord run ./script.ts
target/debug/chord run --engine quickjs ./script.js   # legacy engine
```

(TypeScript and Node-API add-ons need the Bun engine; the QuickJS engine only parses
plain JavaScript.)

## Releases

Every push to `beta` runs [`.github/workflows/release.yaml`](.github/workflows/release.yaml), which
builds the app for Apple Silicon and Intel and uploads the DMGs to the rolling
[`beta`](https://github.com/KeyChord/Chord/releases/tag/beta) prerelease. Beta uses the
`com.leonsilicon.chord.beta` application identifier, while local development uses
`com.leonsilicon.chord.development` and production uses `com.leonsilicon.chord`. This keeps each
channel's settings, packages, caches, logs, and other application-scoped data separate. Each run
replaces the previous assets, so the download URLs stay stable.

CI cannot build the vendored Bun itself — that takes ~30 minutes. Instead
[`KeyChord/rbun`](https://github.com/KeyChord/rbun) builds `libbun_embed.dylib` once per Bun source
commit and publishes it as a `bun-embed-<sha>` release asset; Chord's workflow downloads it and
points the build at it with `RBUN_BUN_LIB_DIR` plus a generated overlay config that overrides
`bundle.macOS.frameworks`. Both repos are checked out side by side so that the relative `rbun` path
in `Cargo.toml` resolves — the dylib satisfies the linker, but cargo still needs rbun's source
manifest. After bumping the Bun submodule in rbun, let its `build-bun-embed` workflow finish before
the next Chord release build.

### Code signing

Release builds are signed with the Developer ID certificate and notarized: `spctl -a -vv` on a
downloaded `.dmg` reports `source=Notarized Developer ID`, so the app opens by double-click with no
Gatekeeper prompt. The secrets below live at the organization level (they do not appear in
`gh secret list`, which shows only repository secrets). Each signing step is individually gated on
its secret, so the workflow still produces an ad-hoc signed build if one is ever removed.

| Secret | Effect |
| --- | --- |
| `APPLE_DEVELOPER_CERTIFICATE_FILE_BASE64`, `APPLE_DEVELOPER_CERTIFICATE_PASSWORD` | Signs with a Developer ID certificate instead of ad-hoc |
| `APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID` | Notarizes and staples the bundle |
| `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Produces the signed updater artifact the in-app updater consumes |

The updater artifact is only built when `TAURI_SIGNING_PRIVATE_KEY` is set: Tauri fails the build if
an updater bundle is requested while `plugins.updater.pubkey` is configured without a private key.
Generate the keypair with `bun tauri signer generate -w ~/.tauri/chord.key`.
