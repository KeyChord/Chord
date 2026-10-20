# Chord

**Multi-letter shortcuts for macOS.**

_Chord_ lets you bind actions to key sequences while **keeping your existing shortcuts untouched**.

## Setup

> _Chord_ currently only supports macOS (Apple Silicon).

1. Download `Chord-aarch64.dmg` from the [latest release](https://github.com/KeyChord/Chord/releases/latest). It is signed and notarized.
2. Open the DMG, drag _Chord_ into `Applications`, and launch it.

Chord stays running in the background and does nothing until you hold down `Caps Lock` + `Space`, which activates _Chord Mode_.

_Chord Mode_ stays active for as long as you keep `Space` held down (you can release `Caps Lock`).

In _Chord Mode_, keys you type will be interpreted as chord inputs and won't be passed through to the focused application.

After typing out a chord, you can execute it by pressing `Caps Lock` (while still keeping `Space` held to stay in _Chord Mode_!)

> For example, with `Space` held down, typing `0,<caps>` opens Chord's settings window.

Once you're done, you can release `Space` to exit _Chord Mode_.

> For sending one-off chords, you can release `Space` right after typing out the chord to execute it. For example, typing `0,` and then releasing `Space` also opens Chord's settings.

<details>
    <summary>Are you a `vim` user?</summary>
    For `vim` users, you can think of _Chord Mode_ as somewhat analagous to _Normal Mode_, with the main difference that it's [a lot easier to exit](https://stackoverflow.com/questions/11828270/how-do-i-exit-vim): just lift up `Space`!
</details>

## Chord Files

All chords are defined in configuration files called _chord files._

A _chord file_ contains chords for a single application and platform, inferred by its path.

For example, a _chord file_ for macOS Finder would live at:

`chords/com/apple/Finder/macos.toml`

<details>
    <summary>Why do chord files end in `.toml`?</summary>
    Chord files are written with a standardized configuration format called [TOML](https://toml.io/en/v1.1.0). You won't need to spend time learning it, as it's very simple and intuitive.
</details>

All _chord files_ live under a top-level `chords/` folder, which itself is part of a _chord package_ that contains both _chord files_ and scripts the chords might use.

Your default _chord package_ is located in `~/.chord/packages/default`.

By adding _chord files_ to this package, you can create your own chords!

For example, if you create the following file in your _chord package:_

`chords/com/apple/Finder/macos.toml`

And paste in the following:

```toml
name = "Finder"

[chords]
# Always use single quotes when defining a chord's key sequence
'gh' = {
  # The `name` property is a required label for the chord
  name = 'Goto Home', 
  
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

You'll then be able to type `/f<caps>` in chord mode to open Finder, and then `gh<caps>` to open your home directory!

## Chord Packages

Instead of defining your own chords for every app, Chord provides pre-made chord packages you can use!

Chord's chords contain certain conventions:

`nesw` - Cardinal directions represent navigation
`lrud` - Positional directions represent movement

> For a full list of these conventions, see the [KeyChords/chords](https://github.com/KeyChord/chords) repo!

To add a Chord package, you can use the Git URL
