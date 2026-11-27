# LEON-562: global locked-prefix Instant Mode

While Chord mode is active (entered with Caps+Space), type a global symbol-led
prefix such as `-` and press Shift. The current buffer becomes one retained prefix.
Every fresh action-key press while either Shift is held executes prefix + key,
without putting that key in the buffer. Release both Shift keys to buffer a
multikey suffix. The next transition from neither Shift to either Shift executes
that suffix once and restores the original prefix. An empty suffix never runs the
prefix. Shift-first activation locks the first global symbol key, including Shift
already held when Chord mode begins. Key autorepeat does not repeat input actions.

The existing `blocks` boundary holds the original prefix; `instant_mode` is `Some(true)`
while instant, `Some(false)` while buffering, and `None` outside this mode. There is
no separate global-prefix character setting in this input path. Scope follows the
existing symbol-led global chord namespace. Letter-led local chords keep their
Shift blocks; leading `0`, repeat counts, quote insertion and backtick configuration
retain their existing behavior. Quote/backtick starters are syntax, not Instant
Mode prefixes. Chord keys retain their physical-key mapping: Shift+Minus is `-`
although a JavaScript KeyboardEvent reports `_`.

Escape and Shift+Backspace clear the lock. Unshifted Backspace removes suffix keys
first, then the locked block. Leaving Chord mode resets everything. Caps, Space
completion and Tab keep their existing roles. Macro/escape expansion edits only a
buffered suffix, so it cannot rewrite the locked prefix. Keyboard callbacks remain
included when a Shift press completes buffered input.

## Reproducible cloud checks

From the repository root, with its pinned Rust and Bun versions:

```sh
cargo test --locked --manifest-path apps/chord-tauri/src-tauri/tests/instant-mode/Cargo.toml
bun test ./packages/dev/improve/chord/routes/chords/index
bun apps/chord-tauri/_build.ts
```

The portable Rust target compiles production key mapping, command/macro/escape
models, observable input state and Instant Mode reducer. Its build script also
copies the exact production `ChordInputManager::handle_key_event` and buffer helper
bodies into an integration harness. App services, action execution and the panel
are simulated; key events are synthetic. This checks the entry point and buffer
transitions without Tauri system libraries. It does not prove macOS event-tap
capture, real action execution, task lifetime, Tauri observable delivery, permissions,
or rendered WebKit behavior. The normal full Rust test command still needs to run
on a configured development machine:

```sh
cargo test --manifest-path apps/chord-tauri/src-tauri/Cargo.toml --lib
```

## Native desktop verification still pending

Run only in an existing authorized Chord development environment. Do not change
Accessibility, Input Monitoring, Secure Input, signing identities or other system
permissions for this checklist. If the authorized app/runtime is unavailable,
leave these checks pending. Follow the repository's existing development setup
for the matching sibling rbun checkout and use the normal launcher:

```sh
bun install --frozen-lockfile
vpx _dev
```

Use an isolated development profile and load `tests/instant-mode/fixture` (relative
to `apps/chord-tauri/src-tauri`) as its local chord package. These fixture actions
only append markers to `/tmp/chord-leon562-native.log`; `-` is deliberately a
runnable sentinel named `UNEXPECTED_PREFIX`. Avoid conflicting installed `-`
bindings in that profile. Focus Finder for the local regression case. Start a fresh
fixture log before each scenario, and inspect it with:

```sh
: > /tmp/chord-leon562-native.log
tail -f /tmp/chord-leon562-native.log
```

Keep Space held after entering Caps+Space except where a step explicitly says to
release it. Perform the physical key presses manually; this task has not done so.

1. **Prefix first:** type `-`, hold left Shift. The log stays empty. Press and hold
   `a`: one `-a` marker must appear before releasing `a`. Tap `a` twice more, then
   `b`: expect exactly `-a`, `-a`, `-a`, `-b`. No `UNEXPECTED_PREFIX` marker.
2. **Shift first and fast order:** enter a fresh Chord mode, hold Shift, then tap
   physical Minus, then `a`. Expect only `-a`. Repeat with near-simultaneous
   Minus/Shift in both orders, and with right Shift. Also hold Shift before
   Caps+Space activation. No `_` prefix or extra prefix action.
3. **Buffered suffix and cycles:** lock `-`, release Shift, type `ab`, press Shift.
   Expect exactly one `-ab` at that press. Repeat release/type/press with `cd` and
   `ef`, then tap `a` while Shift stays held. Expect `-ab`, `-cd`, `-ef`, `-a`;
   no prior suffix may leak into the next input.
4. **Whole current buffer:** type `-wm`, then hold Shift and tap `a`: expect `-wma`.
   Release Shift, type `ab`, press Shift: expect `-wmab`. Repeat with `cd`:
   `-wmcd`. The retained prefix must stay `-wm`.
5. **Empty suffix, autorepeat and two Shift keys:** lock `-`; repeatedly release
   and re-press Shift without typing. Hold Shift long enough for repeat events;
   press the other Shift; release only one. No marker until `a` is pressed,
   which must still execute `-a` immediately. Only releasing both enables buffering.
   Press and hold `a` long enough for key repeat: expect one input action.
6. **Punctuation and event order:** lock `-`, tap Semicolon and Slash while Shift is
   held. Expect `-;`, `-/`. Release Shift before releasing the punctuation key and
   repeat the activation cycle. No phantom suffix, underscore, colon or question-mark
   sequence may appear. Check a non-US layout separately if that is the target setup;
   the existing physical-key mapping intentionally remains unchanged.
7. **Cancellation and completion:** with a lock and a buffered suffix, Esc clears
   everything; Shift+Backspace also clears it. Unshifted Backspace removes suffix
   keys before removing the locked prefix as a whole. Re-entering after Space
   release must start clean. Space release with only a prefix produces no sentinel;
   Space with a suffix and Caps completion keep existing execution/repeat behavior.
8. **Other modes and modifiers:** in Finder, type `g`, hold Shift and type `d`, release
   Shift, type `e`, release Space. Expect `gde`, retaining the existing local block
   behavior. Check ordinary letter-led chords, `0` commands, numeric repeats,
   quote insertion/Caps+Space closing, Tab escapes, and Ctrl/Alt/Command/Fn combinations
   for regressions. Modifier combinations must not cause instant input actions.
9. **Hint Panel and action lifetime:** in both floating and sidebar layouts, see
   “Instant Mode” while held and “Buffering” while released; the locked portion stays
   highlighted. Completed-action display may show the last executed chord. Confirm
   no clipping or stale panel after rapid cycles. Exercise an existing harmless held
   shortcut/handler and confirm its keys release when the action key is released;
   confirm errors retain the prefix and the following action still works.

Record OS, layout, left/right Shift, app identity, fixture markers, keyboard/UI
observations and pass/fail per scenario. Synthetic test success is not native
verification. No merge, push, release or deployment is part of this task.
