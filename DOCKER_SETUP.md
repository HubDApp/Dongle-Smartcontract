# Docker Setup — Dongle Smart Contract (#870)

This guide explains how to build, test, and deploy the Dongle smart contract
inside a fully self-contained Docker environment. No manual Rust, Stellar CLI,
or WASM-target installation is required on your host machine.

---

## Prerequisites

| Tool | Minimum version |
|------|----------------|
| [Docker](https://docs.docker.com/get-docker/) | 24.x |
| [Docker Compose](https://docs.docker.com/compose/install/) | v2.20 |

No other dependencies are needed locally.

---

## Architecture

```
┌─────────────────────────────────┐
│  docker-compose.yml             │
│                                 │
│  ┌─────────────────────────┐    │
│  │  stellar (standalone)   │    │  ← Horizon + Soroban RPC
│  │  port 8000              │    │
│  └───────────┬─────────────┘    │
│              │ dongle-net       │
│  ┌───────────▼─────────────┐    │
│  │  contract (dev image)   │    │  ← Rust 1.85 + Stellar CLI + WASM target
│  │  /app → repo mount      │    │
│  └─────────────────────────┘    │
└─────────────────────────────────┘
```

### Dockerfile stages

| Stage | Purpose |
|-------|---------|
| `builder` | Installs Rust 1.85.0, `wasm32-unknown-unknown` target, Stellar CLI 22.3.0, and compiles the WASM artifact |
| `runtime` | Minimal Debian image carrying only the WASM artifact, toolchain, and source for in-container rebuilds |

The multi-stage build ensures the final `runtime` image is as small as possible
by excluding intermediate compiler artefacts and source from the `builder` stage.

---

## Quick Start

### 1. Start the Stellar standalone node

```bash
docker compose up -d stellar
```

Wait until it is healthy (takes ~30 s on first start):

```bash
docker compose ps
# stellar   running (healthy)
```

### 2. Start the contract dev container

```bash
docker compose up -d contract
```

### 3. Build the contract

```bash
docker compose exec contract make build
# Output: target/wasm32-unknown-unknown/release/dongle_contract.wasm
```

### 4. Run the full test suite

```bash
docker compose exec contract make test
# or verbosely:
docker compose exec contract make test-verbose
```

### 5. Run linting and audit

```bash
docker compose exec contract make lint
docker compose exec contract make audit
```

---

## One-off commands

`docker compose run --rm` lets you execute a single command and remove the
container afterwards — useful for CI or scripted workflows:

```bash
# Build
docker compose run --rm contract make build

# Test
docker compose run --rm contract make test

# Format
docker compose run --rm contract make fmt

# Open an interactive shell
docker compose run --rm contract bash
```

---

## CI: one-shot test runner

A dedicated `test` service runs the suite once and exits with the process exit
code. Activate it with the `test` profile:

```bash
docker compose --profile test run --rm test
```

This is ideal for CI pipelines (GitHub Actions, GitLab CI, etc.) where you want
a clean environment every time.

---

## Environment variables

| Variable | Default | Description |
|----------|---------|-------------|
| `SOROBAN_RPC_URL` | `http://stellar:8000/soroban/rpc` | Soroban RPC endpoint used by Stellar CLI |
| `STELLAR_NETWORK_PASSPHRASE` | `Standalone Network ; February 2017` | Network passphrase for the standalone chain |
| `CARGO_HOME` | `/usr/local/cargo` | Cargo home (maps to a named volume for caching) |
| `CARGO_TARGET_DIR` | `/app/target` | Build artefact directory |

Override them in your shell or in a `.env` file at the repo root before running
`docker compose up`.

---

## Deploy to the local standalone node

```bash
# 1. Create a funded account (keypair) in the standalone network
docker compose exec contract \
    stellar keys generate deployer --network standalone --fund

# 2. Build the contract
docker compose exec contract make build

# 3. Deploy
docker compose exec contract \
    stellar contract deploy \
        --wasm target/wasm32-unknown-unknown/release/dongle_contract.wasm \
        --source deployer \
        --network standalone

# 4. The contract ID is printed on stdout. Store it:
export CONTRACT_ID=<printed-id>

# 5. Initialize the contract (pass your deployer address as admin)
docker compose exec contract \
    stellar contract invoke \
        --id "$CONTRACT_ID" \
        --source deployer \
        --network standalone \
        -- initialize \
        --admin "$(stellar keys address deployer)"
```

---

## Deploying to testnet

```bash
# Create a funded testnet identity
stellar keys generate testnet-deployer --network testnet --fund

# Build and deploy
docker compose exec contract make build
docker compose exec contract \
    stellar contract deploy \
        --wasm target/wasm32-unknown-unknown/release/dongle_contract.wasm \
        --source testnet-deployer \
        --network testnet
```

---

## Volumes and caching

| Volume | Purpose |
|--------|---------|
| `stellar-data` | Stellar Core + Horizon state. Delete to reset the chain. |
| `cargo-registry` | Crate registry cache. Avoids re-downloading on every run. |
| `cargo-git` | Cargo git source cache. |

To do a completely clean reset:

```bash
docker compose down -v   # removes containers AND volumes
docker compose up -d stellar
```

---

## Troubleshooting

### `stellar` container never becomes healthy

The quickstart image needs ~30 s on first boot. Check logs:

```bash
docker compose logs -f stellar
```

If you see `connection refused` from the `contract` container, the stellar node
hasn't finished starting. The `depends_on: condition: service_healthy` in
`docker-compose.yml` already handles this, but CI systems with slow disks may
need a longer `start_period`.

### `cargo: command not found` inside a container

The `CARGO_HOME` and `PATH` are set in the image. If you override `CARGO_HOME`
to a host-mounted path, ensure the binary is on `PATH`:

```bash
export PATH=$CARGO_HOME/bin:$PATH
```

### Build fails with `WASM target not found`

The `rust-toolchain.toml` file in the repo pins the toolchain to Rust 1.85.0
with `wasm32-unknown-unknown` already configured. The image installs this
target during the `builder` stage. If you see this error, try a full image
rebuild:

```bash
docker compose build --no-cache contract
```

### Permission errors on mounted volumes (Linux hosts)

Docker on Linux runs containers as root by default. If generated files (e.g.
`target/`) appear owned by root on the host:

```bash
sudo chown -R $(id -u):$(id -g) target/
```

Or run the container with your UID:

```bash
docker compose run --rm --user "$(id -u):$(id -g)" contract make build
```

---

## Related files

| File | Purpose |
|------|---------|
| `Dockerfile` | Multi-stage build definition |
| `docker-compose.yml` | Service orchestration |
| `DOCKER_SETUP.md` | This document |
| `rust-toolchain.toml` | Pinned Rust version (1.85.0) |
| `dongle-smartcontract/Makefile` | Convenience targets (`build`, `test`, `lint`, …) |
