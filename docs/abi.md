# Rune WASM ABI

This document defines the contract between the Rune runtime (host)
and user-provided WebAssembly modules (guest).

---

## Required Exports

WASM modules must export the following items:

- `memory`: the linear memory the host reads from and writes to
- `handler(ptr: i32, len: i32) -> i32`: the request entrypoint

Modules may also export:

- `alloc(size: i32) -> i32`: an optional allocator the host will use when present

### handler

```rust
extern "C" fn handler(ptr: i32, len: i32) -> i32
```

The main request handler. The host calls this function with a pointer and length to a JSON-serialized `CoreRequest` object in guest memory.

**Parameters:**
- `ptr`: Guest memory pointer to the JSON-serialized `CoreRequest`
- `len`: Length of the request data in bytes

**Returns:**
- A guest memory pointer to the response buffer. The first 4 bytes at this pointer contain the response length as a little-endian `u32`, followed by that many bytes of JSON-serialized `WasmResponse` data.
- Returning `0` or a negative value is a fatal error and signals an invalid pointer.

**CoreRequest Format:**
```json
{
  "method": "GET",
  "path": "/hello",
  "headers": [["content-type", "application/json"]],
  "body": "aGVsbG8="
}
```

**WasmResponse Format:**
```json
{
  "status": 200,
  "headers": [["content-type", "text/plain"]],
  "body": [104, 101, 108, 108, 111]
}
```

### alloc

```rust
extern "C" fn alloc(size: i32) -> i32
```

Allocates a buffer in guest memory and returns a pointer to it.

**Parameters:**
- `size`: Number of bytes to allocate

**Returns:**
- Guest memory pointer to the allocated buffer
- Returning `0` or a negative value signals allocation failure

**Memory Requirements:**
- The allocated region must remain valid for the duration of the host read operation
- The host will not call a corresponding `free` function
- Guest implementations typically use `Vec::leak()` or similar mechanisms

**Error Semantics:**
- If `alloc` is not exported, the host will use a hardcoded reserved region starting at offset 8
- Implementations should return `0` for invalid size values (e.g., `size <= 0`)
- The host validates returned pointers and treats `0` or negative values as errors

### Request payload (`CoreRequest`)

The host serializes the `CoreRequest` as JSON and writes it into the guest
memory at the pointer returned by `alloc`. The `body` field is base64-encoded
bytes (a JSON string). Headers are serialized as an array of `[name, value]`
pairs. The `handler` function is invoked with:

- `ptr`: guest pointer to the start of the JSON request bytes
- `len`: length (in bytes) of the JSON request

### Response payload (`WasmResponse`)

`handler` returns a guest pointer to the response layout:

- First 4 bytes: little-endian `u32` response length
- Next `len` bytes: JSON-serialized `WasmResponse` where `body` is a JSON byte
  array and `headers` (if present) is an array of `[name, value]` pairs

### Runtime limits in the current alpha

- The host enforces fuel and linear-memory limits.
- The host does not currently enforce a wall-clock timeout.

### Error semantics

- Returning `0` from `alloc` or `handler` is an invalid pointer and treated as a
  fatal error by the host.

### Memory lifetime requirements

- The guest must ensure the returned response region remains valid for the host
  to read.
- The host will **not** call `free`; the guest is responsible for managing any
  memory it allocates.
