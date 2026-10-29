# Chord

**Multi-letter shortcuts for macOS.**

**Chord** lets you bind actions to key sequences while _keeping your existing shortcuts untouched_.

## Installation

1. Download `Chord-aarch64.dmg` from the [latest release](https://github.com/KeyChord/Chord/releases/latest). It is signed and notarized.
2. Open the DMG, drag **Chord** into `Applications`, and launch it.

# Setup

## Permissions

Chord requires macOS _Accessibility Permissions_ so it can detect when you press <kbd>⇪</kbd> (Caps Lock) + <kbd>␣</kbd> (Space) to activate _Chord Mode_.

## Chord Mode

Once you enter _Chord Mode_, it stays active as long as <kbd>␣</kbd> is held down.

_Only <kbd>␣</kbd> needs to stay pressed; you should release <kbd>⇪</kbd> after pressing <kbd>␣</kbd> 

In _Chord Mode_, any keys you type will be intercepted by Chord as input. These key events won't be passed to the focused application.

## Triggering a Chord

> Chords can only be triggered in _Chord Mode_, so make sure to keep <kbd>␣</kbd> held down!

To trigger a chord, type out the chord's letters and then press <kbd>⇪</kbd>.

For example, typing <kbd>0</kbd><kbd>,</kbd><kbd>⇪</kbd> opens Chord's settings window.

Once you're done, you can release <kbd>␣</kbd> to exit _Chord Mode_.

### Auto-trigger

For sending one-off chords, you can release <kbd>␣</kbd> right after typing out the chord to execute it.

For example, instead of pressing <kbd>⇪</kbd>, you can also type <kbd>0</kbd><kbd>,</kbd> and then release  <kbd>␣</kbd> to bring up Chord's settings.

<details>
    <summary>Are you a <code>vim</code> user?</summary>
    For <code>vim</code> users, it might help to treat <em>Chord Mode</em> as somewhat analagous to <code>vim</code>'s <em>Normal Mode</em>, with the main difference that it's <a href="https://stackoverflow.com/questions/11828270/how-do-i-exit-vim">a lot easier to exit</a>: just lift up <kbd>␣</kbd>!
</details>

# Configuration

## Chord Files

All chords are defined in configuration files called _chord files._

A _chord file_ contains chords for a single application and platform, which is determined by its path.

### _Chord File_ Example - Finder

A _chord file_ for macOS Finder would live at:\
`chords/com/apple/Finder/macos.toml`

<details>
    <summary>Why do chord files end in <code>.toml</code>?</summary>
    Chord files are written with a standardized configuration format called <a href="https://toml.io/en/v1.1.0">TOML</a>. 

    You likely don't need to spend time learning it, as TOML is quite simple and intuitive.
</details>

## Chord Packages

All _chord files_ live under a top-level `chords/` folder, which itself is part of a _chord package_ that contains both _chord files_ and scripts the chords might use.

Your default _chord package_ is located in `~/.chord/packages/default`.

By adding _chord files_ to this package, you can create your own chords!

### _Chord Package_ Example - Finder 

If you create the following file in your _chord package:_

`chords/com/apple/Finder/macos.toml`

And paste in the following:

```toml
name = "Finder"

[chords]
# Always use single quotes when defining a chord's key sequence
'gh' = {
  # The `name` property is a required label for the chord
  name = 'Goto: Home', 
  
  # The `shortcut` property tells Chord to simulate a shortcut when this chord is triggered
  shortcut = 'cmd+shift+h' 
} # end of `gh` properties

# A chord that starts with a non-letter is a global chord and can be triggered from any application
'/f' = { 
  name = 'Finder', 

  # The `shell` property tells Chord to run a shell command (in the background) when this chord is triggered
  shell = 'open -a Finder'
}
```

> Note that anything after a `#` in _chord files_ is a comment used purely for documentation and is ignored by the configuration file.

You'll then be able to type:
1. <kbd>/</kbd><kbd>f</kbd><kbd>⇪</kbd> to open Finder,
2. <kbd>g</kbd><kbd>h</kbd><kbd>⇪</kbd> to open your home directory!

## Pre-Made Chord Packages

Instead of defining your own chords for every app, Chord provides pre-made chord packages you can use!

Chord's compiled chords contain certain conventions:

- Navigation: <kbd>n</kbd><kbd>e</kbd><kbd>s</kbd><kbd>w</kbd> (north, east, south, west)
- Movement: <kbd>l</kbd><kbd>r</kbd><kbd>u</kbd><kbd>d</kbd> (left, right, up, down)
- Position & Order: <kbd>f</kbd><kbd>k</kbd><kbd>a</kbd><kbd>z</kbd> (front, back, first, last)

> For a full list of these conventions, see the [KeyChords/chords](https://github.com/KeyChord/chords) repo!

### Installing Chord Packages

TODO

## Chord Scripts

In addition to simulating shortcuts and executing shell commands, chords can also run scripts!

This is useful for apps that have configurable shortcuts, in which case, you can't just simulate the default ones.

### Chord Scripts Example - IntelliJ IDEA

For example, here's what a _chord file_ for IntelliJ might look like:

```toml
# chords/com/jetbrains/intellij/macos.toml
name = "IntelliJ IDEA"

[on.action]
file = "jetbrains.js"

[chords]
'fh' = { name = "File: Here", '@input:action' = ['NewFile'] }
```

Where `jetbrains.js` refers to this file located at `js/jetbrains.js`:

```javascript
// `mkdtempDisposable` makes a temporary directory that's automatically cleaned up
import { mkdtempDisposable } from "node:fs/promises";

/**
 * Every script should have a top-level "builder" function as its
 * default export. The builder returns the event handler function.
 */
export default function actionBuilder() {
  // Any code here runs only once: when the chord file is first loaded
  const ideaPath = "/Applications/IntelliJ IDEA.app/Contents/MacOS/idea"
  
  /**
   * The event handler function gets called every time the chord is triggered.
   * The arguments come from the `array` assigned to the '@<event>:<handler>' property
   * specified in the TOML chord file.
   * 
   * For example, if the chord file had:
   * 
   * 'fh' = { 
   *    # ...
   *    '@input:action' = ['NewFile']
   * }
   *
   * Then Chord would call `action("NewFile")`.
   */
  return async function action(actionId) {
    // We create a temporary directory to avoid conflicting with future/past calls
    await using dir = await mkdtempDisposable(`/tmp/jetbrains-`) 
    
    // The following Groovy script runs the action `actionId` in IntelliJ
    const scriptPath = `${dir.path}/action.groovy`;
    await Bun.write(scriptPath, `
      import com.intellij.openapi.actionSystem.ActionManager
      IDE.application.invokeAndWait {
        def m = ActionManager.instance
        def a = m.getAction("${actionId}")
        if (a) m.tryToExecute(a, null, null, null, true)
      }
    `);
  
    // We use `Bun.$` to run the above code with the IntelliJ CLI
    await Bun.$`${ideaPath} ideScript ${scriptPath}`.quiet();
  };
}
```

Using this script, we can define chords for IntelliJ-based IDEs that work regardless of what the user's existing IDE shortcuts are!
