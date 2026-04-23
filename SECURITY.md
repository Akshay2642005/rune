
---

# 📄 `SECURITY.md`

Important since you’re building a runtime system:

```md
# Security Policy

## Supported Versions

This project is under active development. Security fixes are applied to the latest version.

## Reporting a Vulnerability

If you discover a security issue, do NOT open a public issue.

Instead:
- Contact maintainers privately
- Provide a clear description and reproduction steps

## Scope

Security issues include:
- Sandbox escapes
- Resource limit bypass (CPU/memory)
- Unauthorized data access
- Execution isolation failures

## Response

We aim to:
- Acknowledge reports promptly
- Investigate and reproduce
- Release fixes as quickly as possible

## Notes

This project executes untrusted WASM code. Do NOT use in production environments without proper auditing.
