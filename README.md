# Rune

Rune is a sandboxed WASM function runtime inspired by edge platforms like Cloudflare Workers and Deno Deploy.

It provides secure, resource-limited execution with a contract-first architecture, pluggable features, and a deploy pipeline for running user-defined functions.

## Features (Planned)

- Wasmtime-based sandboxed execution
- Per-function CPU and memory limits
- HTTP routing layer
- Function registry and deploy pipeline
- CLI, TUI, and web dashboard

## Status

Alpha vertical slice working: deploy with `rune`, then serve with `rune-server`.

## WASM ABI
See [docs/abi.md](docs/abi.md) for the Rune ↔ WASM interface contract that guest modules must implement.


## Milestone

Core runtime        ██████████  (95%)

Registry            █████████░  (85%)

ABI                 ████████░░  (85%)

HTTP                █████████░  (90%)

Tooling             ████░░░░░░  (40%)

## Alpha Flow

Deploy a function with the CLI:

```bash
cargo run -p runectl -- deploy --id hello --route /hello crates/runtime/tests/fixtures/hello.wasm
```

Start the runtime server:

```bash
cargo run -p rune-server
```

Invoke the deployed function:

```bash
curl http://127.0.0.1:3000/hello
```

For `v0.1.0-alpha`, the CLI and server communicate through local state in `.rune/`.
The future client/server architecture can replace this with a network API without
changing the basic CLI workflow.

For crates.io installs, the intended commands are:

```bash
cargo install runectl
cargo install rune-server
```

## Releases

GitHub releases are driven by `release-plz` from `.github/workflows/release-plz.yml`.
To let the published release trigger the binary upload workflow, configure a
`RELEASE_PLZ_TOKEN` repository secret backed by a personal access token with
permission to create releases and pull requests.
