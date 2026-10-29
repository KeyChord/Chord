# JavaScript

> [!NOTE]
> This document is largely AI-written, so parts of it may not be fully accurate. It will be replaced with a hand-written version when I get the time.

## Recommended Setup

A chord package is a folder with chord files in `chords/` and, optionally, JavaScript handlers. To start one, follow the quick start in [create-chord-package](https://github.com/KeyChord/create-chord-package). Packages build their handlers with [`@keychord/config`](https://github.com/KeyChord/config) and get typings for the `chord` module from [`@keychord/types`](https://github.com/KeyChord/types-monorepo). The packages in [KeyChord/chords](https://github.com/KeyChord/chords) are working examples.

While you work on a package, load its folder in **Settings → Chords → Local Folders**, or use **Link Local Folder** on an installed repo, and reload with `0,r` after each build.

## Recommended Packages

- [bplist-lossless](https://github.com/leonsilicon/bplist-lossless) - A binary plist parser specifically tailored for edits by avoiding loss of precision during parsing and re-serialization.
- [doctor-json](https://github.com/privatenumber/doctor-json) - A JSON editor that preserves all existing formatting/comments
- [keycode-ts2](https://github.com/KeyChord/keycode-ts) - A TypeScript port of the [Rust `keycode` crate](https://crates.io/crates/keycode) which uses the Chromium keycode names as the source of truth (_Chords_ uses these keycode names as the source of truth).

## Chord events and persistent return values

### Meta commands, repetition, and special keys

The leading `0` namespace belongs to Chord in every focused app. Its bindings and help
are defined in `@keychord/chords-chord/chords/com/leonsilicon/chord/macos.toml`.
This protected core file is always loaded, even before optional repositories load,
and cannot be removed, deselected, or overridden. Other files cannot define `0`
commands, hints, or `builtin` actions, and user regexes cannot intercept them.
Complete built-in commands with the normal chord completion controls or the CLI:

| Command | Action |
| --- | --- |
| `0,` | Open Chord Settings |
| `0,r` | Reload configuration and stop active loops |
| `0q` | Quit Chord |
| `0h<ID>` | Halt the loop with that numeric ID, for example `0h12` |
| `0b<digit><sequence>` | Save a temporary macro buffer, numbered 1–9 |
| `0=<name>=<sequence>` | Save a named macro; digit-prefixed names are temporary |
| `0ls` or `0lsl` | Use the left sidebar layout |
| `0lf` or `0lfb` | Use the bottom floating layout |
| `0lv` | Cycle between the layouts |

The selected layout persists across app launches. The compact floating layout fits its
contents near the bottom center of the display, with keys above the resolved action description.
Completed chords remain visible until you start another chord or leave chord mode.

The core file maps chords to native app operations using `builtin` actions:

```toml
[chords]
'0,' = { name = "Settings", builtin = { command = "settings" } }
'0h(\d+)' = { name = "Halt loop #$1", builtin = { command = "halt-loop", argument = "$1" } }
'0b([1-9].*)' = { name = "Save buffer", builtin = { command = "set-buffer", argument = "$1" } }
'0=([^=]+=.*)' = { name = "Save macro", builtin = { command = "set-macro", argument = "$1" } }
```

The supported operations are `settings`, `reload`, `quit`, `halt-loop`, `set-buffer`,
`set-macro`, `set-layout`, and `cycle-layout`. `set-layout` takes `sidebar-left` or
`floating-bottom` as its `argument`. Macro setters capture the entire payload after `0b` or `0=` in
their `argument`.
Only the protected core file has permission to use them. Installed builds bundle
the file so the controls remain available without external package sources.

Leading decimal digits set a chord's repeat count: `12ab` executes `ab` twelve times.
Without a prefix the chord executes once; the prefix `1` starts an indefinite loop.
For example, `1ab` keeps executing `ab` after the keys are released, chord mode ends,
or another chord runs. Each loop receives a distinct ID displayed in the chord panel
alongside its `0h<ID>` command. Halting a loop stops further iterations. An in-flight
JavaScript callback finishes before its values are released, preserving other handlers
in the same package worker. Shell commands are cancelled and simulated shortcut keys
are released. A failed iteration ends its loop; configuration reload and quitting stop
all loops. IDs are not reused during an app session.

Repeat counts apply to completed input actions; init and keyboard callbacks still run
once per matching interval or keyboard event. Regex capture arguments exclude the
repeat prefix: `12hsr34` runs `hsr34` twelve times with `$1` equal to `34`. Handlers
receive the underlying command as `this.input`, excluding the repeat prefix. Loop
iterations share their initialization values until halted. Iterations are sequential,
with a 10 ms pause between them so immediate actions leave time for other input.

A repeated or looped chord calls its built handler once per iteration; the builder
(the module's default export) still runs only when the configuration loads. Chord
selects the focused window once, when the completed input starts running, and passes it
to every iteration and every chord of the run as `this.focusedWindowId`. Use it, rather
than reading focus inside the handler, for actions that target a window: `3/ws]` then
moves one window three Spaces even though focus moves to another window after the first
move. Handlers that should follow focus, such as window cycling, can ignore it.

A config block in backticks may follow the repeat count, or start the input on its
own. It holds comma-separated options whose digits count hundreds of milliseconds:

- `d` (delay) takes one digit and waits that long once, before the first run. Loops and
  repeats never wait for it again. `` `d5`ab `` executes `ab` after 500 ms, whatever
  chord `ab` is.
- `i` (interval) takes one digit per pause between consecutive runs: `` 3`i5`ab ``
  executes `ab` three times, 500 ms apart. Several digits are used in order and repeat,
  and `0` executes the next chord immediately.

`` 3`d2,i130`/ws] `` waits 200 ms, executes `/ws]`, waits 100 ms, executes it again,
waits 300 ms, and executes it a third time. The final `0` only matters in a loop. Each
option appears at most once, in either order. Only a leading block is config; a
backtick later in the input is an ordinary key. Unclosed, unknown, or repeated
options stay pending.

After a repeat count or config block, Enter separates consecutive chords.
`` 3`i13`/w]⏎/w] `` (⏎ is the Enter key, `\n` in text) executes `/w]`, waits 100 ms,
executes `/w]` again, waits 300 ms, and repeats that sequence three times; `1ab⏎cd`
loops `ab` then `cd`. Everywhere else Enter is an ordinary key: it enters the chord
buffer and triggers can contain it. Every chord in a sequence
must resolve before anything runs, and each one keeps its own initialization values.
Loops, configured delays or intervals, and multi-chord sequences run in the background
with a halt ID, like loops; a plain repeat count such as `12ab` still runs in one
action. Halting during the delay cancels the run before anything executes. Handlers
target the window that was focused when the input completed, even after a delay.

A quote types text into the focused app instead of running a chord. Typing `'` at
the start of a step enters insert mode, similar to Vim. Chord stays active, so
you can release Space and type any text, including capitals (with Shift or Caps
Lock), spaces, symbols, Enter, and `'`. Caps Lock toggles as usual while
inserting; returning to chord mode turns it off again, and leaving Chord restores
the Caps Lock state from before Chord started. No key reaches other apps, and Backspace edits the text. Press
Caps+Space to close the quote and return to chord mode with Space held, or Esc to
cancel Chord and discard the whole input. After closing, continue the input as
usual: release Space to run it, or add a config block such as `` `i1` `` to type
one character every 100 ms (the same digits as above; without a block the text
is typed immediately). A quote's block takes only `i`; delay the whole input instead,
as in `` `d5`'hi' ``. A quote can stand alone, follow a repeat count (`3'hi'`),
or be one step of an Enter-separated sequence (`` 2`i5`'hi'⏎/w] ``). Backspace
over a closing quote reopens it in insert mode with its text. Elsewhere `'` is an
ordinary key. Text is typed with a US keyboard layout. In TOML and CLI text
notation, `'hello'` writes the keys between the quotes, unshifted.

Backslash introduces special-key notation: `\up`, `\left`, `\down`, and `\right`
represent arrow keys, and `\n` represents Enter, in TOML triggers and CLI sequences.
For example, `'a\up'` matches `a` followed by the Up arrow. In chord mode, a typed
escape never expands on its own: `\up` stays three keys, and the input stays pending,
until Tab replaces it with the highlighted suggestion. Any unambiguous prefix works,
so `\u` then Tab enters the Up arrow, as does `\up` then Tab. Numbered buffer
references (`\1`–`\9`) are the exception and need no Tab. Backslash is reserved for
special keys and macro references; bare or unknown escapes cannot define chords or
help hints. Inside parenthesized regex sections, backslash keeps its regex meaning
(for example, `(\d+)`). Use literal TOML strings (single quotes) to preserve
backslashes. Backspace continues to edit the chord buffer.

### Macros and temporary buffers

Complete `0b<digit><sequence>` to save a sequence in one of nine numbered buffers.
For example, `0b11720x864` saves `1720x864` in buffer 1, and `/wd\1` then executes
the same chord as `/wd1720x864`.

For a named macro, use `0=<name>=<sequence>`. For example, after
`0=window-dimensions=1720x864`, typing `/wd\window-dimensions=` enters `/wd1720x864`
as soon as the closing `=` is typed, and typing `/wd\win` and pressing Tab does the
same. Both insert the macro's complete expansion, so later redefinitions do not
change input that was already entered. In TOML or CLI text, a complete named
reference such as `a\window-dimensions=b` expands to `a1720x864b`. Names can
contain normal chord characters except `=` and backslash; they do not have to be
identifiers.

A named reference may also start with `=`, as in `\=window-dimensions=`. That form
is required when the name starts with a digit 1–9, because `\1`–`\9` are numbered
buffers: the named macro `\=1=` is separate from buffer `\1`. In text, special-key
escapes win, so `\notes=` is Enter followed by `otes=`; write `\=notes=` there
for a name starting with `n`, `up`, `left`, `down`, or `right`. Typed special keys
only apply on Tab, so typing `\notes=` always references the macro. When a special
key is typed exactly, such as `\n` or `\up`, Tab enters that key rather than a
longer macro name.

Typing `\` in chord mode shows matching macros and their saved sequences before
special-key escapes. Suggestions narrow as you type, including inside a longer
chord such as `/wd\win`. Both overlay layouts show these suggestions and highlight
the one Tab inserts: a special key typed exactly, otherwise the first named macro or
special key that matches.

Named macros persist across app restarts by default. Prefix a name with any digit
to make it temporary: `0=1window-dimensions=1720x864` defines `\=1window-dimensions=` for the
current session only. Numbered buffers `\1`–`\9` are always temporary. Leaving
chord mode, clearing the input buffer, and reloading configuration preserve all
definitions.

Settings → Macros lists the current definitions and lets you add, rename, edit,
and delete named macros or edit and clear numbered buffers. Renaming a named
macro to start with a digit removes its saved definition; renaming it back saves
it across restarts. Saving the same name or number overwrites its value; an empty
sequence, such as `0b1` or `0=window-dimensions=`, sets it to an empty expansion.

Macro bodies are saved verbatim and can reference other macros. Numbered buffer
references typed into a setter stay references; a named macro selected with Tab is
inlined. Expansion happens before repeat-prefix parsing, trigger matching, and
handler argument captures;
handlers receive the expanded command as `this.input`. Missing references keep
input pending. Recursive definitions and excessive expansion are rejected.

### Handler events

Chord actions use `@trigger:event`, with an optional `=identifier` to retain the handler's return value:

```toml
[on.prepare]
file = "prepare.js"

[on.update]
file = "update.js"

[chords.'j,(.*)']
name = "Live clicks"
'@init:prepare=context' = []
'@keydown:update=latest' = ['$context', '$1']
```

The event name refers to an entry in `[on]`. Every event specifier accepts an argument array and may assign a return value. Identifiers contain ASCII letters, digits, or underscores and start with a letter or underscore. An exact argument such as `$context` passes the stored JavaScript value; `$1`, `$2`, etc. continue to pass regex captures. Named references also work inside argument arrays and tables. Referencing an identifier before it has been assigned raises a `ReferenceError`.

Chord keys are literal except for parenthesized regex sections. For example, `hs.(\d+)` matches `hs.12` and captures `12` as `$1`; the dot outside parentheses needs no backslash. Regex syntax, including escaped dots, nested groups, alternatives, and quantifiers, belongs inside the parentheses. Patterns match the entire entered sequence automatically, so no outer `^` or `$` is needed. The same rule applies to regex help hints; compact help ranges retain their special display behavior.

Handlers receive the entered sequence as `this.input` (for example, `hsr12`), alongside `this.focusedAppId`, `this.focusedWindowId`, and `this.event`. This also works for CLI execution. `this.focusedWindowId` is the Core Graphics window number (the `windowId` of `getAppWindows`) of the focused application's focused, else main, window when the completed input started running. It is `null` for callbacks that run while typing (`@init` and keyboard events) and when no window can be identified.

| Trigger           | When it calls the handler                                                                     |
| ----------------- | --------------------------------------------------------------------------------------------- |
| `@init:event`     | Once when this chord starts matching, before its other event handlers.                        |
| `@keydown:event`  | After each keydown, using the buffer after any edit.                                          |
| `@keypress:event` | After the matching keydown handlers, for every key press, including Backspace and modifiers.  |
| `@keyup:event`    | After each key release, using the current buffer.                                             |
| `@input:event`    | When the chord is completed with Caps Lock, Space release, or the CLI.                        |

All matching lifecycle chords are visited in app/global and package priority order. Ordinary input completion keeps its first-match action priority. Initializers run before the other callbacks for a buffer snapshot, and callbacks are awaited in event order, including Promise return values. A callback failure stops the remaining callbacks for that snapshot and is reported through Chord's normal handler-error path.

Values belong to that chord's current matching interval. When the buffer stops matching, clears, or Chord mode ends, its initialization state and values are discarded. Matching again runs init again with fresh values. Different chords and focused apps have separate values. Returned objects, functions, and `undefined` retain their JavaScript identity. A later assignment to the same identifier replaces its value.

For example, `prepare.js` can return a state object and `update.js` can reuse it:

```js
// prepare.js
export default function buildHandler() {
  return async function prepare() {
    return { updates: 0 };
  };
}

// update.js
export default function buildHandler() {
  return function update(context, sequence) {
    if (this.event.key === "Backspace") return context.updates;
    context.updates += 1;
    return context.updates;
  };
}
```

`this.event` follows the [Web KeyboardEvent data model](https://www.w3.org/TR/uievents/#interface-keyboardevent): `type`, `key`, `code`, `location`, `ctrlKey`, `shiftKey`, `altKey`, `metaKey`, `repeat`, `isComposing`, and `timeStamp`. `getModifierState()` supports `Alt`, `Control`, `Meta`, and `Shift`. Non-character keys use names such as `Backspace`, `Escape`, and `Shift`; `code` identifies the physical key, such as `KeyJ` or `ShiftLeft`. Character names use Chord's existing key mapping. Repeated keydowns are delivered with `repeat: true` without adding duplicate buffer characters.

The snapshot extends Bun's `Event`, with read-only keyboard fields, `view: null`, and `detail: 0`. It has no DOM target or propagation path and is non-cancelable, so `preventDefault()` does not change native buffer handling. Composition events are not captured (`isComposing` is false). Init and input callbacks receive the originating keyboard event; `@keypress` callbacks receive `type: "keypress"`. CLI execution does not synthesize keyboard events and uses `this.event: null`.

Chord deliberately sends `@keypress` for every key, including Backspace, even though the browser's legacy `keypress` event has narrower behavior. Normal buffer edits and completion controls still apply; keyboard callbacks observe the resulting buffer.

### App IDs

An app's chords live in `chords/<app ID, with dots as folders>/macos.toml`. `this.focusedAppId`, `onAppLaunch`, `onAppTerminate`, the global hotkey functions, and `getAppWindows` use the same app ID, which is normally the app's bundle identifier. An app shipped as a bare executable, without an `.app` bundle, has none, so Chord uses, in order:

1. a `CFBundleIdentifier` embedded in the executable's `__TEXT,__info_plist` section;
2. the identifier of a certificate (not ad-hoc) code signature;
3. the app ID of a chords file that claims the executable's file name in `executables`.

Ad-hoc signing identifiers are ignored. The linker derives them from the link-time file name, so they change between builds (Cargo appends a hash) or collide (every Go binary is `a.out`). Claim such an executable from the chords file of the app ID its `.app` bundle uses, so the same file covers both:

```toml
# chords/org/livesplit/one/macos.toml
name = 'LiveSplit One'
executables = ['LiveSplit One']
```

`executables` lists file names, not paths. Only app chords files can claim executables, and when packages claim the same file name, the higher-priority package wins. Chord cannot relaunch a bare executable with `setAppNeedsRelaunch`'s button, because Launch Services opens one in Terminal.

## The built-in `chord` module

The built-in `chord` module is intentionally kept minimal. Handlers import it with `import { … } from "chord"`.

```ts
// Simulated keys and shortcuts, written like a chord file's `shortcut` (for example "cmd+shift+p").
export function tap(shortcut: string): void;
export function press(shortcut: string): void;
export function release(shortcut: string): void;

// Called whenever an app with this app ID (normally its bundle ID; see "App IDs") launches or quits.
export function onAppLaunch(
  bundleId: string,
  callback: (app: { pid: number; bundleId: string }) => unknown,
): void;
export function onAppTerminate(
  bundleId: string,
  callback: (app: { pid: number; bundleId: string }) => unknown,
): () => void; // removes the callback

// A shortcut from Chord's global hotkey pool, assigned to this app and ID (see the readme).
export function registerGlobalHotkey(bundleId: string, hotkeyId: string): string | undefined;
export function getGlobalHotkey(bundleId: string, hotkeyId: string): string | undefined;

// Runs a program as root after macOS asks for an administrator password. Output is discarded.
export function runSudoCommand(program: string, args: string[]): Promise<void>;

export class Applescript {
  constructor(source: string);
  constructor(fn: (...args: unknown[]) => unknown, ...args: unknown[]);
  compile(): void;
  execute(): unknown;
}

export function setAppNeedsRelaunch(bundleId: string, needsRelaunch: boolean): void;

export type WindowBounds = { x: number; y: number; width: number; height: number };
export function getAppWindows(bundleId: string): {
  windowId: number;
  processId: number;
  bounds: WindowBounds;
  title: string | null;
}[];
export function getScreenBounds(): WindowBounds;
export function mouse(options: {
  action: "click" | "down" | "move" | "up";
  windowId?: number;
  x?: number;
  y?: number;
  expectedBounds?: WindowBounds;
  button?: "left" | "right" | "middle";
  clickCount?: 1 | 2;
  pressDurationMs?: number;
}): Promise<void>;
export function click(options: {
  windowId: number;
  x: number;
  y: number;
  expectedBounds?: WindowBounds;
  moveCursor?: boolean;
  button?: "left" | "right" | "middle";
  clickCount?: 1 | 2;
  pressDurationMs?: number;
}): Promise<void>;
```

Chord also attaches a namespace to every module it loads, so path resolution is bound to the
calling module rather than passed in:

```ts
interface ImportMeta {
  chord: {
    resolveNative(relpath: string): string;
    resolvePackageFile(path: string): string;
  };
}
```

`setAppNeedsRelaunch` marks or clears an app in the settings UI and gives the user a one-click relaunch button.

`tap`, `press`, `release`, the global hotkey functions, `setAppNeedsRelaunch`, `onAppLaunch`, and `onAppTerminate` need the running app; under `chord bun` they throw. `registerGlobalHotkey` returns the existing assignment when there is one, and `undefined` when the pool is used up.

`Applescript` uses the macOS `OSAKit` framework via the Rust `osakit` crate. Call `compile()` before `execute()`.
When constructed with a string it runs AppleScript source directly. When constructed with a function, Chord serializes the trailing args, uses `fn.toString()`, wraps it as JXA, and executes it as `JavaScript` in OSAKit.

`getAppWindows` lists an already-running app's on-screen normal windows without launching or
activating it. Windows retain their IDs and geometry even when another app covers them. Titles
can be null when macOS does not expose them. Select the intended window before sending input.

`click` sends button down/up events to that window. It defaults to one left click;
`button` selects left, right, or middle, and `clickCount: 2` sends a double click.
`pressDurationMs` sets the hold duration (default 80 ms, accepted range 1–5000 ms). `x` and `y` are window-local macOS
logical screen points measured from its top-left corner; they are independent of Retina scaling.
The helper preserves the foreground app and, by default, the Mac pointer. `moveCursor: true`
moves the pointer to the target point after validation while events still go directly to the
window; it never clicks through the cursor. Optional `expectedBounds` rejects a
window that has moved or resized since coordinates were calculated. Invalid coordinates, missing
windows, denied Accessibility access, and unavailable private APIs reject the Promise.
These helpers work in both the app and `chord bun` CLI mode; they do not require an app context.

```ts
import { click, getAppWindows } from "chord";

const windows = getAppWindows("com.apple.ScreenContinuity");
if (windows.length !== 1) throw new Error("Select the intended Mirroring window");
const window = windows[0];
await click({
  windowId: window.windowId,
  x: window.bounds.width / 2,
  y: window.bounds.height - 20,
  expectedBounds: window.bounds,
});
```

Rust callers use `chords_lib::native_input::{get_app_windows, click, ClickOptions}`. The click
helper holds the button for 80 ms and should run on a worker thread. Chord's JS bridge does this
automatically. The macOS implementation uses `NSEvent`'s window number, both window-under-pointer
fields, mouse subtype 3, and Command on background events, stamps window-local coordinates through
the private `CGEventSetWindowLocation` symbol, then calls `CGEventPostToPid`. It never posts a mouse
move event or activates the target. An explicitly requested cursor move uses
`CGWarpMouseCursorPosition` before the window-targeted button events. It does not fall back to
cursor-based clicks if the SPI is missing.

`mouse` adds separate button-down, movement, and button-up operations. Supply
`windowId` for window-local input with the same targeting and geometry checks as
`click`; omit it for ordinary global macOS points. Global input reaches the visible
app under the pointer and can activate it. `getScreenBounds` returns the display
containing the pointer, using macOS logical points and including negative display
origins. Resolve all coordinates against a captured set of bounds before sending
input.

After `down`, `move` sends the corresponding left/right/middle dragged event.
`up` releases at the most recent successful point, even if the window has since
moved or disappeared. Mouse sessions belong to each package worker's host
connection; errors and worker disconnection release held buttons. A later normal
cursor-moving click releases a latched button before clicking. A `down` can stay
held across handler invocations; authors should otherwise release it in `finally`.
Standalone scripts share a session during their invocation and release it on exit.
This private recipe was verified on macOS 26.5.2; its availability and behavior can change with OS updates.

## URL Scheme

Chord registers the `chord:` URL scheme on macOS so scripts and launcher tools can trigger app actions.

### Commands

- `ui`
- `settings`
- `open-settings`
- `show-settings`
- `reload-config`
- `reload-configs`
- `add-package?repo=<owner/name>` — installs a package from the official Browse
  catalog, opens Settings → Browse, and reloads packages. Existing installations
  keep their selected version. Links received during startup wait for the initial
  package load. Other repositories must be added through Settings → Chords.

### Examples

```sh
# Open the Chord UI
open --background 'chord:ui'

# Open the settings window
open --background 'chord:settings'

# Open the settings window (host-style form)
open --background 'chord://settings'

# Reload chord configs
open --background 'chord:reload-config'

# Install an official package from the website or a launcher
open 'chord://add-package?repo=KeyChord%2Fchords-vscode'
```

## CLI

The Chord app's executable doubles as a `chord` CLI. To put it on your `PATH`, open
**Settings → General → Command Line** and click **Add chord to PATH**, which links
`/usr/local/bin/chord` to the app (macOS may ask for your password). Development builds use the
name `chordd`. Run `chord --help` for the full list.

Chord sequences execute in the running app (launching it in the background when needed) so they
use its loaded packages and frontmost-app context; `bun` uses Chord's embedded runtime, and `exec`
runs the command as a Chord child process.

### Commands

- `ui`, `settings` (aliases: `open-settings`, `show-settings`)
- `reload-configs` (alias: `reload-config`)
- `bun <script> [args...]` (alias: `js`)
- `run <file> <export> [args...]`
- `exec <shell-command>`
- `chord <sequence>`
- `<sequence>` (shorthand for `chord <sequence>`)

### Examples

```sh
chord bun ./script.ts hello
chord js ./script.ts hello
chord run ./script.ts main hello
chord exec 'open https://example.com'
chord chord fq
chord fq
chord ui
chord settings
chord reload-configs
```

In a Chord checkout, `apps/chord-tauri/chord` is a shell wrapper for the same commands. It looks for
a build in `apps/chord-tauri/src-tauri/target`, then in the standard `/Applications` locations; set
`CHORD_EXECUTABLE` when the binary lives somewhere else. Its `ui`, `settings`, and `reload-configs`
commands go through the `chord:` URL scheme, so the app bundle must have been launched at least
once.

# Native code

Every chord handler is a JavaScript module, and Chord runs it on an embedded [Bun](https://bun.sh) runtime via the [`rbun`](https://github.com/KeyChord/rbun) crate. Native Swift is exposed as a Node-API add-on built with [NodeSwift](https://github.com/kabiroberai/node-swift):

`Sources/KeychordChordsMenuNativeMenu/menu.swift` → `target/<triple>/menu/menu.node`

`@keychord/config` builds the dynamic library products in the package's root `Package.swift` and resolves normal `.node` imports in handlers. Its plugin packages each addon under `js/native/<triple>/<name>/`, with NodeSwift's dynamic `libNodeAPI.dylib` beside it. Keeping that runtime dynamic lets Chord safely load add-ons from several packages in one process; the package does not need an npm dependency on NodeSwift at runtime.

> **Trust:** native code runs inside Chord with your full user permissions. Only enable packages from sources you trust.

## Package layout

```text
my-package/
├── chords/macos.toml
├── src/
│   ├── js/menu.ts                 # imports ./menu.node
│   └── types/native.d.ts          # declares the addon's TypeScript API
├── Package.swift                 # Swift products and dependencies
├── Sources/KeychordChordsMenuNativeMenu/
│   ├── menu.swift                 # entry point: exports #NodeModule
│   └── helpers.swift
├── js/
│   ├── menu.js                    # generated JavaScript bundle
│   └── native/aarch64-apple-darwin/menu/
│       ├── menu-<hash>.node       # packaged addon
│       └── libNodeAPI.dylib
└── target/                        # platform-specific output — commit/publish it like js/
    └── aarch64-apple-darwin/
        └── menu/
            ├── menu.node          # Node-API add-on
            └── libNodeAPI.dylib   # shared NodeSwift runtime, found through @loader_path
```

`src/` and `Sources/` are authored source, `chords/` is Chord configuration, and `js/` holds the generated bundles and their native assets. `target/<triple>/` holds native build products. Native build state lives in `node_modules/.cache/chord/native/`, covered by the usual `node_modules/` ignore. Keep distributable `js/` and `target/` files tracked.

## Writing one

Import `NodeAPI` and declare the add-on exports with `#NodeModule`. NodeSwift converts supported Swift values and thrown errors at the Node-API boundary, so no C ABI or manual pointer ownership is needed:

```swift
// Sources/KeychordExampleNativeBeep/beep.swift
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

The handler imports the addon normally. `config()` supplies the native-addon plugin automatically, so the handler needs no build alias or runtime path resolver:

```ts
// src/js/beep.ts
import addon from "./beep.node";

export default function build(times = 1) {
  return function beep() {
    addon.beep(times);
  };
}
```

Declare the imported API in `src/types/native.d.ts`:

```ts
declare module "*/beep.node" {
  const addon: { beep(times: number): void };
  export default addon;
}
```

- The plugin packages each configured target triple and selects the matching platform and architecture at runtime. Static imports load the addon with the handler module; a `require("./beep.node")` inside a function keeps loading lazy. Generated native asset URLs are relative to the emitted JavaScript, including after vendoring.
- Existing handlers may still use `import.meta.chord.resolveNative("beep")`, which resolves `target/<Chord's triple>/beep/beep.node`. `import.meta.chord.resolvePackageFile("any/relative/path")` resolves arbitrary package files. Both remain bound to the calling module's logical package.
- Each installed package runs in its own supervised process with a persistent embedded Bun VM. Imports, handler factories, lifecycle callbacks, timers, and native add-ons all stay in that process. On macOS the worker services AppKit events and Swift's `@MainActor`; keep NodeAPI values on `@NodeActor` and pass Swift values to the main actor. Return or await native Promises so Chord waits for completion and reports errors.
- JavaScript and Swift exceptions are reported as handler errors. A native abort, bad pointer, or `process.exit()` terminates only the package worker. Its supervisor automatically starts a fresh VM and restores the registered handler factories in their original order. Existing handler IDs remain usable, other packages keep running, and the interrupted action is never replayed. Repeated crashes or startup failures use exponential backoff from 250 ms to 30 seconds, resetting after a healthy minute. Setup and health checks have a 15-second deadline; actions and lifecycle callbacks have a five-minute deadline for interactive UI.
- Recovery resets the package's JavaScript globals, sequence return values, timers, and native state. Factories run again to rebuild handlers and lifecycle subscriptions; keep factories suitable for reinitialization. Unloading/reloading configuration and quitting stop the supervisor and prevent it from respawning workers.
- Reloading configuration replaces package workers, so rebuilt JavaScript, transitive imports, and native add-ons are loaded into fresh VMs. Return-value assignments preserve JavaScript object identity inside the package's VM between events in the same matching interval.
- Workers retain the user's filesystem and process access; this is crash isolation, not a permissions sandbox.
- Package stdout and stderr inherit Chord's log streams.

## Declaring it

Nothing special: it is a JavaScript handler.

```toml
[on.beep]
file = "beep.js"
args = [2]

[chords."-b"]
"@input:beep" = []
```

The old `kind = "native"` declaration is rejected with a pointer to this section.

## Using native handlers from other packages

Vendor the dependency with `config({ vendor: ["@keychord/chords-menu"] })` and import its JavaScript normally. Its copied `js/` tree includes the packaged native addons and runtime libraries, so relative native asset URLs continue to work. Vendoring also preserves the `target/` tree for existing handlers using runtime path resolvers. Add-ons are consumed through their JavaScript API, not by importing their generated Swift modules.

For compatibility with installed packages, the `chord` module still exports `resolveNativeModulePath(import.meta, relpath)` and `resolvePackageFile(import.meta, path)`. These delegate to the same per-module resolvers. New native handlers built with `@keychord/config` can use `.node` imports directly.

## Building

Packages built with `@keychord/config` compile the package's `Package.swift` automatically as part of `vp pack` (and rebuild on change in `vp pack --watch`). To build every distributable folder at once—`js/` and `target/<triple>/`—run `vpr compile` (`--triple <triple>` to cross-build, `--skip-js`/`--skip-native` for one half, `vpr -r compile` for the whole workspace). Building needs a Swift toolchain from Xcode or the Command Line Tools. Build options go in `vite.config.ts`; Swift frameworks, compiler flags, and linker settings belong in `Package.swift`:

```ts
import { config } from "@keychord/config";

export default config({
  native: {
    triples: ["aarch64-apple-darwin", "x86_64-apple-darwin"], // default: host triple
  },
});
```

`@keychord/config` builds the package's root `Package.swift` directly. SwiftPM build products and staging files live in each package's `node_modules/.cache/chord/native/`, grouped by target triple and add-on; distributable files are copied to `target/`. `@keychord/config` currently supports Apple triples.

Any other build system works too: Chord imports the package's JavaScript from disk, and its loader must resolve a Node-API addon compatible with Bun plus any dynamic dependencies. For the standard NodeSwift build, `libNodeAPI.dylib` sits beside the addon and is found through `@loader_path`.

## Testing without the app

A Chord build's CLI runs a script on the same embedded Bun, with the `chord` module available. Functions that need the running app (see [the built-in `chord` module](#the-built-in-chord-module)) throw in this mode. `chords-menu` has such a script, which imports the handler and calls it once:

```sh
chord bun scripts/run.ts BY_LETTERS f     # from the chords-menu package folder
```

Package tests run under `bun test` with the `chord` module mocked by [`@keychord/chordtest`](https://github.com/KeyChord/chords/tree/main/@keychord/chordtest).

## Troubleshooting

- Native loading failures name the missing file or Node-API registration problem; run `vp pack` and check that the packaged addon and `libNodeAPI.dylib` under `js/native/<your triple>/` are included and that the `#NodeModule` export names match the TypeScript binding.
- Node-API add-ons run in Chord's embedded Bun runtime; check that the add-on targets Bun's supported Node-API surface.
- Accessibility calls inherit Chord's Accessibility permission—grant it to Chord.
- A native crash reports failure for the interrupted action; the worker restarts automatically for later actions. Run the handler from the CLI while developing to inspect its diagnostics.

## FAQ

### Why Bun rather than a lighter runtime?

Bun brings JavaScriptCore's JIT and Node/Bun APIs, including the Node-API loader used by NodeSwift add-ons. Bun has no official embedding API, so Chord embeds a lightly patched Bun runtime through the [`rbun`](https://github.com/KeyChord/rbun) crate and exposes the custom `chord` module to scripts.

### Compact help ranges

A help hint can end in a quoted letter or digit range:

```toml
[chords]
'?"1-9"' = { name = "Number" }
'?m"0-9"' = { name = "Menu item number" }
'?-"a-z"' = { name = "Menu item by letter" }
```

The first hint shows one physical keycap containing `1-9`. After pressing `m`,
the second shows `0-9`; after `-`, the third shows `A-Z`. Each row replaces
individual options in that range and highlights when any covered key is selected. Ranges are inclusive,
ascending, and use lowercase letters (`a-z`) or digits (`0-9`). The prefix
before the range is a normal key sequence. Hints describe options; they do not
create executable chord bindings. Parenthesized hints such as `?-(a-z)` remain
supported and use the same compact keycap display.

The core `@keychord/chords-chord` package defines repeat-count help with
`'?"1-9"' = { name = "Repeat count" }`. The panel extends this range to `0-9`
after a numeric prefix. At the start of a sequence, `0` (Chord settings) and
`1` (Loop chord) also keep their dedicated rows.
