# Native file-picker listener

> [!NOTE]
> This document is largely AI-written, so parts of it may not be fully accurate. It will be replaced with a hand-written version when I get the time.

The fixture and production listener are Rust, using typed Apple framework
bindings. From the repository root, compile them and check AppKit's
nonlocalized Open/Save identifiers:

```sh
bun apps/chord-tauri/src-tauri/tests/file-picker/verify.ts
```

Add `--ui` to exercise real native panels in a controlled AppKit fixture. The
executable needs Accessibility access and an active desktop session. It returns
77 when permission is unavailable; it never prompts or changes permissions.

```sh
cargo run --manifest-path apps/chord-tauri/src-tauri/Cargo.toml --example file-picker -- --ui
```

The Bun entry point accepts `--ui` too. The fixture is an opt-in Cargo example,
so ordinary test runs do not initialize AppKit or display windows. No
C/Objective-C compiler step is needed.

The fixture verifies an unrelated alert remains inactive, a native Open sheet
activates the listener, switching to another document clears it, returning to the
sheet reactivates it, cancellation clears it, a standalone Save panel works with
a custom title, attaching to an already-open picker works, and disabling releases
the listener and clears its state. Assertions follow Accessibility callbacks;
there is a single overall timeout and no polling loop. Every panel is canceled.

Rust's `file_picker_tests` cover scope priority, fallback, deduplication,
preservation of the actual foreground app, and the protected core namespace.

Caps key tests live in `caps_remap.rs` (mapping merge, holder liveness, restoring
displaced entries), `caps_lock_guard.rs` (toggle rules against a simulated
computer), `caps_guardian.rs` (agent definition) and `tap.rs` (decoding the
remapped key). The ignored `reads_native_caps_lock_without_changing_it` test
exercises the real IOKit API without posting input or toggling Caps Lock:

```sh
cargo test --manifest-path apps/chord-tauri/src-tauri/Cargo.toml --lib reads_native_caps_lock_without_changing_it -- --ignored
```
