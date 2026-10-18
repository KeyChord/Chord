# Chord

**Multi-letter shortcuts for macOS.**

_Chord_ lets you bind actions to key sequences while **keeping your existing shortcuts untouched**.

> 🚧 Under Construction! Docs may be outdated and/or incomplete...

## Installation

> [!NOTE]
> This section is largely AI-written, so parts of it may not be fully accurate. It will be replaced with a hand-written version when I get the time.

_Chord_ runs on macOS. Builds are currently published for Apple Silicon Macs only.

1. Download `Chord-aarch64.dmg` from the [latest release](https://github.com/KeyChord/Chord/releases/latest). It is signed and notarized.
2. Open the DMG, drag _Chord_ into `Applications`, and launch it.
3. When _Chord_ asks, grant it Accessibility access (System Settings → Privacy & Security → Accessibility). It's the only permission _Chord_ needs.

To try it, hold `Caps Lock`, press `Space`, and keep holding `Space`: the chords panel lists the chords for the focused app. Type `0,` and release `Space` to open _Chord_'s settings. While the panel is open, `Esc` clears what you've typed and `Shift+Tab` hides or shows the panel.

_Chord_ comes with a few chord packages preinstalled, including `chords-menu` (menu bar navigation) and `chords-com.apple` (Apple's built-in apps). To add more:

- **Settings → Chords → Chord Monorepos → Add Monorepo** with `KeyChord/chords`, then tick the packages you want under **Packages to use**. This is the recommended way to get the official packages from [KeyChord/chords](https://github.com/KeyChord/chords) (Finder, browsers, editors, terminals, windows and Spaces, and more) at their latest versions. The preinstalled `chords-menu` and `chords-com.apple` keep loading from their own repos until you remove those under **Chord Repos**.
- **Settings → Browse** lists a few official packages (Chromium browsers, VS Code, JetBrains IDEs, terminals, AeroSpace) that you can add one at a time.
- **Settings → Chords → Chord Repos → Add Repo** installs a single chord package from its own GitHub repo (`owner/name` or a URL).
- **Settings → Chords → Local Folders** loads a package from a folder on your Mac, for chords you write yourself.

**Settings → General → Command Line** can add a `chord` command to `/usr/local/bin` for running chords and scripts from a terminal (see [the CLI docs](./scripting.md#cli)).

_Chord_ doesn't update itself: to update, download the latest DMG again and replace the app. Quitting _Chord_ (`0q`) gives the `Caps Lock` key back to macOS.

To write your own chords, follow the quick start in [create-chord-package](https://github.com/KeyChord/create-chord-package), then see [scripting.md](./scripting.md). To build _Chord_ itself, see [development.md](./development.md).

## How does it work?

Chords are sequences of non-modifier keys that correspond to actions. Most of the time, these actions are defined as existing keyboard shortcuts in an application.

For example, here are some example chords for the macOS Finder app:

```toml
# chords/com/apple/finder/macos.toml
name = "Finder"

[chords]
gd = { name = "Goto Downloads", shortcut = "opt+cmd+l" }
du = { name = "Directory Up", shortcut = "cmd+up" }
tt = { name = "Toggle Tabs", shortcut = "cmd+shift+t" }
ts = { name = "Toggle Sidebar", shortcut = "ctrl+cmd+s" }
tp = { name = "Toggle Preview", shortcut = "cmd+shift+p" }
nds = { name = "New Directory with Selection", shortcut = "ctrl+cmd+n" }
```

Because chords are only composed of letters and/or numbers, they are often easier to remember than their modifier + key counterparts.

In order to run a chord, you need to have the _Chord_ app installed and running in the background. _Chord_ won't do anything until you press the activation sequence to activate _Chord Mode_, which is `Caps Lock` + `Space`.

<details>
  <summary>Why Caps Lock + Space?</summary>

An ideal requirement for typing chords is to have all your fingers free to type arbitrary chord sequences while a certain key is held down. One of the only keys that fit this requirement is the `Space` key.

However, `Space` needs to be pressed as part of a key combination, since pressing it alone will output the actual space ` ` character. The key which makes the most sense as part of this combination is `Caps Lock`, since it's one of the easiest keys to reach yet still remains relatively unused on most layouts.

> [!NOTE]
> This section is largely AI-written, so parts of it may not be fully accurate. It will be replaced with a hand-written version when I get the time.

Because we only use it as part of a key combination, pressing `Caps Lock` on its own will still toggle on Caps as usual, and this special behavior only applies when `Space` is pressed down while `Caps Lock` is pressed. While _Chord Mode_ is active, Caps Lock is kept off: activating it turns Caps Lock off, and pressing `Caps Lock` to run a chord never turns it on. When _Chord Mode_ ends, Caps Lock returns to whatever it was before you activated it.

To make this reliable, _Chord_ takes over the `Caps Lock` key while it runs and decides every change to Caps Lock itself. A tap turns Caps Lock off as soon as you press it, but turns it on only when you release it, because until then the press could still become `Caps Lock` + `Space`. While _Chord_ runs, features that rely on macOS handling the key itself don't work:

- "Use the Caps Lock key to switch to and from ABC" in Keyboard settings.
- The Caps Lock key as the VoiceOver modifier. Set the VoiceOver modifier to `Control` + `Option` instead; _Chord_'s settings remind you when this applies.
- Apps that read `Caps Lock` as an ordinary key, such as games or virtual machines that capture the keyboard.

When _Chord_ quits, the key goes back to macOS. If _Chord_ crashes, a small background helper puts it back.

</details>

After pressing the `Space` key, the chords panel will appear on the left, which displays the valid chords for the focused application.

In addition to app-specific chords, you're also able to define global chords (which can be triggered from any app) by starting the key sequence with a symbol instead of a letter:

```toml
# chords/macos.toml
[chords]
"/q" = { name = "Force Quit", shortcut = "cmd+opt+esc" }
```

After typing out all the chord letters, you can simply release `Space` to trigger it and exit _Chord Mode_.

If you want to trigger a chord without exiting _Chord Mode_, you can press `Caps Lock` which will trigger and clear your input. You'll remain in _Chord Mode_ as long as `Space` is still held down.

<details>
  <summary>Why `Caps Lock`?</summary>

`Caps Lock` is a key that's significantly more comfortable to press after many chord sequences, especially ones containing symbols. To try it yourself, compare pressing `[o` followed by `Caps Lock`, and then compare it to pressing `Enter` afterwards.

</details>

You can run a chord multiple times by pressing `Caps Lock` again. Pressing the following sequence of keys in _Chord Mode_ goes up three folders in Finder:

<details>
  <summary>du⇪⇪⇪</summary>

1. Tap(D)
2. Tap(U)
3. Tap(Caps)
4. Tap(Caps)
5. Tap(Caps)
</details>

If you're triggering a sequence of chords that share the same prefix, you can hold `Shift` while pressing the last key of the chord:

<details>
  <summary>/F</summary>

1. Tap(Slash)
2. Press(Shift)
3. Tap(F)
4. Release(Shift)
</details>

This will execute the chord with that sequence (lowercased, since all chords must be lowercase) _without_ adding appending the key to your existing sequence.

As an example, say you wanted to quickly toggle the tabs view, the sidebar view, and the preview in Finder. Instead of typing out the entirety of three separate chords:

<details>
  <summary>tt⇪ts⇪tp⇪</summary>

1. Tap(T)
2. Tap(T)
3. Tap(Caps)
4. Tap(T)
5. Tap(S)
6. Tap(Caps)
7. Tap(T)
8. Tap(P)
9. Tap(Caps)
</details>

You can just type `T` once and keep `Shift` held down for the other three keys:

<details>
  <summary>tTSP</summary>

1. Tap(T)
2. Press(Shift)
3. Tap(T)
4. Tap(S)
5. Tap(P)
6. Release(Shift)
</details>

In addition, because chords don't use modifier keys, you're able to use any existing shortcuts while _Chord Mode_ is active. The following sequence of keys will move all the contents of your Downloads folder into a new folder:

<details>
  <summary>/FgD⌘ands⇪</summary>

1. Tap(Slash)
2. Press(Shift)
3. Tap(F)
4. Release(Shift)
5. Tap(G)
6. Press(Shift)
7. Tap(D)
8. Release(Shift)
9. Press(Command)
10. Tap(A)
11. Release(Command)
12. Tap(N)
13. Tap(D)
14. Tap(S)
15. Tap(CapsLock)
</details>

Pressing `Backspace` will delete the last key in your current sequence. Pressing `Shift+Backspace` will clear the entire sequence.

## Actions

Actions can also take the form of shell commands, which is useful when certain functionality isn't available via a keyboard shortcut:

```toml
[chords]
"/f" = { name = "Finder", shell = "open -a Finder" }
```

> **Chords** ignores all inputs whenever a modifier key (other than Shift) is held down.

To exit _Chord Mode_, all you need to do is simply release your `Space` key. It's that simple!

## JavaScript Scripting

In addition to running shortcuts and shell commands, chords can also run arbitrary JavaScript scripts, which provides more power for certain use-cases, especially for apps that don't necessarily have shortcuts bound to every action.

```toml
# chords/com/microsoft/VSCode/macos.toml
name = "VSCode"

[on.command]
file = "vscode.js"
args = ["/Applications/Visual Studio Code.app"]

# A handler from another package, vendored with `config({ vendor: ["@keychord/chords-menu"] })`
[on.menu]
file = "@keychord/chords-menu/js/menu.js"

[chords]
fh = { name = "File: Here", '@input:command' = [ "explorer.newFile" ] }
mt = { name = "Menu: Terminal", '@input:menu' = [
	"BY_PATH",
	["View", "Terminal"]
] }
```

> [!NOTE]
> This section is largely AI-written, so parts of it may not be fully accurate. It will be replaced with a hand-written version when I get the time.

The top-level `on` table maps event names to JavaScript filenames. Chords reference those handlers with `@input:<event>`, or use `@init`, `@keydown`, `@keyup`, and `@keypress` for live buffer events. Any specifier can retain the handler's return value with `=<identifier>`; later handlers can receive it through `$identifier`. See [the event contract](./scripting.md#chord-events-and-persistent-return-values) for examples and `this.event` keyboard data.

Chord embeds [Bun](https://bun.com) as a runtime using [rbun](https://github.com/KeyChord/rbun).

## Global Hotkeys

> [!NOTE]
> This section is largely AI-written, so parts of it may not be fully accurate. It will be replaced with a hand-written version when I get the time.

Many macOS apps can only be activated through a global hotkey. Chord packages can ask _Chord_ for one with `registerGlobalHotkey(bundleId, hotkeyId)` from the `chord` module. _Chord_ assigns the first unused shortcut from a pool of multi-modifier combinations (such as `cmd+ctrl+alt+shift+a`, across letters, digits, and `F1`–`F12`, minus combinations macOS already uses), remembers the assignment, and returns the shortcut so the package can bind it in the app and press it from a chord. **Settings → Global Shortcuts** lists every assignment and lets you change it.

## Regex

Chord triggers can also come in the form of regexes. To define a regex trigger, you just need to use parentheses:

```toml
# @keychord/chords-menu: chords/macos.toml
'=/(.+)' = { name = 'Menu Bar: Search $1', '@input:menu' = ['SEARCH', '$1'] }
```

> [!NOTE]
> This section is largely AI-written, so parts of it may not be fully accurate. It will be replaced with a hand-written version when I get the time.

Only the parenthesized parts are regex syntax; the rest of the trigger is matched literally, and the pattern must match the whole input. Captures are available as `$1`, `$2`, … in the chord's `name` and in handler arguments. `shell` commands run exactly as written, without capture substitution, so use a JavaScript handler when an action needs the captured text.

## Comparison to keyboard shortcuts

Because keyboard shortcuts must be composed of one or more modifier keys followed by a single character, they come with inherent limitations:

### Limited key combinations

Because there are only a limited amount of modifier+key combinations available, many shortcuts don't intuitively map to their action:

```toml
# chords/com/microsoft/VSCode/macos.toml
[chords]
gf = {
	name = "Go to File",
	# cmd+p doesn't make you think of "File" (my best guess is that cmd+f is already taken by Find, and so it's adapted from the shortcut for the similar feature Command Palette which is cmd+shift+p (p for palette)
	# Either way, "gf" for "goto file" is a lot easier to remember
	shortcut = "cmd+p"
}
gd = {
	name = "Go to Definition",
	# Some shortcuts don't even use letters at all...
	shortcut = "F12"
}
```

### Differences between apps

Different apps will often have different keybindings for similar actions. While you are able to set custom keymaps in certain apps, they make it harder to follow along with documentation (which often assume the default keymap) and require you to create and maintain your own keybindings files if you ever want to make changes.

With chords, you can define the same chord across multiple apps which map to the corresponding shortcut for that app. This way, you can just remember one chord for an action and it'll work across all your apps without you having to memorize the specific shortcuts for each app:

```toml
# chords/com/microsoft/VSCode/macos.toml
[chords]
gd = { name = "Go to definition", shortcut = "f12" }
rs = { name = "Rename Symbol", shortcut = "f2" }
rf = { name = "Recent Files", shortcut = "cmd+e" }
cp = { name = "Command Palette", shortcut = "cmd+shift+p" }
fc = { name = "Format Code", shortcut = "shift+alt+f" }
```

```toml
# chords/com/jetbrains/intellij/macos.toml
[chords]
gd = { name = "Go to definition", shortcut = "cmd+b" }
rs = { name = "Rename Symbol", shortcut = "shift+f6" }
rf = { name = "Recent Files", shortcut = "ctrl+tab" }
cp = { name = "Command Palette", shortcut = "cmd+shift+a" }
fc = { name = "Format Code", shortcut = "cmd+option+l" }
```

### Multi-modifier combinations are difficult to press and remember

```toml
# chords/com/apple/finder/macos.toml
[chords]
ss = { name = "Sort by Size", shortcut = "ctrl+opt+cmd+6" }
```
