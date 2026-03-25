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
export function setAppNeedsRelaunch(bundleId: string, needsRelaunch: boolean): void;
```

This marks or clears an app in the settings UI and gives the user a one-click relaunch button.

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

## FAQ

### Why not bundle a full-fledged runtime like Deno or Bun?

Deno has too much overhead, an experiment was previously tried but it makes the keypress handler lag significantly (maybe I embedded it wrong, but not worth the trouble of debugging).

Bun on the other hand is great, but doesn't have an official integration API, which makes it impossible to expose custom Rust functions (needed in order to synchronize state). It can still be used for one-off CLI tasks such as browser automation, though.
