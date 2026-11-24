# Chord

**Multi-letter shortcuts for macOS, Linux, and Windows.**

**Chord** lets you bind actions to key sequences while _keeping your existing shortcuts untouched_.

## Installation

### macOS

Download `Chord-aarch64.dmg` for Apple silicon or `Chord-x86_64.dmg` for Intel from the [latest release](https://github.com/KeyChord/Chord/releases/latest). Open the signed disk image, drag **Chord** into **Applications**, and launch it.

### Linux and Windows

Download the installer from the [latest release](https://github.com/KeyChord/Chord/releases/latest).

- On Windows 10 or 11 (x64), run `Chord-windows-x86_64-setup.exe` and launch Chord from the Start menu. Chord sends keys to apps running at the same privilege level. Elevated windows and the secure desktop are outside that session.
- On Linux (x86-64), install `Chord-linux-x86_64.deb` with your package manager or make `Chord-linux-x86_64.AppImage` executable and run it. Choose an **X11** desktop session at your login screen.

For Linux keyboard capture, Chord needs read access to the physical keyboard's `/dev/input/event*` device and write access to `/dev/uinput`. On Debian and Ubuntu, a local administrator can configure those permissions as follows:

```sh
sudo modprobe uinput
echo uinput | sudo tee /etc/modules-load.d/chord.conf
sudo groupadd -f input
sudo usermod -aG input "$USER"
echo 'KERNEL=="uinput", GROUP="input", MODE="0660", OPTIONS+="static_node=uinput"' | sudo tee /etc/udev/rules.d/70-chord-uinput.rules
sudo udevadm control --reload-rules
sudo udevadm trigger --subsystem-match=misc --sysname-match=uinput
```

Sign out and back in after changing group membership. Membership in `input` lets processes under your account read input devices. Run Chord as your normal user.

On Linux and Windows, Chord installs starter packages for navigation, mouse clicks, menus, modifiers, window controls, the file manager, and system settings on first launch.

Chord opens guided setup on first launch. Open **Settings → General → Getting Started** to revisit it.

# Setup

## Permissions

On macOS, Chord requires _Accessibility Permissions_ (called _Device Control and Data Access_ on macOS 27) so it can detect when you press <kbd>⇪</kbd> (Caps Lock) + <kbd>␣</kbd> (Space) to activate _Chord Mode_.

## Chord Mode

Once you enter _Chord Mode_, it stays active as long as <kbd>␣</kbd> is held down.

_Only <kbd>␣</kbd> needs to stay pressed; you should release <kbd>⇪</kbd> after pressing <kbd>␣</kbd> 

In _Chord Mode_, any keys you type will be intercepted by Chord as input. These key events won't be passed to the focused application.

While _Chord Mode_ is active, the _Hint Panel_ shows what you've typed and which keys can come next. Press <kbd>Tab</kbd> to hide it, and again to show it. To start with it hidden, turn on **Hide Hint Panel by default** in Chord's settings (General → Application).

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

Use `chords/macos.toml`, `chords/linux.toml`, or `chords/windows.toml` for global chords. An app-specific file goes in `chords/<app ID with dots replaced by slashes>/<platform>.toml`. Linux and Windows packages can declare `executables = ["program-name"]` in app files to associate those bindings with a focused application. Use the full executable filename on Windows, such as `Code.exe`. Linux also checks X11 `WM_CLASS`. A shared TOML file can be imported by platform files.

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

Open **Browse** in Chord's settings and choose **Install** for an official package. Official packages are loaded through `~/.chord/packages/@keychord/`.

### Installing Third-Party Chord Packages

> **WARNING:** Since Chord scripts can run arbitrary code (necessary for macOS scripting), you should ONLY install chord packages from people you trust or after auditing code.

To install a third-party package, first manually download and extract it into a `~/.chord/packages/<name>` folder.

Then, you need to quit and restart Chord, and manually approve it in the Chords tab.

In **Chords → Packages → Third-Party Packages**, choose **Review…**, review the package path and warning, then choose **Approve and Enable**. Until you approve it, Chord does not enable any of its chords or load its handlers. Approval persists across restarts; open **Approved packages** and choose **Revoke** to disable it again.

Official packages live under `~/.chord/packages/@keychord/` and do not require this approval. Every package outside that folder is untrusted by default, even if its `package.json` claims an `@keychord` name. Chord approves personal packages when you create them through **Local Chords**; existing folders require approval.

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
