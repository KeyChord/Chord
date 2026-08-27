# llrt_core SDK endpoint map patch

JSSG codemod that patches `llrt_core/build.rs` in the [KeyChord LLRT fork](https://github.com/KeyChord/llrt) to comment out the `generate_sdk_client_endpoint_map` build-script call. Chord embeds LLRT for chord scripting only and does not need the AWS SDK endpoint map that step generates from `sdk.cfg`.

## Why the patch exists

Upstream LLRT's `build.rs` always runs:

```rust
generate_sdk_client_endpoint_map(&out_dir)?;
```

That reads `../sdk.cfg` and emits AWS SDK client endpoint metadata into `OUT_DIR`. Chord's Tauri app depends on LLRT via git (`llrt_core`, `llrt_modules`, `llrt_os`) for embedded JavaScript execution, not for the AWS Lambda runtime bundle.

## What the codemod does

Finds every `generate_sdk_client_endpoint_map(...)` call, walks up to the enclosing statement (including the trailing `?;`), and prefixes it with `// ` while preserving indentation:

```rust
// generate_sdk_client_endpoint_map(&out_dir)?;
```

Re-running is a no-op when the statement is already commented out.

## Layout

```
codemod.yaml      # Codemod package metadata
workflow.yaml     # Standalone codemod workflow
patch.json        # Manifest: { gitDependency, bundles[] }
codemod.ts        # JSSG transformation
@fixtures/        # JSSG fixture pairs (input.rs / expected.rs)
```

`patch.json` targets:

```
llrt_core/build.rs   # parsed as rust
```

Unlike sanji's npm patches, LLRT is a **git Cargo dependency**. Apply this codemod directly against a checkout of the fork at the rev pinned in `apps/chord-tauri/src-tauri/Cargo.toml`, then commit the result to the fork.

## Apply the patch to the LLRT fork

```sh
cd /path/to/llrt
codemod jssg run --target llrt_core/build.rs -l rust --allow-fs --allow-dirty \
  /path/to/chord/dev/improve/chord/configs/patching/codemods/llrt/codemod.ts
```

## Run the JSSG tests

```sh
bun run --filter @chord/dev.improve.chord.configs.patching.codemods.llrt test
```

This runs:

```sh
npx --yes codemod@latest jssg test -l rust ./codemod.ts ./@fixtures --strictness loose
```
