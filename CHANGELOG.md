# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.3](https://github.com/Akshay2642005/rune/compare/v0.2.2...v0.2.3) - 2026-05-09

### Added

- feat added auth utils and <AuthGuard> with login screen and commented
- Integrate Leptos UI components using rust-ui initialization ..
- *(tui)* overhaul dashboard with overlays, async I/O, and Config tab
- *(runtime)* implement wasm execution engine with ABI and response

### Fixed

- apply CodeRabbit auto-fixes
- *(runtime)* add memory resource limiting to WasmExecutor via StoreLimits

### Other

- Merge pull request #21 from Akshay2642005/dependabot/cargo/cargo-4410b433dd
- (feat): fixed AuthGuard and proxy spa for the trunk serve and all rouets
- Update leptos dependencies and add utilities
- Remove  nightly feature from Leptos
- Move rust-toolchain.toml to root
- (bump): update Changelog and bump to v0.2.2
- (spec): added tui components and notification toast helper function for
- (spec): update tui ux design - working-1
- Bump version to 0.2.1 and update changelog
- Make route optional and auto-generate when omitted
- Add ACME TLS wildcard support
- Bump crate versions to 0.2.0 and simplify deploy logic
- Update crates/cli/src/client.rs
- Potential fix for pull request finding 'CodeQL / Cleartext transmission of sensitive information'
- Generalize duplicate-route to DuplicateIdentifier
- Add control-plane API and SQLite-backed registry
- (release): release-v0.1.1-alpha
- Harden 0.1.1-alpha runtime and deploy flow
- (release): re-release 0.1.0-alpha
- Add CLI deploy flow, runtime bootstrap, and CI
- Add HTTP server with Axum handler
- Update README.md
- Add WASM ABI, serialization, and executor improvements
- (bump) add required readmes ..
- (init): initialize repo with README.md
- Tidy imports, allow dead code, update WASM fixture
- Support base64-encoded request bodies
- add regsitry crate along with InMemoryFunctionStore and unit tests
- implement impl std::error::Error for RuneError
- fixup review issues
- added minimal function metadata, function storage trait
- add initial contract layer (request, response, error, config)
- Update crates/registry/src/lib.rs
- Update crates/runtime/src/executor.rs

## [0.2.2](https://github.com/Akshay2642005/rune/releases/tag/v0.2.2) - 2026-05-08

### Added

- TUI dashboard: `Config` tab for editing control-plane URL, function URL, and API key in-TUI; `F2` saves to disk and rebuilds the client live.
- TUI dashboard: deploy form overlay (`D`) — deploy a WASM function by filling in id, route, subdomain, and path without leaving the dashboard.
- TUI dashboard: invoke overlay (`i`) — fire a GET request to the selected function's route and view the response in a scrollable popup.
- TUI dashboard: confirm prompt before destructive actions (delete function, revoke key).
- TUI dashboard: create key dialog (`n`) — create API keys from within the dashboard.
- TUI dashboard: live search/filter (`/`) across functions and keys; list titles show `(filtered/total)` counts.
- TUI dashboard: clipboard copy (`c`) — copies function URL or key ID via `arboard`.
- TUI dashboard: splash screen with animated spinner on startup; shows a clear error and exits gracefully if the server is unreachable.
- TUI dashboard: connection status dot in the header (green / yellow loading / red offline).
- TUI dashboard: vim-style half-page scroll (`Ctrl-d` / `Ctrl-u`).
- `RuneClient::with_function_url` constructor; `function_url` field added to `RuneConfig` for separate control-plane vs. function-traffic URLs.
- `RuneClient` now exposes `server_url()`, `api_key()`, `function_url`, and `invoke_function()` helpers.

### Changed

- All TUI network calls moved off the event loop into `tokio::spawn` tasks communicating via `mpsc::Sender<BgResult>`; the UI never blocks on I/O.
- `POLL_INTERVAL` reduced from 250 ms → 100 ms; `REFRESH_TIMEOUT` raised from 2 s → 5 s.
- `DashboardTab` gains a third variant `Config`; tab cycling updated accordingly.
- Footer toast moved to a dedicated row; footer left now shows INSERT / NORMAL / CONFIRM / INVOKE mode label plus active search query.
- Help popup expanded with grouped Navigation / Actions sections.
- `RuneClient` derives `Clone`.

## [0.2.1](https://github.com/Akshay2642005/rune/releases/tag/v0.2.1) - 2026-05-01(https://github.com/Akshay2642005/rune/releases/tag/v0.2.1) - 2026-05-01

### Added

- Auto-generate function routes when omitted and reuse existing routes on redeploy.
- Allow optional routes in the CLI deploy flow and control-plane API.
- Route `"/"` through the function handler to support subdomain-only access.

### Changed

- Install the Rustls crypto provider explicitly at startup to avoid runtime panics.
- Allow local HTTP control-plane URLs in the CLI (with a warning).
- Upgrade ACME provisioning to `instant-acme` 0.8.5 and update the DNS-01 flow.
- Improve host detection for subdomain routing by falling back to URI authority.

### Fixed

- Align the ACME DNS TXT prompt box output.
- Normalize HTTPS + control-plane join error handling in TLS mode.

## [0.2.0](https://github.com/Akshay2642005/rune/releases/tag/v0.2.0) - 2026-04-28

### Added

- *(runtime)* implement wasm execution engine with ABI and response

### Fixed

- apply CodeRabbit auto-fixes
- *(runtime)* add memory resource limiting to WasmExecutor via StoreLimits

### Other

- Update release-plz workflow and config
- Bump crate versions to 0.2.0 and simplify deploy logic
- Update src/handler.rs
- Potential fix for pull request finding 'CodeQL / Uncontrolled data used in path expression'
- Potential fix for pull request finding 'CodeQL / Uncontrolled data used in path expression'
- Generalize duplicate-route to DuplicateIdentifier
- Add control-plane API and SQLite-backed registry
- (chore): added sqlx and tower dependencies
- release v0.1.1-alpha
- (release): release-v0.1.1-alpha
- Harden 0.1.1-alpha runtime and deploy flow
- (release): re-release 0.1.0-alpha
- (release): re-release 0.1.0-alpha
- Update release-plz.yml
- Add CLI deploy flow, runtime bootstrap, and CI
- Add HTTP server with Axum handler
- Tidy imports, allow dead code, update WASM fixture
- Support base64-encoded request bodies
- Update README.md
- Update crates/runtime/src/executor.rs
- Update crates/registry/src/lib.rs
- Add WASM ABI, serialization, and executor improvements
- add regsitry crate along with InMemoryFunctionStore and unit tests
- Create dependabot.yml for version updates
- implement impl std::error::Error for RuneError
- fixup review issues
- added minimal function metadata, function storage trait
- add initial contract layer (request, response, error, config)
- Update SECURITY.md
- (bump) add required readmes ..
- (init): initialize repo with README.md

## [0.1.1-alpha](https://github.com/Akshay2642005/rune/releases/tag/v0.1.1-alpha) - 2026-04-26

### Added

- *(runtime)* implement wasm execution engine with ABI and response

### Fixed

- apply CodeRabbit auto-fixes
- *(runtime)* add memory resource limiting to WasmExecutor via StoreLimits

### Other

- (release): release-v0.1.1-alpha
- Harden 0.1.1-alpha runtime and deploy flow
- (release): re-release 0.1.0-alpha
- (release): re-release 0.1.0-alpha
- Update release-plz.yml
- Add CLI deploy flow, runtime bootstrap, and CI
- Add HTTP server with Axum handler
- Tidy imports, allow dead code, update WASM fixture
- Support base64-encoded request bodies
- Update README.md
- Update crates/runtime/src/executor.rs
- Update crates/registry/src/lib.rs
- Add WASM ABI, serialization, and executor improvements
- add regsitry crate along with InMemoryFunctionStore and unit tests
- Create dependabot.yml for version updates
- implement impl std::error::Error for RuneError
- fixup review issues
- added minimal function metadata, function storage trait
- add initial contract layer (request, response, error, config)
- Update SECURITY.md
- (bump) add required readmes ..
- (init): initialize repo with README.md

- Added the `rune` CLI deploy flow that copies wasm artifacts into `.rune`,
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
