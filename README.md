# Rune

Rune is a self-hosted WASM function runtime inspired by Cloudflare Workers and Deno Deploy — wildcard TLS, per-function subdomains, and a clean REST control plane included.

## Features

- **Wasmtime-based sandboxed execution** with per-function fuel + memory limits
- **Automatic wildcard TLS** via Let's Encrypt DNS-01 (instant-acme + rustls)
- **Per-function subdomains** — deploy `hello` and reach it at `hello.yourdomain.com`
- **REST control plane** — deploy, list, and remove functions via HTTP API
- **CLI** (`rune`) for deploys and API key management
- **SQLite persistence** — single-file database, zero ops
- **Self-hostable** — single binary, works on any Linux VPS

## Quick start (plain HTTP, no domain)

```bash
# Install
curl -fsSL https://raw.githubusercontent.com/Akshay2642005/rune/master/install.sh | sh

# Start the server (first run prints a default API key)
rune-server

# Save the printed key
rune auth save --key rune_sk_<printed-key>

# Build and deploy the example function
cargo build --manifest-path examples/hello/Cargo.toml --target wasm32-unknown-unknown
rune deploy --id hello --route /hello examples/hello/target/wasm32-unknown-unknown/debug/hello.wasm

# Invoke it
curl http://localhost:3000/hello
```

## TLS + wildcard subdomains

Set these environment variables before starting the server:

| Variable           | Required | Example              | Description                                   |
|--------------------|----------|----------------------|-----------------------------------------------|
| `RUNE_DOMAIN`      | yes      | `example.com`        | Your base domain. Enables TLS.                |
| `RUNE_ACME_EMAIL`  | yes      | `you@example.com`    | Contact email for Let's Encrypt account.      |
| `RUNE_ACME_STAGING`| no       | `1`                  | Use LE staging (testing only, untrusted cert). |
| `RUNE_CERT_DIR`    | no       | `.rune/certs`        | Where to store certs and ACME account.        |

On first start, the server will print a DNS TXT record to set:

```
╔══════════════════════════════════════════════════════════════╗
║              RUNE — TLS certificate provisioning             ║
╠══════════════════════════════════════════════════════════════╣
║  Add the following DNS TXT record to your DNS zone:          ║
║                                                              ║
║  Name:  _acme-challenge.example.com                          ║
║  Type:  TXT                                                  ║
║  Value: <token>                                              ║
║                                                              ║
║  Wait for DNS propagation (~60 s), then press Enter.         ║
╚══════════════════════════════════════════════════════════════╝
```

After that, the server binds:
- `:443` — HTTPS function traffic
- `:80`  — HTTP → HTTPS redirect
- `:3001` — control plane API (localhost only)

### Deploy with subdomain

```bash
rune deploy --id hello --route /hello --subdomain hello hello.wasm
```

The function is now reachable at both `https://hello.example.com` and
`https://example.com/hello`.

## VPS deployment (Docker Compose)

```bash
git clone https://github.com/Akshay2642005/rune
cd rune

# Set your domain and email
export RUNE_DOMAIN=example.com
export RUNE_ACME_EMAIL=you@example.com

# First run — interactive so you can see the TXT record prompt
docker compose run --rm rune

# After cert is provisioned, run as daemon
docker compose up -d
```

Make sure ports 80 and 443 are open in your firewall/security group, and your
DNS `A` record for `*.example.com` points to your VPS IP.

## Environment variables (full list)

| Variable         | Default             | Description                          |
|------------------|---------------------|--------------------------------------|
| `RUNE_DOMAIN`    | —                   | Base domain. TLS off if unset.       |
| `RUNE_ACME_EMAIL`| —                   | Let's Encrypt contact email.         |
| `RUNE_ACME_STAGING` | —                | Set to `1` for LE staging.          |
| `RUNE_CERT_DIR`  | `.rune/certs`       | Cert storage dir.                    |
| `RUNE_DB_PATH`   | `.rune/rune.db`     | SQLite database path.                |
| `RUNE_WASM_DIR`  | `.rune/functions`   | WASM artifact directory.             |
| `RUNE_API_ADDR`  | `127.0.0.1:3001`    | Control plane bind address.          |
| `RUNE_ADDR`      | `0.0.0.0:3000`/`:80`| Function traffic bind address.       |
| `RUNE_HTTPS_ADDR`| `0.0.0.0:443`       | HTTPS bind address (TLS mode).       |
| `RUST_LOG`       | `info`              | Log level filter.                    |

## WASM ABI

See [docs/abi.md](docs/abi.md) for the guest module interface contract.

## Milestone

| Component        | Status |
|------------------|--------|
| Core runtime     | ██████████ 100% |
| Registry         | ██████████ 100% |
| ABI              | ██████████ 100% |
| HTTP routing     | ██████████ 100% |
| SQLite store     | ██████████ 100% |
| Control plane    | ██████████ 100% |
| CLI (HTTP)       | ██████████ 100% |
| TLS / ACME       | ██████████ 100% |
| Subdomain routing| ██████████ 100% |
| TUI dashboard    | ░░░░░░░░░░   0% |
| Web UI           | ░░░░░░░░░░   0% |

## Releases

GitHub releases are driven by `release-plz`. Binary artifacts for Linux x86_64/aarch64, macOS x86_64/arm64, and Windows x86_64 are attached to every release.

Install with:
```bash
curl -fsSL https://raw.githubusercontent.com/Akshay2642005/rune/master/install.sh | sh
```
