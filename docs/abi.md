# Rune WASM ABI

This document defines the contract between the Rune runtime (host)
and user-provided WebAssembly modules (guest).

---

## Function Signature

WASM modules must export:

```rust
extern "C" fn handler(ptr: i32, len: i32) -> i32
