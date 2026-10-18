# Development

> [!NOTE]
> This document is largely AI-written, so parts of it may not be fully accurate. It will be replaced with a hand-written version when I get the time.

This guide is for building _Chord_ from source. To just use it, download the app as described in the [readme](./readme.md#installation).

## Building from source

_Chord_ builds on macOS. It links against a patched Bun from [rbun](https://github.com/KeyChord/rbun), which has to be checked out next to this repo, with its Bun submodule:

```sh
mkdir KeyChord && cd KeyChord
git clone https://github.com/KeyChord/Chord
git clone --recurse-submodules https://github.com/KeyChord/rbun
```

[KeyChord/chords](https://github.com/KeyChord/chords) is optional. When it is checked out next to `Chord`, development builds read Chord's core `0` bindings from it (see [Editing an installed chordpack locally](#editing-an-installed-chordpack-locally)).

Then install the toolchain (see [Toolchain](#toolchain)), build Bun once, and start the app:

```sh
cd Chord
proto install
bun install --frozen-lockfile
# one-time: build Bun and libbun_embed.dylib in the rbun checkout (~20 min cold);
# install the prerequisites from rbun's README first
(cd ../rbun && bun install && bun run _build-bun)
bun run dev
```

`bun run dev` signs the development app with a certificate-backed code-signing identity and does not fall back to ad-hoc signing. If you are not on the maintainer's machine, set `CHORD_DEV_SIGNING_IDENTITY` to one of your identities (list them with `security find-identity -v -p codesigning`; a free Apple Development certificate from Xcode works). The development app is called **Chord (Development)** and needs its own Accessibility grant.

## Version control

Chord uses Jujutsu (`jj`) in a colocated Git checkout, following Haniverse's
workflow with `dev` as the integration bookmark. Existing Git history and remotes
are reused. Install `jj` on macOS with `brew install jj`, then initialize each new
checkout from the repository root:

```sh
jj git init --colocate
jj config edit --repo
```

Add the following repository-local configuration. It overrides Haniverse's
user-level `trunk` aliases only for Chord; user identity and other user settings
are inherited.

```toml
[revset-aliases]
'trunk()' = 'dev@origin'

[aliases]
gf = ["git", "fetch"]
pull = ["util", "exec", "--", "bash", "-c", """
set -e
jj git fetch
jj bookmark set dev -r dev@origin --allow-backwards
jj rebase -d dev
"""]
push = ["util", "exec", "--", "bash", "-c", """
set -e
target=$(jj log --no-graph -r 'heads(::@ & mutable() & ~description(exact:"") & ~empty())' -T 'commit_id' 2>/dev/null || true)
if [ -z "$target" ]; then
  echo "error: nothing to push (no described, non-empty commit on this branch)" >&2
  exit 1
fi
jj bookmark move dev --to "$target"
exec jj git push -b dev "$@"
""", "bash"]
```

Track the integration bookmark once:

```sh
jj bookmark track dev --remote origin
```

Everyday commands:

```sh
jj st
jj --no-pager diff --git
jj --no-pager log
jj describe -m "Describe the change"
jj push       # advance dev to the latest described, non-empty change and push
jj pull       # fetch origin, update dev, and rebase onto it
```

Changes are snapshotted automatically; there is no staging area. `jj push` skips
empty or undescribed changes and forwards options such as `--dry-run` to
`jj git push`. `jj pull` rebases the working copy, so run it when shared work is
ready to move. The agent workflow is documented in
[the jujutsu skill](./.agents/skills/jujutsu/SKILL.md).

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

Start the desktop app with `bun run dev`, after building rbun's Bun once as shown in
[Building from source](#building-from-source).

On macOS, the development runner builds with Tauri and uses Cargo's runner hook
to launch a signed, immutable `.app` snapshot through Launch Services. The executable
and embedded Bun dylib are copied out of the build directories; the copy's Bun
load command points directly to its bundled dylib. Rebuilding Chord or rbun while
the app is running cannot replace these files. Vite hot reload still works.
Running `bun run dev` inside `apps/chord-tauri` uses the same launcher and starts Vite.
Raw `cargo run` or `tauri dev` bypasses this launcher and runs the build output directly.

Snapshots live in `~/Library/Application Support/<development identifier>/runtime-snapshots/`,
outside Cargo's target directory. `current` is updated atomically after signing and
verification; CLI installation points through it so CLI links survive rebuilds.
Every app and CLI worker holds a shared runtime lease. Collection removes only
non-current snapshots whose lease can be acquired exclusively. Do not manually
remove a snapshot while it is in use. The runner forwards stop signals to the
specific launched app after verifying its PID's executable path.

The default signing identity comes from `bundle.macOS.signingIdentity`; override it
with `CHORD_DEV_SIGNING_IDENTITY` or `APPLE_SIGNING_IDENTITY`. A certificate-backed
identity is required; the launcher does not fall back to ad-hoc signing. Grant
Accessibility to **Chord (Development)** once after switching to this launcher.
Warp's grants no longer authorize the app. Chord continues to stop interception
whenever access is unavailable and resumes after a grant. Permission logs report
executable replacement separately, rather than claiming every failed check was a
revocation.

Chord needs only Accessibility. Its event tap is an active filter, which macOS
authorizes through Accessibility, and nothing reads raw HID input. macOS also
reports Input Monitoring as granted to an app that only has Accessibility.

Verify the snapshot launcher on a Mac with the configured signing certificate and
an active desktop session using
`bun test ./packages/dev/improve/chord/configs/dev/tauri/runtime.test.ts`.
The tests exercise signed executable/library isolation, generation leases,
PID-safe signaling, and Launch Services output/shutdown. Rust's runtime-worker
lease and permission-state tests are included in `cargo test --lib`.

Compare isolated package workers with the previous in-process Bun runtime using
`bun apps/chord-tauri/src-tauri/tests/package-isolation/benchmark.ts` from the root.
It builds optimized binaries and records paired latency, throughput, startup, and
resource measurements. See [the benchmark guide](./apps/chord-tauri/src-tauri/tests/package-isolation/README.md#performance-comparison)
for cases, options, and interpretation.

On macOS, a new development app launch replaces the running instance with the
same development bundle identifier. Startup logs the previous PID, sends SIGTERM,
and waits for it to exit before starting the new instance. If it ignores SIGTERM
for two seconds, startup verifies its socket ownership and sends SIGKILL. Concurrent launches
are serialized. CLI chord commands still forward to the running app, and
production/beta launches retain the existing instance.

### Input ownership across app instances

Each process registers at most one keyboard event tap.
All app channels share an OS-held input lease. Development bundle identifiers
(`com.leonsilicon.chord.development` and its dotted suffixes) take priority over
production and other channels. Among instances of the same priority, the current
owner stays active until it exits or becomes ineligible.

An instance must have Accessibility enabled and its event tap running before
requesting ownership. Standby instances forward keyboard
events untouched and leave the Caps key to macOS. On handoff, Chord drains in-flight
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

### Caps key ownership

macOS toggles Caps Lock in its HID filter, separately from the event stream that
Chord's tap sees, so the two cannot be ordered. Instead, the input owner remaps
the physical Caps key (usage `0x700000039`) to F20 (`0x70000006F`) in the
system-wide `UserKeyMapping` (`app/keyboard/caps_remap.rs`). macOS then never
toggles Caps Lock on its own. The key reaches the tap as F20, in order with Space,
and the tap swallows it. Chord is the only writer of the toggle
(`IOHIDSetModifierLockState`); `caps_lock_guard.rs` holds the rules. Chord edits
only its own entry. Other Caps entries it displaces are restored when it lets go,
unless the Caps key has been mapped again since.

The mapping outlives the process that set it. Taking ownership records the holder
(PID and start time) and the displaced entries in
`com.leonsilicon.chord/caps-remap-v1/state.json` in the application-support
directory, shared across channels and guarded by `state.lock`. Giving up ownership
or quitting removes the entry and the file. For crashes, Chord registers a launchd
agent (`~/Library/LaunchAgents/<identifier>.caps-guardian.plist`; development
variants share `com.leonsilicon.chord.development.caps-guardian`). It runs
`chord __caps-guardian --state <file>` and is kept alive exactly while the state
file exists, including at the next login. The guardian waits on the holder's exit
and on the state directory with kqueue, removes a dead holder's mapping, and exits
when the file is gone. launchd throttles respawns to one every 10 seconds, so the
guardian can start up to 10 seconds late after a quick relaunch. Its log is
`<label>.log` next to the state file. A development launch re-registers the agent
when the executable path changes.

Secure Input hides keys from every event tap, so the owner also registers a Carbon
hotkey for F20 (`caps_hotkey.rs`). It only fires while the tap cannot see the key;
the tap swallows it otherwise. A Caps Lock toggle that macOS makes itself (keycode
57 in the tap) means another app removed the mapping, and Chord installs it again.
UTM's "Caps Lock as key" mode does this on launch.

Apps learn about Caps Lock changes from `flagsChanged` events. A real toggle sends
one with keycode 57. `IOHIDSetModifierLockState` instead sends one with keycode 255
from every keyboard, then another with the next key's code just before that key.
Chromium ignores these and Safari reports an "Unidentified" key. While Chord owns
input, the tap rewrites the first report of each change (a `flagsChanged` event in
which only the Caps Lock flag changed) to keycode 57, and swallows the repeats. Every
app then sees what a real toggle would send, whichever app changed the state.

VoiceOver only recognizes the physical Caps key as its modifier. Chord behaves the
same for VoiceOver users. When VoiceOver has been set up with Caps Lock as a
possible modifier (`SCRKeysToUseForVOModifier` in `com.apple.VoiceOver4/default` is
not 0), the Activation Trigger settings card recommends Control-Option instead
(`app/keyboard/voice_over.rs`).

To check the mapping by hand, read `UserKeyMapping` from an
`IOHIDEventSystemClient` with no service. `hidutil property --get UserKeyMapping`
lists each keyboard's copy instead.

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

After changing IPC methods, regenerate the frontend bindings without launching
the app with `cargo test --manifest-path apps/chord-tauri/src-tauri/Cargo.toml --lib export_typescript_bindings -- --ignored`.
Run `bun run gen-types` after changing observable or model types.

The project tool versions are declared in [`.prototools`](./.prototools). Rust is also pinned in [`rust-toolchain.toml`](./rust-toolchain.toml) because proto delegates Rust version selection to rustup.

## File-picker chord context

Native macOS Open/Save panels temporarily give `com.apple.finder` chords priority
over the foreground app's chords, followed by global chords. The real app ID is
preserved for handlers and shortcut delivery, and the chord panel shows the same
Finder bindings. The Finder package must be installed or loaded from a monorepo.

An Accessibility observer listens for window/sheet creation, focus changes and
destruction. Workspace activation/lifecycle notifications maintain observers for
the foreground app and Open/Save panel services. Each event refreshes cached
context; keyboard dispatch reads that cache without querying Accessibility or
running a polling timer. Initial attachment also recognizes an already-open panel.
Revoking Accessibility access releases the observers and clears the context;
granting it attaches them again.

The listener lives in `app/frontmost/file_picker/native.rs`, using `objc2`
framework bindings. The Caps key mapping and toggle state live in
`app/keyboard/caps_remap.rs` and `caps_lock.rs`. These components and their
fixtures are Rust; Chord no longer compiles handwritten C or Objective-C sources.

Detection uses AppKit's nonlocalized `open-panel` / `save-panel` identifiers on
the focused picker or its containing sheet, so ordinary alerts do not activate
Finder chords. Custom file browsers without those identifiers are not recognized.
Finder bindings that explicitly launch or script Finder retain that behavior.

See the [native fixture](apps/chord-tauri/src-tauri/tests/file-picker/README.md)
for listener validation and `cargo test --lib file_picker_tests` for scope tests.

## Editing an installed chordpack locally

Chord's core `0` bindings live in the sibling checkout at
`../chords/@keychord/chords-chord/chords/com/leonsilicon/chord/macos.toml`.
Development builds and reloads read that file directly. Keep its distribution copy
at `apps/chord-tauri/data/chords-chord/chords/com/leonsilicon/chord/macos.toml`
in sync; builds without the sibling checkout embed that copy. The core file is
always loaded globally and cannot be disabled or replaced by another package source.

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
be linked to a local checkout using its link icon. **Add Repo** automatically
detects a monorepo when it contains `chords-*` directories. Chord discovers these
folders directly below the root or nested in other folders; no `packages/`
directory is required. Each matching folder is its own package, named by
its `package.json` name or, when absent, its folder name. Only the packages ticked
under **Packages to use** in the monorepo's entry are loaded; `@keychord/chords-chord`
is always ticked, and an empty selection loads nothing. A matching folder must
contain `package.json` or a `chords/`, `js/`, or `bin/` directory; empty folders
and leftover build folders are ignored. Discovery stops at each package and
skips hidden directories (including `.git`, `.jj`, `.moon`, and `.build`),
`node_modules`, and `target`, so vendored dependencies and build outputs are
not treated as separate packages. Repos with their own root `chords/` folder
continue to load as standalone packages.

Local links persist across restarts. **Reload** rescans the monorepo, including
new or removed packages, and rebuilds the JS runtime. **Unlink** stops loading a
standalone local monorepo or restores the cached GitHub source for a linked repo;
it leaves local files intact.

Packages with the same name are overridden in this order (last wins): default
config packages, cached GitHub sources, standalone local monorepos, locally
linked GitHub sources, and individual local folders. All local sources therefore
take precedence over remote GitHub packages. Paths and repo slugs are sorted within each group for
consistent results, so a GitHub monorepo such as `KeyChord/chords` does not replace the default
chordpack's pinned `KeyChord/chords-menu` or `KeyChord/chords-com.apple`; remove those repos to use
the monorepo's copies. Duplicate package names within one monorepo are rejected.

## Stores

There can be many owners of Observables (e.g. the AppHandle needs to `.manage` it so we can read the current state when initializing a window, and certain structs should be able to own it in order to modify it).

## JavaScript runtime

Chord runs all JS handlers on Bun, embedded through the
[`rbun`](https://github.com/KeyChord/rbun) crate checked out next to this repo
at `../rbun`. Packages can use Bun and Node APIs and load native code through
Node-API add-ons with `process.dlopen`. The build links `libbun_embed.dylib`
from the rbun checkout, so build it once:

```sh
# one-time: build Bun and libbun_embed.dylib in the rbun checkout (~20 min cold);
# install the prerequisites from rbun's README first
(cd ../rbun && bun install && bun run _build-bun)

bun run dev
```

The runtime integration lives in `apps/chord-tauri/src-tauri/src/bun_js/` and exposes the
`chord` module to packages. The desktop app supervises one child process per
installed JavaScript package; it never executes package JavaScript or loads
package native add-ons in the GUI process. The hidden `__package-worker` CLI
command owns each persistent VM and its native UI event loop. State and keyboard
host APIs use authenticated, bounded IPC. A stable supervisor replaces crashed
workers and restores handler factories in registration order without configuration
reload. Handler IDs remain valid; failed actions are not replayed. Repeated failures
use exponential backoff from 250 ms to 30 seconds, resetting after a healthy minute. See the
[isolation regression](apps/chord-tauri/src-tauri/tests/package-isolation/README.md).
The CLI uses the same embedded runtime:

```sh
apps/chord-tauri/src-tauri/target/debug/chord bun ./script.ts
apps/chord-tauri/src-tauri/target/debug/chord run ./script.ts export-name arg1 arg2
```

## Releases

Every push to `beta` or `prod` runs [`.github/workflows/release.yaml`](.github/workflows/release.yaml),
which builds the app and uploads the DMGs to the rolling release of the same name (Apple Silicon, plus
Intel when a matching Intel Bun build exists; see below). `beta` is published as a prerelease;
[`prod`](https://github.com/KeyChord/Chord/releases/tag/prod) is marked as the repository's latest
release, so `https://github.com/KeyChord/Chord/releases/latest` always serves production. Beta uses
the `com.leonsilicon.chord.beta` application identifier, while local development uses
`com.leonsilicon.chord.development` and production uses `com.leonsilicon.chord`. This keeps each
channel's settings, packages, caches, logs, and other application-scoped data separate. Each run
replaces the previous assets, so the download URLs stay stable.

To promote a build to production, push its commit to `prod`, for example the current `dev`:

```sh
jj bookmark set prod -r dev --allow-backwards
jj git push -b prod
```

CI cannot build the vendored Bun itself — that takes ~30 minutes. Instead
[`KeyChord/rbun`](https://github.com/KeyChord/rbun) builds `libbun_embed.dylib` once per Bun source
commit and publishes it as a `bun-embed-<sha>` release asset; Chord's workflow downloads it and
points the build at it with `RBUN_BUN_LIB_DIR` plus a generated overlay config that overrides
`bundle.macOS.frameworks`. Both repos are checked out side by side so that the relative `rbun` path
in `Cargo.toml` resolves — the dylib satisfies the linker, but cargo still needs rbun's source
manifest. The workflow downloads the release for the Bun commit that rbun's checkout pins, because
Chord links against that commit's embedding API. After bumping the Bun submodule in rbun, let its
`build-bun-embed` workflow finish before the next Chord release build. The Intel leg is optional:
rbun's Intel build of Bun does not currently finish within its job limit, so without a matching
Intel dylib the release ships Apple Silicon only.

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
| `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Produces the signed updater archive (`Chord-<arch>.app.tar.gz` and its `.sig`) |

The updater artifact is only built when `TAURI_SIGNING_PRIVATE_KEY` is set: Tauri fails the build if
an updater bundle is requested while `plugins.updater.pubkey` is configured without a private key.
Generate the keypair with `bun tauri signer generate -w ~/.tauri/chord.key`.
