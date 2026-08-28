# JavaScript

## Recommended Setup

TODO

## Recommended Packages

- [bplist-lossless](https://github.com/leonsilicon/bplist-lossless) - A binary plist parser specifically tailored for edits by avoiding loss of precision during parsing and re-serialization.
- [doctor-json](https://github.com/privatenumber/doctor-json) - A JSON editor that preserves all existing formatting/comments
- [keycode-ts2](https://github.com/leonsilicon/keycode-ts2) - A TypeScript port of the [Rust `keycode` crate](https://crates.io/crates/keycode) which uses the Chromium keycode names as the source of truth (_Chords_ uses these keycode names as the source of truth).

## The built-in `chord` module

The built-in `chord` module is intentionally kept minimal.

```ts
export class Applescript {
	constructor(source: string);
	constructor(fn: (...args: unknown[]) => unknown, ...args: unknown[]);
	compile(): void;
	execute(): unknown;
}

export function setAppNeedsRelaunch(bundleId: string, needsRelaunch: boolean): void;

export function resolveNativeModulePath(meta: ImportMeta, relpath: string): string;
export function resolvePackageFile(meta: ImportMeta, path: string): string;
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

Every chord handler is a JavaScript module, and Chord runs it on an embedded [Bun](https://bun.sh) runtime via the [`rbun`](https://github.com/KeyChord/rbun) crate. Native Swift is exposed as a Node-API add-on built with [NodeSwift](https://github.com/kabiroberai/node-swift):

`src/swift/menu/menu.swift` → `target/<triple>/menu/menu.node`

`@keychord/config` discovers and compiles this layout automatically. It publishes NodeSwift's dynamic `libNodeAPI.dylib` beside each `.node` file. Keeping that runtime dynamic lets Chord safely load add-ons from several packages in one process; the package does not need an npm dependency on NodeSwift at runtime.

> **Trust:** native code runs inside Chord with your full user permissions. Only enable packages from sources you trust.

## Package layout

```text
my-package/
├── chords/macos.toml
├── src/
│   ├── js/menu.ts                 # resolves and loads the Node-API add-on
│   └── swift/
│       ├── _shared/**             # compiled into every add-on
│       └── menu/
│           ├── menu.swift         # entry point: exports #NodeModule
│           └── helpers.swift      # optional additional Swift source
├── js/menu.js                     # portable JavaScript build output
└── target/                        # platform-specific output — commit/publish it like js/
    └── aarch64-apple-darwin/
        └── menu/
            ├── menu.node          # Node-API add-on
            └── libNodeAPI.dylib   # shared NodeSwift runtime, found through @loader_path
```

`src/` is authored source, `chords/` is Chord configuration, `js/` holds portable JavaScript artifacts, and `target/<triple>/` holds native artifacts. Make sure your `.gitignore` ignores `.chord-native-build/` but does not ignore `target/`.

## Writing one

Import `NodeAPI` and declare the add-on exports with `#NodeModule`. NodeSwift converts supported Swift values and thrown errors at the Node-API boundary, so no C ABI or manual pointer ownership is needed:

```swift
// src/swift/beep/beep.swift
import AppKit
import NodeAPI

enum BeepError: Error {
    case invalidCount
}

#NodeModule(exports: [
    "beep": try NodeFunction { (times: Int) throws in
        guard times > 0 else { throw BeepError.invalidCount }
        for _ in 0..<times { NSSound.beep() }
        return try NodeUndefined()
    },
])
```

The handler resolves the add-on through the `chord` module—never with a hardcoded target triple—and loads it with Bun's Node-compatible `process.dlopen`:

```ts
// src/js/beep.ts
import { resolveNativeModulePath } from "chord";

type BeepAddon = { beep(times: number): void };
let addon: BeepAddon | undefined;

function openAddon(): BeepAddon {
  const module = { exports: {} as BeepAddon };
  process.dlopen(module, resolveNativeModulePath(import.meta, "beep"));
  return module.exports;
}

export default function build(times = 1) {
  return function beep() {
    addon ??= openAddon();
    addon.beep(times);
  };
}
```

- `resolveNativeModulePath(import.meta, "beep")` returns the absolute path of `target/<Chord's triple>/beep/beep.node` for the calling module's package, including when the package is vendored inside another one. `resolvePackageFile(import.meta, "any/relative/path")` does the same for arbitrary package files.
- Handlers run on Chord's JavaScript worker thread, not the main thread. The Accessibility client API can be called there. Native UI APIs that require the main thread need appropriate dispatching; avoid synchronously hopping to a main thread that is not running a UI loop when exercising the handler through the CLI.
- A thrown Swift error is reported as a JavaScript handler error. A crash in native code (`fatalError`, a bad pointer, `exit()`) takes Chord down with it—there is no separate process—so keep the exported surface small and validate inputs.
- Once opened, an add-on stays loaded until Chord quits; reloading packages picks up changed JavaScript, but a rebuilt `.node` file needs a restart.
- `print` output goes to Chord's stdout.

## Declaring it

Nothing special: it is a JavaScript handler.

```toml
[on.beep]
file = "beep.js"
args = [2]

[chords."-b"]
"emit:beep" = []
```

The old `kind = "native"` declaration is rejected with a pointer to this section.

## Using native handlers from other packages

Vendor the dependency with `config({ vendor: ["@keychord/chords-menu"] })` and import its JavaScript normally. Vendoring copies the dependency's `target/` tree alongside its `js/` and `chords/`; the dependency's own `resolveNativeModulePath` call automatically maps to `target/@keychord/chords-menu/target/<triple>/menu/menu.node`. Add-ons are consumed through their JavaScript API, not by importing their generated Swift modules.

## Building

Packages built with `@keychord/config` compile `src/swift` automatically as part of `vp pack` (and rebuild on change in `vp pack --watch`). To build every distributable folder at once—`js/` and `target/<triple>/`—run `vpr compile` (`--triple <triple>` to cross-build, `--skip-js`/`--skip-native` for one half, `vpr -r compile` for the whole workspace). Building needs a Swift toolchain from Xcode or the Command Line Tools. Options go in `vite.config.ts`:

```ts
import { config } from "@keychord/config";

export default config({
  native: {
    triples: ["aarch64-apple-darwin", "x86_64-apple-darwin"], // default: host triple
    targets: {
      menu: {
        frameworks: ["ApplicationServices"], // imports autolink; list extras here
        swiftFlags: [],
        linkerFlags: [],
      },
    },
  },
});
```

The generated SwiftPM package and NodeSwift dependency build are shared across the workspace under `.chord-native-build/`; each package still gets an independent content cache and committed output. `@keychord/config` currently supports Apple triples.

Any other build system works too: Chord only cares that `target/<triple>/<path>/<name>.node` is a Node-API add-on compatible with Bun's `process.dlopen` and that its dynamic dependencies are resolvable. For the standard NodeSwift build, `libNodeAPI.dylib` sits beside the add-on and is found through `@loader_path`.

## Testing without the app

A Chord build's CLI runs a script on the same embedded Bun, with the `chord` module available (`resolveNativeModulePath` anchors on the nearest `package.json`):

```sh
chord run scripts/run.ts by-letters f     # from the chords-menu checkout
```

## Troubleshooting

- `process.dlopen` failures name the missing file or Node-API registration problem; check that the add-on and `libNodeAPI.dylib` under `target/<your triple>/` were built and committed and that the `#NodeModule` export names match the TypeScript binding.
- Node-API add-ons need the Bun engine (the default). With the legacy QuickJS engine selected in Settings → General → JavaScript Engine, loading the add-on fails.
- Accessibility calls inherit Chord's Accessibility permission—grant it to Chord.
- Native crashes crash Chord: run the handler from the CLI first while developing.

## FAQ

### Why Bun rather than a lighter runtime?

Chord started on QuickJS, which remains selectable as the legacy engine. Bun brings JavaScriptCore's JIT and Node/Bun APIs, including the Node-API loader used by NodeSwift add-ons. Bun has no official embedding API, so Chord embeds it through the [`rbun`](https://github.com/KeyChord/rbun) crate, which exposes an rquickjs-shaped Rust API (and the custom `chord` module) over a lightly patched Bun runtime.
