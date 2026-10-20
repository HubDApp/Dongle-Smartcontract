#Requires -Version 7.0
<#
.SYNOPSIS
    Build, test, and optionally deploy the Dongle smart contract on Windows.

.DESCRIPTION
    PowerShell equivalent of the Bash-based build and deployment scripts for
    Windows 10/11 with PowerShell 7+.  Covers:
      - Prerequisite validation (Rust, wasm32 target, Stellar CLI)
      - Building the WASM contract
      - Running the test suite
      - Optimizing the WASM output
      - Deploying to Stellar testnet or local network
      - Initialising a freshly-deployed contract

    All commands mirror what the Makefile and *.sh scripts do so that Windows
    developers get byte-for-byte identical build artefacts.

.PARAMETER Action
    The action to perform. Choices:
      build        - Compile the contract to WASM (default)
      test         - Run the full test suite on the native host
      test-verbose - Run tests with stdout captured (--nocapture)
      test-release - Run tests in release mode
      check        - cargo check --target wasm32-unknown-unknown (no artefact)
      fmt          - cargo fmt --all
      lint         - cargo clippy (warnings → errors)
      optimize     - Optimize the WASM artefact with stellar / wasm-opt
      clean        - cargo clean
      dev          - fmt + lint + test + build (full local dev cycle)
      ci           - check + lint + test   (no build artefact, mirrors CI)
      deploy       - Deploy to Stellar network (needs $Env:DEPLOYER_IDENTITY)
      initialize   - Initialize a deployed contract (needs $Env:CONTRACT_ID)
      all          - build + optimize + deploy + initialize

.PARAMETER Network
    Stellar network to target for deploy/initialize.  Defaults to "testnet".
    Pass "local" to target a local standalone node.

.PARAMETER DeployerIdentity
    Stellar CLI identity name to use for signing. Overrides
    $Env:DEPLOYER_IDENTITY.

.PARAMETER ContractId
    Contract address for the initialize step. Overrides $Env:CONTRACT_ID.
    If absent, the script reads .contract_id from the repo root.

.PARAMETER AdminAddress
    Stellar address to pass to `initialize`. Defaults to the address resolved
    from DeployerIdentity.

.EXAMPLE
    # Build the contract
    .\scripts\build_windows.ps1

.EXAMPLE
    # Run tests
    .\scripts\build_windows.ps1 -Action test

.EXAMPLE
    # Full dev cycle
    .\scripts\build_windows.ps1 -Action dev

.EXAMPLE
    # Deploy to testnet
    $Env:DEPLOYER_IDENTITY = "alice"
    .\scripts\build_windows.ps1 -Action deploy

.NOTES
    Requires:
      - PowerShell 7+ (pwsh)
      - Rust 1.85.0 (as pinned in rust-toolchain.toml)
      - wasm32-unknown-unknown target: rustup target add wasm32-unknown-unknown
      - Stellar CLI 27.x prebuilt binary on PATH
        Download: https://github.com/stellar/stellar-cli/releases
    See WINDOWS_BUILD_GUIDE.md for full setup instructions.
#>

[CmdletBinding()]
param(
    [ValidateSet(
        "build", "test", "test-verbose", "test-release", "check",
        "fmt", "lint", "optimize", "clean", "dev", "ci",
        "deploy", "initialize", "all"
    )]
    [string]$Action = "build",

    [string]$Network = $Env:NETWORK ?? "testnet",

    [string]$DeployerIdentity = $Env:DEPLOYER_IDENTITY,

    [string]$ContractId = $Env:CONTRACT_ID,

    [string]$AdminAddress = $Env:ADMIN_ADDRESS
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

# ── Paths ──────────────────────────────────────────────────────────────────────
$ScriptDir   = $PSScriptRoot
$ProjectRoot = Split-Path -Parent $ScriptDir
$ContractDir = Join-Path $ProjectRoot "dongle-smartcontract"
$WasmPath    = Join-Path $ProjectRoot "target\wasm32-unknown-unknown\release\dongle_contract.wasm"
$OptWasmPath = Join-Path $ProjectRoot "target\wasm32-unknown-unknown\release\dongle_contract.optimized.wasm"
$ContractIdFile = Join-Path $ProjectRoot ".contract_id"
$EnvFile     = Join-Path $ProjectRoot ".env"

# ── Helpers ───────────────────────────────────────────────────────────────────

function Write-Header {
    param([string]$Title)
    Write-Host ""
    Write-Host ("=" * 60) -ForegroundColor Cyan
    Write-Host "  $Title" -ForegroundColor Cyan
    Write-Host ("=" * 60) -ForegroundColor Cyan
}

function Write-Success { param([string]$Msg) Write-Host "✓ $Msg" -ForegroundColor Green }
function Write-Fail    { param([string]$Msg) Write-Host "✗ $Msg" -ForegroundColor Red }
function Write-Info    { param([string]$Msg) Write-Host "  $Msg" -ForegroundColor Yellow }

function Invoke-Checked {
    <#
    .SYNOPSIS Runs an external command and throws on non-zero exit code.
    #>
    param([string[]]$Cmd)
    Write-Verbose "Running: $Cmd"
    & $Cmd[0] $Cmd[1..($Cmd.Length - 1)]
    if ($LASTEXITCODE -ne 0) {
        throw "Command failed with exit code ${LASTEXITCODE}: $Cmd"
    }
}

function Load-EnvFile {
    if (Test-Path $EnvFile) {
        Write-Info "Loading environment from $EnvFile"
        Get-Content $EnvFile | Where-Object { $_ -notmatch '^\s*#' -and $_ -match '=' } | ForEach-Object {
            $parts = $_ -split '=', 2
            $key   = $parts[0].Trim()
            $value = $parts[1].Trim().Trim('"').Trim("'")
            [System.Environment]::SetEnvironmentVariable($key, $value, "Process")
        }
    }
}

function Assert-Command {
    param([string]$Name, [string]$InstallHint)
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        Write-Fail "Required command '$Name' not found on PATH."
        Write-Info  $InstallHint
        throw "Missing prerequisite: $Name"
    }
}

function Assert-Prerequisites {
    Write-Header "Checking Prerequisites"

    Assert-Command "cargo" "Install Rust: https://rustup.rs"
    Assert-Command "rustup" "Install Rust: https://rustup.rs"

    # Check Rust version
    $rustVersion = (rustup show active-toolchain 2>&1) -as [string]
    Write-Info "Active toolchain: $rustVersion"

    # Check wasm32 target
    $targets = rustup target list --installed 2>&1
    if ($targets -notmatch "wasm32-unknown-unknown") {
        Write-Info "wasm32-unknown-unknown not installed — installing now..."
        Invoke-Checked @("rustup", "target", "add", "wasm32-unknown-unknown")
    }

    # Check Stellar CLI (optional — only required for optimize/deploy/initialize)
    $hasStellar = [bool](Get-Command "stellar" -ErrorAction SilentlyContinue)
    if (-not $hasStellar) {
        Write-Info "Stellar CLI not found. Required for: optimize, deploy, initialize."
        Write-Info "Download from: https://github.com/stellar/stellar-cli/releases"
    }

    Write-Success "Prerequisites OK"
}

# ── Build actions ─────────────────────────────────────────────────────────────

function Invoke-Build {
    Write-Header "Building Contract"
    Push-Location $ContractDir
    try {
        if (Get-Command "stellar" -ErrorAction SilentlyContinue) {
            Invoke-Checked @("stellar", "contract", "build")
        } else {
            Invoke-Checked @("cargo", "build", "--target", "wasm32-unknown-unknown", "--release")
        }
        Write-Success "Build complete: $WasmPath"
    } finally {
        Pop-Location
    }
}

function Invoke-Test {
    param([switch]$Verbose, [switch]$Release)
    Write-Header "Running Tests"
    Push-Location $ContractDir
    try {
        $args = @("cargo", "test", "-p", "dongle-contract")
        if ($Release) { $args += "--release" }
        if ($Verbose) { $args += "--"; $args += "--nocapture" }
        Invoke-Checked $args
        Write-Success "All tests passed"
    } finally {
        Pop-Location
    }
}

function Invoke-Check {
    Write-Header "Checking Code (no artefact)"
    Push-Location $ContractDir
    try {
        Invoke-Checked @("cargo", "check", "--target", "wasm32-unknown-unknown")
        Write-Success "Check passed"
    } finally {
        Pop-Location
    }
}

function Invoke-Fmt {
    Write-Header "Formatting Code"
    Push-Location $ContractDir
    try {
        Invoke-Checked @("cargo", "fmt", "--all")
        Write-Success "Formatting complete"
    } finally {
        Pop-Location
    }
}

function Invoke-Lint {
    Write-Header "Running Clippy (warnings → errors)"
    Push-Location $ContractDir
    try {
        Invoke-Checked @(
            "cargo", "clippy",
            "-p", "dongle-contract",
            "--target", "wasm32-unknown-unknown",
            "--all-features",
            "--", "-D", "warnings"
        )
        Write-Success "Lint passed"
    } finally {
        Pop-Location
    }
}

function Invoke-Optimize {
    Write-Header "Optimizing WASM"

    if (-not (Test-Path $WasmPath)) {
        throw "WASM binary not found at '$WasmPath'. Run build first."
    }

    $before = (Get-Item $WasmPath).Length

    if (Get-Command "stellar" -ErrorAction SilentlyContinue) {
        Write-Info "Optimizer: stellar contract optimize"
        Invoke-Checked @(
            "stellar", "contract", "optimize",
            "--wasm", $WasmPath,
            "--wasm-out", $OptWasmPath
        )
    } elseif (Get-Command "wasm-opt" -ErrorAction SilentlyContinue) {
        Write-Info "Optimizer: wasm-opt"
        Invoke-Checked @(
            "wasm-opt", "-Oz",
            "--enable-bulk-memory", "--enable-mutable-globals",
            $WasmPath, "-o", $OptWasmPath
        )
    } else {
        throw "No WASM optimizer found. Install Stellar CLI or Binaryen (wasm-opt)."
    }

    if (-not (Test-Path $OptWasmPath)) {
        throw "Optimizer did not produce '$OptWasmPath'"
    }

    $after  = (Get-Item $OptWasmPath).Length
    $saved  = $before - $after
    Write-Info "Original:  $before bytes"
    Write-Info "Optimized: $after bytes  (saved $saved bytes)"
    Write-Success "Optimized WASM written to $OptWasmPath"
}

function Invoke-Clean {
    Write-Header "Cleaning Build Artefacts"
    Push-Location $ContractDir
    try {
        Invoke-Checked @("cargo", "clean")
        Write-Success "Clean complete"
    } finally {
        Pop-Location
    }
}

# ── Stellar network actions ───────────────────────────────────────────────────

function Ensure-StellarCli {
    Assert-Command "stellar" @"
Install Stellar CLI (prebuilt, includes wasm-opt):
  VERSION=27.1.0
  Invoke-WebRequest -Uri "https://github.com/stellar/stellar-cli/releases/download/v`$VERSION/stellar-cli-`$VERSION-x86_64-pc-windows-msvc.zip" -OutFile stellar-cli.zip
  Expand-Archive stellar-cli.zip -DestinationPath "$Env:USERPROFILE\.stellar-cli"
  [System.Environment]::SetEnvironmentVariable("PATH", "$Env:PATH;$Env:USERPROFILE\.stellar-cli", "User")
See WINDOWS_BUILD_GUIDE.md §2 for full instructions.
"@
}

function Configure-Network {
    param([string]$NetworkName)

    $rpcUrl = switch ($NetworkName) {
        "testnet" { "https://soroban-testnet.stellar.org:443" }
        "local"   { "http://localhost:8000/soroban/rpc" }
        default   { $Env:RPC_URL ?? "https://soroban-testnet.stellar.org:443" }
    }
    $passphrase = switch ($NetworkName) {
        "testnet" { "Test SDF Network ; September 2015" }
        "local"   { "Standalone Network ; February 2017" }
        default   { $Env:PASSPHRASE ?? "Test SDF Network ; September 2015" }
    }

    Write-Info "Configuring network '$NetworkName' (RPC: $rpcUrl)"
    stellar network add `
        --global $NetworkName `
        --rpc-url $rpcUrl `
        --network-passphrase $passphrase 2>$null
    # Ignore "already exists" errors — the above may fail silently on re-runs
}

function Invoke-Deploy {
    Write-Header "Deploying Contract"
    Ensure-StellarCli

    if (-not $DeployerIdentity) {
        throw "DEPLOYER_IDENTITY not set. Pass -DeployerIdentity or set `$Env:DEPLOYER_IDENTITY."
    }

    Configure-Network $Network

    # Auto-generate & fund identity if it does not exist
    $idAddress = stellar keys address $DeployerIdentity 2>&1
    if ($LASTEXITCODE -ne 0) {
        Write-Info "Identity '$DeployerIdentity' not found — generating..."
        Invoke-Checked @("stellar", "keys", "generate", "--global", $DeployerIdentity, "--network", $Network)
        Write-Info "Funding account on $Network..."
        Invoke-Checked @("stellar", "keys", "fund", $DeployerIdentity, "--network", $Network)
        $idAddress = stellar keys address $DeployerIdentity
    }
    Write-Info "Deployer address: $idAddress"

    # Determine WASM to deploy (prefer optimized)
    $wasmToUse = if (Test-Path $OptWasmPath) { $OptWasmPath } else { $WasmPath }
    if (-not (Test-Path $wasmToUse)) {
        throw "WASM binary not found at '$wasmToUse'. Run build first."
    }
    Write-Info "WASM: $wasmToUse"

    $contractIdOut = stellar contract deploy `
        --wasm $wasmToUse `
        --source $DeployerIdentity `
        --network $Network

    if ($LASTEXITCODE -ne 0) {
        throw "stellar contract deploy failed"
    }

    $contractIdOut = $contractIdOut.Trim()
    Write-Info "Contract ID: $contractIdOut"
    Set-Content -Path $ContractIdFile -Value $contractIdOut -NoNewline
    Write-Info "Contract ID saved to $ContractIdFile"
    Write-Success "Contract deployed successfully"

    # Export for the current session so initialize can use it
    $script:ContractId = $contractIdOut
}

function Invoke-Initialize {
    Write-Header "Initializing Contract"
    Ensure-StellarCli

    if (-not $DeployerIdentity) {
        throw "DEPLOYER_IDENTITY not set."
    }

    # Resolve ContractId
    if (-not $ContractId) {
        if (Test-Path $ContractIdFile) {
            $script:ContractId = (Get-Content $ContractIdFile -Raw).Trim()
        } else {
            throw "CONTRACT_ID not set and $ContractIdFile not found. Deploy the contract first."
        }
    }

    # Resolve AdminAddress
    if (-not $AdminAddress) {
        $script:AdminAddress = stellar keys address $DeployerIdentity
        if ($LASTEXITCODE -ne 0) { throw "Could not resolve address for identity '$DeployerIdentity'" }
    }

    Write-Info "Contract ID:   $ContractId"
    Write-Info "Identity:      $DeployerIdentity"
    Write-Info "Admin address: $AdminAddress"
    Write-Info "Network:       $Network"

    Invoke-Checked @(
        "stellar", "contract", "invoke",
        "--id", $ContractId,
        "--source", $DeployerIdentity,
        "--network", $Network,
        "--", "initialize",
        "--admin", $AdminAddress
    )
    Write-Success "Contract initialized"
}

# ── Composite actions ─────────────────────────────────────────────────────────

function Invoke-Dev {
    Write-Header "Running Full Dev Cycle (fmt → lint → test → build)"
    Invoke-Fmt
    Invoke-Lint
    Invoke-Test
    Invoke-Build
    Write-Success "Dev cycle complete"
}

function Invoke-Ci {
    Write-Header "Running CI Cycle (check → lint → test)"
    Invoke-Check
    Invoke-Lint
    Invoke-Test
    Write-Success "CI checks passed"
}

# ── Entry point ───────────────────────────────────────────────────────────────

Load-EnvFile
Assert-Prerequisites

switch ($Action) {
    "build"        { Invoke-Build }
    "test"         { Invoke-Test }
    "test-verbose" { Invoke-Test -Verbose }
    "test-release" { Invoke-Test -Release }
    "check"        { Invoke-Check }
    "fmt"          { Invoke-Fmt }
    "lint"         { Invoke-Lint }
    "optimize"     { Invoke-Optimize }
    "clean"        { Invoke-Clean }
    "dev"          { Invoke-Dev }
    "ci"           { Invoke-Ci }
    "deploy"       { Invoke-Deploy }
    "initialize"   { Invoke-Initialize }
    "all"          {
        Invoke-Build
        Invoke-Optimize
        Invoke-Deploy
        Invoke-Initialize
    }
}
