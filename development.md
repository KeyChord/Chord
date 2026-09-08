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

On macOS, a new development app launch replaces the running instance with the
same development bundle identifier. Startup logs the previous PID, sends SIGTERM,
and waits for it to exit before starting the new instance. If it ignores SIGTERM
for two seconds, startup verifies its socket ownership and sends SIGKILL. Concurrent launches
are serialized. CLI chord commands still forward to the running app, and
production/beta launches retain the existing instance.

### Input ownership across app instances

Each process registers at most one keyboard event tap and one Caps Lock HID listener.
All app channels share an OS-held input lease. Development bundle identifiers
(`com.leonsilicon.chord.development` and its dotted suffixes) take priority over
production and other channels. Among instances of the same priority, the current
owner stays active until it exits or becomes ineligible.

An instance must have Accessibility and Input Monitoring enabled and its input
handlers running before requesting ownership. Standby instances forward keyboard
events untouched and ignore Caps Lock input. On handoff, Chord drains in-flight
input processing, exits chord mode, clears held-key state, and rejects queued
input from the previous ownership generation. Ownership is checked every 25 ms;
a handoff also waits for any current input handler to finish.

Lease files live in the user's application-support directory under
`com.leonsilicon.chord/input-ownership-v1/`, shared across bundle identifiers and
independent of `TMPDIR`. **Do not delete these files while Chord is running.**
Their existence does not indicate ownership: the OS releases the locks even on
crash or force-quit, so a remaining lockfile cannot suppress another instance.
Ownership transitions appear in logs as `Input ownership: active` / `standby`.

Both dev and production must run a build containing this protocol. Older installed
builds do not participate and must be quit or updated before running alongside dev.

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

## Editing an installed chordpack locally

In Settings → Chords, choose **Link Local Folder** on an installed chord repo and
enter the absolute path to your local checkout. Its `package.json` name must match
the installed package. Links also work for pinned chordpacks and persist across
app restarts.

Chord loads the linked folder in place. After editing chords or JavaScript, click
**Reload** on the repo (or use the tray reload action) to reload configs and rebuild
the JS runtime. Sync is disabled while linked. **Unlink** restores the cached
version without changing your local files.

## Chord monorepos

In Settings → Chords → **Chord Monorepos**, add a GitHub repository or enter an
absolute local monorepo path and click the link icon. A GitHub monorepo can also
be linked to a local checkout using its link icon. Chord discovers immediate
directories matching `packages/chords-*`; other directories and nested packages
are ignored. Each matching folder is loaded as its own package, using its
`package.json` name or, when absent, its folder name.

Local links persist across restarts. **Reload** rescans the monorepo, including
new or removed packages, and rebuilds the JS runtime. **Unlink** stops loading a
standalone local monorepo or restores the cached GitHub source for a linked repo;
it leaves local files intact.

Packages with the same name are overridden in this order (last wins): cached
GitHub sources, standalone local monorepos, locally linked GitHub sources, and
individual local folders. All local sources therefore take precedence over
remote GitHub packages. Paths and repo slugs are sorted within each group for
consistent results. Duplicate package names within one monorepo are rejected.

## Stores

There can be many owners of Observables (e.g. the AppHandle needs to `.manage` it so we can read the current state when initializing a window, and certain structs should be able to own it in order to modify it).

## JavaScript runtime

Chord runs all JS handlers on Bun, embedded through the
[`rbun`](https://github.com/KeyChord/rbun) crate checked out next to this repo
at `../rbun`. Packages can use Bun and Node APIs and load native code through
Node-API add-ons with `process.dlopen`. The build links `libbun_embed.dylib`
from the rbun checkout, so build it once:

```sh
# one-time: build Bun and libbun_embed.dylib in the rbun checkout (~20 min cold)
(cd ../rbun && bun install && bun dev/improve/rbun/configs/bun/build/_build-bun.ts)

bun run dev
```

The runtime integration lives in `src-tauri/src/bun_js/` and exposes the
`chord` module to packages. The CLI uses the same embedded runtime:

```sh
target/debug/chord bun ./script.ts
target/debug/chord run ./script.ts export-name arg1 arg2
```

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
