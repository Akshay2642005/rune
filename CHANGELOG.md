# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

- Added the `rune-cli` deploy flow that copies wasm artifacts into `.rune`,
  persists deployment metadata, and prepares functions for server startup.
- Added server bootstrap loading from `.rune/deployments.json` so deployed
  functions can be served without hardcoded startup registration.
- Added GitHub Actions release automation with `release-plz` plus binary asset
  uploads for server and CLI targets on Linux, macOS, and Windows.
- Shifted deployment ownership from the server binary into the CLI while keeping
  the local file-based boundary that can later be replaced by a network API.
- Updated the README milestone section and documented the alpha deploy/serve
  flow as the first complete vertical slice of the platform.
- Standardized the workspace on the coordinated prerelease version
  `0.1.0-alpha` for the initial GitHub release.

## [0.1.0-alpha] - 2026-04-24

### Added

- Initialized the repository with baseline project documentation, contributing
  notes, security guidance, and automated dependency update configuration
  (`d7f2044`, `fe38d77`, `435e52e`, `fad6b3a`).
- Introduced the `rune-core` contract layer with request/response types,
  runtime config, error types, headers, and function metadata/storage traits
  (`b004fbc`, `1f98c06`, `c94e89c`, `ab78471`, `b6f967c`).
- Added the `rune-registry` crate with a thread-safe in-memory function store
  and unit coverage for registration, duplicate routes, and concurrent access
  (`818c490`, `ae69bef`, `4fcb2f9`).
- Added the `rune-runtime` crate with Wasmtime-based execution, ABI handling,
  JSON request/response bridging, and runtime dispatch to registered functions
  (`7b7a7b3`, `62d4b67`).
- Added an Axum-based HTTP server for routing incoming requests into the runtime
  (`aa2fd1a`).

### Changed

- Updated runtime and registry implementation details during the feature branch
  integration work (`ae69bef`, `b3c3e71`, `70c211f`, `4fcb2f9`).

### Fixed

- Applied review-driven fixes to the core contract layer and runtime internals.
- Tightened executor behavior around memory handling, fixture updates, and
  general runtime correctness issues identified during iteration
  (`7f77f1e`, `aa7b9f5`, `f6d7570`, `84caaa3`).
