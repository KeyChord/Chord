# sfsymbols-react remove Next.js dependency patch

JSSG codemod that patches `@bradleyhodges/sfsymbols-react` to remove the unused `next` entry from `package.json` `dependencies`. Chord uses the package for SF Symbol icons in a Tauri + Vite app and never runs Next.js.

## Why the patch exists

`@bradleyhodges/sfsymbols-react@7.0.4` declares:

```json
"dependencies": {
  "next": "^15.3.4"
}
```

The compiled package does not import Next.js. The dependency is packaging noise that pulls the entire Next.js tree into the lockfile.

## What the codemod does

Removes the `"next": …` pair from the top-level `dependencies` object in `package.json`, including the adjacent comma. Re-running is a no-op when `next` is already gone.

## Layout

```
codemod.yaml      # Codemod package metadata
workflow.yaml     # Standalone codemod workflow
patch.json        # Manifest: { npmDependency, bundles[] }
codemod.ts        # JSSG transformation
@fixtures/        # JSSG fixture pairs (input.json / expected.json)
```

Unlike the LLRT codemod (a git Cargo dependency), this is an **npm package** patch applied automatically via Bun `patchedDependencies`.

## Regenerate the Bun patch file

When `@bradleyhodges/sfsymbols-react` is upgraded, re-apply the codemod and commit a fresh patch:

```sh
bun patch @bradleyhodges/sfsymbols-react@<version>
codemod jssg run --target package.json -l json --allow-fs --allow-dirty \
  node_modules/@bradleyhodges/sfsymbols-react/codemod.ts
# (or copy codemod.ts into the package folder and point --target at its package.json)
bun patch --commit 'node_modules/@bradleyhodges/sfsymbols-react'
```

## Run the JSSG tests

```sh
bun run --filter @chord/dev.improve.chord.configs.patching.codemods.sfsymbols-react test
```
