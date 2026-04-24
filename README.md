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

Early development — architecture-first implementation in progress.

## WASM ABI
See [docs/abi.md](docs/abi.md) for the Rune ↔ WASM interface contract that guest modules must implement.


## Milestone

Core runtime        ██████████  (done)

Registry            █████████░

ABI                 ████████░░

HTTP                ░░░░░░░░░░

Tooling             ░░░░░░░░░░
