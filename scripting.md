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

# Native code

Every chord handler is a JavaScript module, and Chord runs it on an embedded [Bun](https://bun.sh) runtime (the [`rbun`](https://github.com/KeyChord/rbun) crate). When JavaScript cannot reach an API you need, your package ships a **prebuilt native library** and opens it from the handler with Bun's [`bun:ffi`](https://bun.sh/docs/api/ffi) — in-process, on the hot path, with no helper process and nothing to compile at runtime. There is no Chord SDK on the native side: your Swift (or C, C++, Objective-C, Rust, Zig, …) code calls AppKit, Accessibility, Core Graphics or anything else directly and exports plain C functions.

> **Trust:** native code runs inside Chord with your full user permissions. Only enable packages from sources you trust.

## Package layout

```
my-package/
├── chords/macos.toml
├── src/
│   ├── js/menu.ts                 # the handler: opens the library with bun:ffi
│   ├── swift/menu.swift           # one native module per file; exports @_cdecl functions
│   ├── swift/_shared/**           # compiled into every module
│   ├── swift/menu/**              # compiled only into `menu`
│   └── {c,cpp,objc,objcxx}/...    # optional companion sources, linked into the module
├── js/menu.js                     # portable JS build output
└── target/                        # platform-specific native build output — commit/publish it like js/
    └── aarch64-apple-darwin/
        └── native/
            └── menu/
                ├── menu.dylib                           # the library
                ├── MyPackageNativeMenu.swiftmodule      # importable by other packages' Swift
                ├── MyPackageNativeMenu.swiftdoc
                └── MyPackageNativeMenu.swiftinterface
```

`src/` is authored source, `chords/` is Chord configuration, `js/` holds portable JS artifacts and `target/<triple>/` holds native artifacts. The source language never appears in the output path: `menu` is the logical compiled module; Swift, C++ and Objective-C are merely its inputs. Make sure your `.gitignore` does not ignore `target/`.

## Writing one

The native side exports whatever C ABI you like. Keep it small and string-based; return errors as strings the JS side frees:

```swift
// src/swift/beep.swift
import AppKit

/// Returns NULL on success or a strdup'ed error message (release with `beep_free`).
@_cdecl("beep_run")
public func beepRun(_ times: Int32) -> UnsafeMutablePointer<CChar>? {
    guard times > 0 else { return strdup("times must be positive") }
    for _ in 0..<times { NSSound.beep() }
    return nil
}

@_cdecl("beep_free")
public func beepFree(_ message: UnsafeMutablePointer<CChar>?) { free(message) }
```

The handler locates the library through the `chord` module — never by a hardcoded path — and calls it with `bun:ffi`:

```ts
// src/js/beep.ts
import { resolveNativeLibrary } from "chord";
import { CString, dlopen, FFIType } from "bun:ffi";

const lib = dlopen(resolveNativeLibrary(import.meta, "beep"), {
  beep_run: { args: [FFIType.i32], returns: FFIType.ptr },
  beep_free: { args: [FFIType.ptr], returns: FFIType.void },
});

export default function build(times = 1) {
  return function beep() {
    const error = lib.symbols.beep_run(times);
    if (error) {
      const message = new CString(error).toString();
      lib.symbols.beep_free(error);
      throw new Error(message);
    }
  };
}
```

- `resolveNativeLibrary(import.meta, "beep")` returns the absolute path of `target/<Chord's triple>/native/beep/beep.dylib` (`.so`/`.dll` elsewhere) for the calling module's package, including when the package is vendored inside another one. `resolvePackageFile(import.meta, "any/relative/path")` does the same for arbitrary package files.
- Handlers run on Chord's JS worker thread, not the main thread. AppKit calls that require the main thread must hop there (`DispatchQueue.main.sync { … }` — the app's run loop is running, so this returns promptly). The Accessibility API can be called from any thread.
- A thrown error (as above) is reported as a handler error. A crash in native code (`fatalError`, a bad pointer, `exit()`) takes Chord down with it — there is no separate process any more — so keep the exported surface small and validate inputs.
- Once opened, a library stays loaded until Chord quits; reloading packages picks up changed JS, but a rebuilt `.dylib` needs a restart.
- `print` output goes to Chord's stdout.

## Declaring it

Nothing special: it is a JS handler.

```toml
[on.beep]
file = "beep.js"
args = [2]

[chords."-b"]
"emit:beep" = []
```

(The pre-`bun:ffi` `kind = "native"` declaration is rejected with a pointer to this section.)

## Importing modules from other packages

Every module is built with a deterministic Swift module name: the package name and module name in PascalCase joined by `Native`, e.g. `@keychord/chords-menu` + `menu` → `KeychordChordsMenuNativeMenu`. Vendoring a package (`config({ vendor: ["@keychord/chords-menu"] })`) copies its `target/` next to its `js/` and `chords/` and makes its modules importable and linked from your own Swift:

```swift
import KeychordChordsMenuNativeMenu   // the Swift equivalent of importing @keychord/chords-menu/js/menu.js

try KeychordChordsMenuNativeMenu.runMenuAction(processName: "Safari", action: "by-letters", value: "f")
```

Declarations you want to expose must be `public`. Modules are built with library evolution and ship a `.swiftinterface`, so they can be imported by packages built with a different Swift compiler. From JavaScript, simply import the vendored package's JS (`@keychord/chords-menu/js/menu.js`); its own `resolveNativeLibrary` call finds the vendored library.

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

Any other build system works too: Chord only cares that `target/<triple>/native/<name>/<name>.dylib` exists and exports the symbols your JS declares in `dlopen`.

## Testing without the app

A Chord build's CLI runs a script on the same embedded Bun, with the `chord` module available (outside the app, `resolveNativeLibrary` anchors on the nearest `package.json`):

```sh
chord run scripts/run.ts by-letters f     # from the chords-menu checkout
```

## Troubleshooting

- `dlopen` failures name the missing file or symbol; check that `target/<your triple>/` was built and committed, and that the `@_cdecl` names match the `dlopen` declarations.
- `bun:ffi` needs the Bun engine (the default). With QuickJS selected in Settings → General → JavaScript Engine the import fails at load time.
- Accessibility calls inherit Chord's Accessibility permission — grant it to Chord.
- Native crashes crash Chord: run the handler from the CLI first while developing.

## FAQ

### Why Bun rather than a lighter runtime?

Chord started on QuickJS (still available as a fallback engine). Bun brings JavaScriptCore's JIT, Node/Bun APIs, and `bun:ffi` — which is what lets packages load native code without Chord having to know anything about it. Bun has no official embedding API, so Chord embeds it through the [`rbun`](https://github.com/KeyChord/rbun) crate, which exposes an rquickjs-shaped Rust API (and the custom `chord` module) over a lightly patched Bun runtime.
