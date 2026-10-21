# Windows Build Guide

> **Issue:** [#869 — Windows build scripts](https://github.com/HubDApp/Dongle-Smartcontract/issues/869)

This guide covers building, testing, and deploying the Dongle smart contract on
**Windows 10 / Windows 11** using **PowerShell 7+** (`pwsh`).

---

## Table of Contents

1. [Prerequisites](#1-prerequisites)
2. [Installing the Stellar CLI](#2-installing-the-stellar-cli)
3. [Quick Start](#3-quick-start)
4. [Available Actions](#4-available-actions)
5. [PowerShell Equivalents for Every Bash Command](#5-powershell-equivalents-for-every-bash-command)
6. [Environment Variables](#6-environment-variables)
7. [Deployment Walkthrough](#7-deployment-walkthrough)
8. [Troubleshooting](#8-troubleshooting)
9. [CI/CD on Windows](#9-cicd-on-windows)

---

## 1. Prerequisites

| Requirement | Version | How to install |
|---|---|---|
| **PowerShell** | 7.0 + | [Microsoft Store](https://aka.ms/PSWindows) / `winget install Microsoft.PowerShell` |
| **Rust & Cargo** | 1.85.0 (pinned) | `winget install Rustlang.Rustup` → `rustup toolchain install 1.85.0` |
| **wasm32 target** | any | `rustup target add wasm32-unknown-unknown` |
| **Stellar CLI** | 27.x | See [§2](#2-installing-the-stellar-cli) |
| **Git** | 2.40 + | `winget install Git.Git` |

> **Tip:** All Rust toolchain requirements are pinned in `rust-toolchain.toml`.
> When you run `cargo` inside the repo, `rustup` will automatically switch to the
> correct version.

### Execution Policy

PowerShell's default execution policy blocks unsigned scripts. Enable script
execution for the current user:

```powershell
Set-ExecutionPolicy -Scope CurrentUser -ExecutionPolicy RemoteSigned
```

---

## 2. Installing the Stellar CLI

The Stellar CLI ships as a single self-contained binary.  **Do not** use
`cargo install stellar-cli` — the published crate requires an MSRV newer than
the toolchain this repo pins and the build will fail.

### Option A — Prebuilt ZIP (recommended)

```powershell
$VERSION = "27.1.0"
$ZipUrl  = "https://github.com/stellar/stellar-cli/releases/download/v$VERSION/stellar-cli-$VERSION-x86_64-pc-windows-msvc.zip"
$Dest    = "$Env:USERPROFILE\.stellar-cli"

New-Item -ItemType Directory -Force -Path $Dest | Out-Null
Invoke-WebRequest -Uri $ZipUrl -OutFile "$Env:TEMP\stellar-cli.zip"
Expand-Archive -Path "$Env:TEMP\stellar-cli.zip" -DestinationPath $Dest -Force

# Add to your user PATH permanently
$oldPath = [System.Environment]::GetEnvironmentVariable("PATH", "User")
[System.Environment]::SetEnvironmentVariable("PATH", "$oldPath;$Dest", "User")

# Reload PATH for the current session
$Env:PATH = "$Env:PATH;$Dest"

stellar --version   # should print stellar x.y.z
```

### Option B — WinGet (when available)

```powershell
winget install stellar.stellar-cli
```

### Verify Installation

```powershell
stellar --version
# stellar 27.1.0
```

---

## 3. Quick Start

```powershell
# Clone the repository
git clone https://github.com/HubDApp/Dongle-Smartcontract.git
cd Dongle-Smartcontract

# Add wasm32 target (once)
rustup target add wasm32-unknown-unknown

# Build the contract
.\scripts\build_windows.ps1

# Run the test suite
.\scripts\build_windows.ps1 -Action test
```

---

## 4. Available Actions

Run `Get-Help .\scripts\build_windows.ps1 -Full` for the complete reference.

| Action | Description | Bash / Makefile equivalent |
|---|---|---|
| `build` | Compile WASM (default) | `make build` |
| `test` | Run tests on host target | `make test` |
| `test-verbose` | Tests with stdout captured | `make test-verbose` |
| `test-release` | Tests in release mode | `make test-release` |
| `check` | `cargo check` (no artefact) | `make check` |
| `fmt` | `cargo fmt --all` | `make fmt` |
| `lint` | Clippy, warnings → errors | `make lint` |
| `optimize` | Optimize WASM with `stellar` or `wasm-opt` | `./scripts/optimize_wasm.sh` |
| `clean` | `cargo clean` | `make clean` |
| `dev` | fmt + lint + test + build | `make dev` |
| `ci` | check + lint + test | `make ci` |
| `deploy` | Deploy to Stellar network | `./scripts/deploy_testnet.sh` |
| `initialize` | Initialize a deployed contract | `./scripts/initialize.sh` |
| `all` | build + optimize + deploy + initialize | — |

---

## 5. PowerShell Equivalents for Every Bash Command

### Install Dependencies

| Bash | PowerShell |
|---|---|
| `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` | `winget install Rustlang.Rustup` |
| `rustup target add wasm32-unknown-unknown` | `rustup target add wasm32-unknown-unknown` *(identical)* |
| `cargo install --locked soroban-cli --features opt` | See [§2](#2-installing-the-stellar-cli) — use prebuilt binary |

### Build

| Bash | PowerShell |
|---|---|
| `cargo build --target wasm32-unknown-unknown --release` | `cargo build --target wasm32-unknown-unknown --release` *(identical)* |
| `stellar contract build` | `stellar contract build` *(identical)* |
| `make build` | `.\scripts\build_windows.ps1 -Action build` |

### Test

| Bash | PowerShell |
|---|---|
| `cargo test` | `cargo test` *(identical)* |
| `cargo test -- --nocapture` | `cargo test -- --nocapture` *(identical)* |
| `make test` | `.\scripts\build_windows.ps1 -Action test` |
| `make test-verbose` | `.\scripts\build_windows.ps1 -Action test-verbose` |

### Format & Lint

| Bash | PowerShell |
|---|---|
| `cargo fmt --all -- --check` | `cargo fmt --all -- --check` *(identical)* |
| `cargo clippy ... -D warnings` | `cargo clippy ... -D warnings` *(identical)* |

### Optimize WASM

| Bash | PowerShell |
|---|---|
| `./scripts/optimize_wasm.sh` | `.\scripts\build_windows.ps1 -Action optimize` |
| `stellar contract optimize --wasm <in> --wasm-out <out>` | `stellar contract optimize --wasm <in> --wasm-out <out>` *(identical)* |

### Deploy

| Bash | PowerShell |
|---|---|
| `export DEPLOYER_IDENTITY=alice` | `$Env:DEPLOYER_IDENTITY = "alice"` |
| `./scripts/deploy_testnet.sh` | `.\scripts\build_windows.ps1 -Action deploy` |
| `DEPLOYER_IDENTITY=alice ./scripts/deploy_testnet.sh` | `.\scripts\build_windows.ps1 -Action deploy -DeployerIdentity alice` |

### Initialize

| Bash | PowerShell |
|---|---|
| `export CONTRACT_ID=C...` | `$Env:CONTRACT_ID = "C..."` |
| `./scripts/initialize.sh` | `.\scripts\build_windows.ps1 -Action initialize` |

### Environment Variables

| Bash | PowerShell (session) | PowerShell (permanent) |
|---|---|---|
| `export VAR=value` | `$Env:VAR = "value"` | `[System.Environment]::SetEnvironmentVariable("VAR","value","User")` |
| `unset VAR` | `Remove-Item Env:VAR` | `[System.Environment]::SetEnvironmentVariable("VAR",$null,"User")` |
| `source .env` | Handled automatically by `build_windows.ps1` | — |

### Path Separators

Windows uses `\` as the directory separator and `;` as the `PATH` separator.
The PowerShell script normalises paths automatically, but if you run raw `cargo`
or `stellar` commands, use backslashes or PowerShell's `Join-Path` cmdlet:

```powershell
# Bash
# ./target/wasm32-unknown-unknown/release/dongle_contract.wasm

# PowerShell
Join-Path $PWD "target" "wasm32-unknown-unknown" "release" "dongle_contract.wasm"
```

---

## 6. Environment Variables

The script reads a `.env` file in the repo root (same format as on Linux/macOS)
and exports variables into the current process before executing.

```ini
# .env  (repo root — do NOT commit secrets)
DEPLOYER_IDENTITY=alice
NETWORK=testnet
CONTRACT_ID=CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
ADMIN_ADDRESS=GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
```

Variables can also be passed directly as parameters:

```powershell
.\scripts\build_windows.ps1 `
    -Action deploy `
    -DeployerIdentity alice `
    -Network testnet
```

| Variable | Default | Description |
|---|---|---|
| `DEPLOYER_IDENTITY` | *(required for deploy)* | Stellar CLI identity name |
| `NETWORK` | `testnet` | Stellar network (`testnet` \| `local`) |
| `RPC_URL` | Derived from `NETWORK` | Custom RPC endpoint |
| `PASSPHRASE` | Derived from `NETWORK` | Network passphrase |
| `CONTRACT_ID` | Read from `.contract_id` | Contract address for initialize |
| `ADMIN_ADDRESS` | Resolved from identity | Admin address for initialize |

---

## 7. Deployment Walkthrough

### Step 1 — Build

```powershell
.\scripts\build_windows.ps1 -Action build
# ✓ Build complete: target\wasm32-unknown-unknown\release\dongle_contract.wasm
```

### Step 2 — Optimize (optional but recommended)

```powershell
.\scripts\build_windows.ps1 -Action optimize
# ✓ Optimized WASM written to target\...\dongle_contract.optimized.wasm
```

### Step 3 — Deploy

```powershell
$Env:DEPLOYER_IDENTITY = "alice"
.\scripts\build_windows.ps1 -Action deploy
# ✓ Contract deployed successfully
# Contract ID saved to .contract_id
```

### Step 4 — Initialize

```powershell
.\scripts\build_windows.ps1 -Action initialize
# ✓ Contract initialized
```

Or run all four steps in one command:

```powershell
.\scripts\build_windows.ps1 -Action all -DeployerIdentity alice
```

---

## 8. Troubleshooting

### `Set-ExecutionPolicy` blocked

```
.\scripts\build_windows.ps1 : File ... cannot be loaded because running scripts is disabled
```

Fix:
```powershell
Set-ExecutionPolicy -Scope CurrentUser -ExecutionPolicy RemoteSigned
```

### `rustup` not found after install

Close and reopen your PowerShell terminal so `PATH` changes take effect, or:

```powershell
$Env:PATH = "$Env:USERPROFILE\.cargo\bin;$Env:PATH"
```

### wasm32 target missing

```
error[E0463]: can't find crate for `core`
```

Fix:
```powershell
rustup target add wasm32-unknown-unknown
```

### `stellar` not found

```
stellar : The term 'stellar' is not recognized
```

Follow [§2](#2-installing-the-stellar-cli) and ensure the binary directory is on
`$Env:PATH`.

### `cargo build` fails with MSRV errors

This happens when `cargo install stellar-cli` is used instead of the prebuilt
binary. The prebuilt binary bundles `wasm-opt` and has no MSRV constraints on
this project's toolchain. See [§2](#2-installing-the-stellar-cli).

### Line-ending issues with `.env`

If your `.env` file has Windows CRLF line endings and the bash scripts reject
it, convert with:

```powershell
(Get-Content .env) | Set-Content .env
```

### Antivirus false positives

Some AV scanners flag WASM binaries or Rust-compiled executables. If `cargo
build` or `stellar` is blocked, add exclusions for:
- `%USERPROFILE%\.cargo\`
- `%USERPROFILE%\.rustup\`
- The repo's `target\` directory

---

## 9. CI/CD on Windows

The CI pipeline (`.github/workflows/ci.yml`) includes a `windows-build` job
that runs all lint, test, and build steps on `windows-latest`.

### Running CI Steps Locally on Windows

```powershell
# Mirrors the CI workflow exactly
.\scripts\build_windows.ps1 -Action ci

# Or run individual steps
cargo fmt --all -- --check
cargo clippy -p dongle-contract --target wasm32-unknown-unknown --all-features -- -D warnings
cargo test -p dongle-contract
cargo build -p dongle-contract --target wasm32-unknown-unknown --release
```

### Notes on Windows-Specific CI Behaviour

- The `validate-changelog` and `validate-manifest` jobs run Python scripts;
  Python 3 is pre-installed on `windows-latest` GitHub runners.
- The `optimize` job downloads the Stellar CLI prebuilt ZIP for Windows
  (`x86_64-pc-windows-msvc`) rather than the Linux tarball used by the
  `ubuntu-latest` job.
- Path separators in `stellar contract optimize` output use `\` on Windows;
  the PowerShell script normalises this transparently.
