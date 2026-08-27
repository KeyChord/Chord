# Development

## Toolchain

Chord uses [proto](https://moonrepo.dev/proto) to install the versions of Bun and Rust pinned for the repository.

Install proto on macOS, Linux, or WSL:

```sh
bash <(curl -fsSL https://moonrepo.dev/install/proto.sh) 0.61.1 --yes
```

Restart your shell, then install the toolchain and project dependencies from the repository root:

```sh
proto install
bun install --frozen-lockfile
```

Start the desktop app with:

```sh
bun run dev
```

The project tool versions are declared in [`.prototools`](./.prototools). Rust is also pinned in [`rust-toolchain.toml`](./rust-toolchain.toml) because proto delegates Rust version selection to rustup.

## Stores

There can be many owners of Observables (e.g. the AppHandle needs to `.manage` it so we can read the current state when initializing a window, and certain structs should be able to own it in order to modify it).
