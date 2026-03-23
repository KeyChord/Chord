# Chord

**Multi-letter shortcuts for macOS.**

**Chord** lets you bind actions to key sequences while _keeping your existing shortcuts untouched_.

## How does it work?

Chords are sequences of non-modifier keys that correspond to actions. Most of the time, these actions are defined to simulate existing keyboard shortcuts within an application.

For example, here are some example chords for the macOS Finder app:
```toml
# chords/com/apple/finder/macos.toml
gd = { name = "Goto Downloads", shortcut = "opt+cmd+l" }
du = { name = "Directory Up", shortcut = "cmd+up" }
tt = { name = "Toggle Tabs", shortcut = "cmd+shift+t" }
ts = { name = "Toggle Sidebar", shortcut = "ctrl+cmd+s" }
tp = { name = "Toggle Preview", shortcut = "cmd+shift+p" }
nds = { name = "New Directory with Selection", shortcut = "ctrl+cmd+n" }
```

Because chords are only composed of multiple letters, they are often easier to remember than their modifier + key counterparts.

In order to run a chord, you need to have the **Chord** app installed and running in the background. **Chord** won't do anything until you press the activation sequence to activate _Chord Mode_, which defaults to `Caps Lock` + `Space`.

<details>
  <summary>Why Caps Lock + Space?</summary>

  An ideal requirement for typing chords is to have all your fingers free to type arbitrary chord sequences while a certain key is held down. One of the only keys that fit this requirement is the `Space` key.

  However, `Space` needs to be pressed as part of a key combination, since pressing it alone will output the actual space ` ` character. The key which makes the most sense as part of this combination is `Caps Lock`, since it's one of the easiest keys to reach yet still remains relatively unused on most layouts.

  Because we only use it as part of a key combination, pressing `Caps Lock` on its own will still toggle on Caps as usual, and this special behavior only applies when `Space` is pressed down while `Caps Lock` is pressed.
</details>

After pressing the `Space` key, the chords panel will appear on the left, which displays the valid chords for the focused application.

In addition to app-specific chords, you're also able to define global chords (which can be triggered from any app) by starting the key sequence with a symbol instead of a letter:

```toml
# chords/macos.toml
"/q" = { name = "Force Quit", command = "Force Quit", shortcut = "cmd+opt+esc" }
```

After typing out all the chord letters, you can simply release `Space` to trigger it and exit _Chord Mode_.

If you want to trigger a chord without exiting _Chord Mode_, you can press `Caps Lock` which will trigger and clear your input. You'll remain in _Chord Mode_ as long as `Space` is still held down.

<details>
  <summary>Why `Caps Lock`?</summary>

  `Caps Lock` is a key that's significantly more comfortable to press after many chord sequences, especially ones containing symbols. To try it yourself, compare pressing `[o` followed by `Caps Lock`, and then compare it to pressing `Enter` afterwards.
 <details>

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
  1. Tap(T)
  1. Tap(Caps)
  1. Tap(T)
  1. Tap(S)
  1. Tap(Caps)
  1. Tap(T)
  1. Tap(P)
  1. Tap(Caps)
</details>

You can just type `T` once and keep `Shift` held down for the other three keys:

<details>
  <summary>tTSP</summary>

  1. Tap(T)
  1. Press(Shift)
  1. Tap(T)
  1. Tap(S)
  1. Tap(P)
  1. Release(Shift)
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
  11. Tap(N)
  11. Tap(D)
  11. Tap(S)
  12. Tap(CapsLock)
</details>

## Actions

Actions can also take the form of shell commands, which is useful when certain functionality isn't available via a keyboard shortcut:
```toml
"/f" = { name = "Finder", command = "Finder", shell = "open -a Finder" }
```




> **Chords** ignores all inputs whenever a modifier key (other than Shift) is held down.

To exit _Chord Mode_, all you need to do is simply release your `Space` key. It's that simple!

<!-- TODO: This section should be introduced alongside `shell` -->
## JavaScript Scripting

In addition to running shortcuts and shell commands, chords can also run arbitrary JavaScript scripts, which provides more power for certain use-cases, especially for apps that don't necessarily have shortcuts bound to every action.

```toml
# chords/com/microsoft/VSCode/macos.toml
[config.js]
module = '''
export default (commandId: string) => {
  // ...
}

export const menu = (...segments: string[]) => {
  // ...
}
'''

[chords]
# `explorer.newFile` doesn't have a default shortcut in VSCode
fh = { name = "File: Here", args = ["explorer.newFile"] }
# `args:menu` calls the named `menu` export instead of `default`
mc = { name = "Menu: Columns", 'args:menu' = ["View", "Columns"] }
# String args are evaluated as JavaScript and must return an array
df = { name = "Dynamic File", args = '["explorer.newFile", Date.now().toString()]' }
# ...
```

`args` and `args:*` accept either a TOML array of literal values or a raw JavaScript string. When you use the string form, Chords evaluates it in the embedded JS runtime and expects the result to be an array, which is then spread into the target function call.

Chords embeds the QuickJS JavaScript environment (excluding its standard library) as well as certain LLRT modules (which are based on the Node APIs). Module resolution is currently only implemented for root imports (e.g. if you have a `src/file.js` at the root of your repo, you have to write `import file from "src/file.js"`).

## Global Hotkeys

Many macOS apps can only be activated through a global hotkey. We thus use a synthetic hotkey pool:
- `cmd+ctrl+alt+shift+{a-z}`
- `cmd+ctrl+alt+shift+{0..9}`
- `cmd+ctrl+alt+shift+f{1..12}`
- `cmd+ctrl+alt+f{1..12}`

## Comparison to keyboard shortcuts

Because keyboard shortcuts must be composed of one or more modifier keys followed by a letter/number/symbol, they come with inherent limitations:

### Limited key combinations

Because you can only choose one of 26 letters for your shortcut, many shortcuts end up with letters that don't intuitively map to their action:

```toml
# chords/com/microsoft/VSCode/macos.toml
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


### Differences between platforms

The same app on different platforms (Windows/Linux/MacOS) often use different shortcuts for the same action (including different modifier keys), which can be a pain to deal with if you need to switch between platforms.

Chords can act as an abstraction over these shortcut differences by letting you map the same chord to different shortcuts on each platform.

<!-- TODO: give example -->

### Differences between apps

Different apps will often have different keybindings for similar actions. While you are able to set custom keymaps in certain apps, they make it harder to follow along with documentation (which often assume the default keymap) and require you to create and maintain your own keybindings files if you ever want to make changes.

With chords, you can define the same chord across multiple apps which map to the corresponding shortcut for that app. This way, you can just remember one chord for an action and it'll work across all your apps without you having to memorize the specific shortcuts for each app:

```toml
# chords/com/microsoft/VSCode/macos.toml
gd = { name = "Go to definition", shortcut = "f12" }
rs = { name = "Rename Symbol", shortcut = "f2" }
rf = { name = "Recent Files", shortcut = "cmd+e" }
cp = { name = "Command Palette", shortcut = "cmd+shift+p" }
fc = { name = "Format Code", shortcut = "shift+alt+f" }
```

```toml
# chords/com/jetbrains/intellij/macos.toml
gd = { name = "Go to definition", shortcut = "cmd+b" }
rs = { name = "Rename Symbol", shortcut = "shift+f6" }
rf = { name = "Recent Files", shortcut = "ctrl+tab" }
cp = { name = "Command Palette", shortcut = "cmd+shift+a" }
fc = { name = "Format Code", shortcut = "cmd+option+l" }
```

### Multi-modifier combinations are difficult to press and remember
```toml
ss = { name = "Sort by Size", shortcut = "cmd+opt+cmd+6" }
```
