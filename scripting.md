# JavaScript

## Recommended Setup

Use [pnpm] because it supports specifying a dependency which is subdirectory of a GitHub repository, which is necessary since `LLRT` doesn't have an npm package for its types:

```jsonc
// package.json
{
	"devDependencies": {
		"llrt-types": "github:awslabs/llrt#path:/types"
		// ...
	}
}
```

Make sure your `tsconfig.json` has the `types` property set to `llrt-types`:

```jsonc
// tsconfig.json
{
	"compilerOptions": {
		"types": ["llrt-types"]
		// ...
	}
}
```

## Recommended Packages

- [nano-spawn-compat](https://github.com/leonsilicon/nano-spawn-compat) - A more ergonomic `child_process.spawn` that works in LLRT.
- [bplist-lossless](https://github.com/leonsilicon/bplist-lossless) - A binary plist parser specifically tailored for edits by avoiding loss of precision during parsing and re-serialization.
- [doctor-json](https://github.com/privatenumber/doctor-json) - A JSON editor that preserves all existing formatting/comments
- [keycode-ts2](https://github.com/leonsilicon/keycode-ts2) - A TypeScript port of the [Rust `keycode` crate](https://crates.io/crates/keycode) which uses the Chromium keycode names as the source of truth (_Chords_ uses these keycode names as the source of truth).

## `chord`

The built-in `chord` module also exposes:

```ts
export class Applescript {
	constructor(source: string);
	constructor(fn: (...args: unknown[]) => unknown, ...args: unknown[]);
	compile(): void;
	execute(): unknown;
}

export function setAppNeedsRelaunch(bundleId: string, needsRelaunch: boolean): void;
```

This marks or clears an app in the settings UI and gives the user a one-click relaunch button.

`Applescript` uses the macOS `OSAKit` framework via the Rust `osakit` crate. Call `compile()` before `execute()`.
When constructed with a string it runs AppleScript source directly. When constructed with a function, Chord serializes the trailing args, uses `fn.toString()`, wraps it as JXA, and executes it as `JavaScript` in OSAKit.

## URL Scheme

Chord registers the `chord:` URL scheme on macOS so scripts and launcher tools can trigger app actions.

### Commands

- `settings`
- `open-settings`
- `show-settings`
- `reload-config`
- `reload-configs`

### Examples

```sh
# Open the settings window
open --background 'chord:settings'

# Open the settings window (host-style form)
open --background 'chord://settings'

# Reload chord configs
open --background 'chord:reload-config'
```

## CLI

This repo also includes a small `chord` CLI wrapper that forwards commands to the `chord:` URL scheme.

### Commands

- `settings`
- `open-settings`
- `show-settings`
- `reload-config`
- `reload-configs`

### Examples

```sh
./chord settings
./chord reload-configs
```

If you want to run it as `chord` from anywhere, add the repo copy to your `PATH` or symlink it into a directory that is already on your `PATH`.

The CLI depends on macOS recognizing the bundled Chord app as the handler for the `chord:` URL scheme, so the app bundle needs to be built and launched at least once first.

# Native handlers

Chord can run **prebuilt native libraries** as chord handlers. They are written in Swift (plus any C, C++, Objective-C or other code you link in), compiled ahead of time by the package build, and executed by a persistent helper process, `chord-native-host`, that Chord starts alongside itself. On a keystroke Chord sends one small message over a local socket and the helper calls the already-loaded function — no compiler, no process spawn, no `dlopen` on the hot path (a no-op handler round-trips in well under a millisecond).

Use native handlers when the embedded JavaScript runtime cannot reach an API you need. There is no Chord SDK to import: your Swift code calls AppKit, Accessibility, Core Graphics, Swift packages, C libraries, shell processes, or anything else directly.

> **Trust:** a native handler runs with your full user permissions, outside any sandbox. The separate process protects Chord from a handler that crashes; it does not protect you from a handler that misbehaves. Only enable native packages from sources you trust.

## Package layout

```
my-package/
├── chords/macos.toml
├── src/
│   ├── js/menu.ts                 # JS handlers (unchanged)
│   ├── swift/menu.swift           # one native module per file; defines `run`
│   ├── swift/_shared/**           # compiled into every module
│   ├── swift/menu/**              # compiled only into `menu`
│   └── {c,cpp,objc,objcxx}/...    # optional companion sources, linked into the module
├── js/menu.js                     # portable JS build output
└── target/                        # platform-specific native build output — commit/publish it like js/
    └── aarch64-apple-darwin/
        └── native/
            └── menu/
                ├── menu.dylib                           # the handler library
                ├── MyPackageNativeMenu.swiftmodule      # importable by other packages
                ├── MyPackageNativeMenu.swiftdoc
                └── MyPackageNativeMenu.swiftinterface
```

`src/` is authored source, `chords/` is Chord configuration, `js/` holds portable JS artifacts and `target/<triple>/` holds native artifacts. The source language never appears in the output path: `menu` is the logical compiled module; Swift, C++ and Objective-C are merely its inputs. Chord loads `target/<its own triple>/native/<name>/<name>.dylib` (`.dll`/`.so` on other platforms). Make sure your `.gitignore` does not ignore `target/`.

## Writing one

Each `src/swift/<name>.swift` defines exactly one function:

```swift
import AppKit

func run(_ handlerArguments: [String], _ eventArguments: [String]) throws {
    NSSound.beep()
}
```

- `handlerArguments` are the static `args` from the handler declaration; `eventArguments` come from the chord's `emit:*` value (regex captures such as `$1` are already substituted). Both are strings: TOML strings pass through unchanged, numbers/booleans use their text form, arrays and tables arrive as compact JSON.
- Throwing an error reports it as a handler error and leaves the helper running. A `fatalError`, forced unwrap trap, segmentation fault or `exit()` kills the helper; Chord keeps running, logs the crash with the helper's stderr, and starts a new helper. A handler that crashes or hangs three times within ten seconds is disabled until the next package reload.
- A handler that does not return within 30 seconds is killed (which restarts the helper — there is one helper for all native handlers).
- Handlers run one at a time on the helper's main thread, so AppKit is usable directly. The helper does not run an `NSApplication` event loop; pump `RunLoop.main.run(until:)` yourself if you wait on run-loop callbacks.
- Module-level state persists for the lifetime of the helper (i.e. until the next package reload or crash).
- `print` output and stderr show up in Chord's log with the `native-host` target.

These environment variables are set for the duration of each call: `CHORD_PACKAGE_NAME`, `CHORD_CHORDS_FILE_PATHSLUG`, `CHORD_HANDLER_ID`, `CHORD_INVOCATION_ID`, and `CHORD_FOCUSED_APP_ID` (unset when no app is focused).

## Declaring it

```toml
[on.menu]
kind = "native"   # "js" (default) or "native"
file = "menu"     # logical module name -> target/<triple>/native/menu/menu.dylib. No extension.
args = ["Safari"]

[chords."-([a-z]+)"]
"emit:menu" = ["by-letters", "$1"]
```

`kind = "js"` (or no `kind`) keeps the existing JavaScript behaviour. A chords file may mix both kinds.

## Importing modules from other packages

Every module is built with a deterministic Swift module name: the package name and module name in PascalCase joined by `Swift`, e.g. `@keychord/chords-menu` + `menu` → `KeychordChordsMenuNativeMenu`. Vendoring a package (`config({ vendor: ["@keychord/chords-menu"] })`) copies its `target/` next to its `js/` and `chords/` and makes its modules importable and linked:

```swift
import KeychordChordsMenuNativeMenu   // the equivalent of importing @keychord/chords-menu/js/menu.js

func run(_ handlerArguments: [String], _ eventArguments: [String]) throws {
    try KeychordChordsMenuNativeMenu.run(["Safari"], ["by-letters", "f"])
}
```

Declarations you want to expose must be `public`. Modules are built with library evolution and ship a `.swiftinterface`, so they can be imported by packages built with a different Swift compiler.

## Building

Packages built with `@keychord/config` compile `src/swift` automatically as part of `vp pack` (and rebuild on change in `vp pack --watch`). To build every dist folder at once — `js/` and `target/<triple>/native/` — run `vpr compile` (`--triple <triple>` to cross-build, `--skip-js`/`--skip-native` for one half, `vpr -r compile` for the whole workspace). Building needs a Swift toolchain from Xcode or the Command Line Tools (`xcrun --find swiftc`). Options go in `vite.config.ts`:

```ts
import { config } from "@keychord/config";

export default config({
  native: {
    triples: ["aarch64-apple-darwin", "x86_64-apple-darwin"], // default: the host triple
    targets: {
      menu: {
        frameworks: ["ApplicationServices"], // imports are autolinked; list extras here
        bridgingHeader: "src/objc/menu/Bridging.h", // auto-detected when named Bridging.h
        cxxInterop: true,
        cxxFlags: ["-std=c++20"],
      },
    },
  },
});
```

Companion sources are compiled with `clang`/`clang++` (`.c`, `.cc/.cpp/.cxx`, `.m` with ARC, `.mm`) and linked into the module's library; expose them to Swift through a bridging header or a module map.

Any other build system works too: Chord only requires `target/<triple>/native/<name>/<name>.dylib` to export

```c
int32_t chord_native_run_v1(int32_t handler_argc, const char *const *handler_argv,
                            int32_t event_argc,   const char *const *event_argv,
                            uint8_t *error_buf,   size_t error_buf_cap);
// 0 ok · 1 threw (error_buf: NUL-terminated message) · 2 bad arguments · 3 other wrapper failure
```

so C, C++, Objective-C, Rust or Zig handlers are equally possible. The generated Swift wrapper that implements this lives in Chord's `crates/chord-native-protocol/swift/ChordEntry.swift`.

## Testing without the app

```sh
chord native-run target/aarch64-apple-darwin/native/menu/menu.dylib --handler-arg Safari --event-arg by-letters --event-arg f
chord native-bench target/aarch64-apple-darwin/native/noop/noop.dylib --iterations 10000
```

(`CHORD_NATIVE_HOST_BIN` points the CLI at a `chord-native-host` binary when it is not next to `chord`.)

## Availability and troubleshooting

- Native handlers require the full-power Chord build (no App Sandbox). A sandboxed build reports `native handlers are unavailable in the sandboxed build` for `kind = "native"` handlers.
- The helper inherits Chord's Accessibility permission. Grant it to Chord, not to the helper.
- Look for `native-host` and `native_host` entries in Chord's log: they include the helper PID, generation loads, load failures (missing `chord_native_run_v1`, wrong architecture), crashes with the helper's stderr tail, timeouts and crash-loop suppression.
- A package without `target/<your triple>/` artifacts fails at load time with `requires prebuilt artifacts for <triple>`; build for that triple (`native.triples`) or on that machine.

## FAQ

### Why not bundle a full-fledged runtime like Deno or Bun?

Deno has too much overhead, an experiment was previously tried but it makes the keypress handler lag significantly (maybe I embedded it wrong, but not worth the trouble of debugging).

Bun on the other hand is great, but doesn't have an official integration API, which makes it impossible to expose custom Rust functions (needed in order to synchronize state). It can still be used for one-off CLI tasks such as browser automation, though.
