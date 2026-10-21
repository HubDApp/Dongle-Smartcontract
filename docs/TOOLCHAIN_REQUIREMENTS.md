# Toolchain Requirements — WASM Build Compatibility (#955)

Pinned toolchain: see `rust-toolchain.toml` (currently Rust **1.85.0**,
`wasm32-unknown-unknown` target, `rustfmt` + `clippy` components).

## Supported toolchains

| Rust version | Status |
| --- | --- |
| 1.70 | Minimum supported (MSRV floor for `soroban-sdk` deps; build-only, not CI-gated) |
| 1.74 | Verified compatible |
| 1.85.0 (pinned) / latest stable | Required for reproducible builds and CI |

`rust-toolchain.toml` pins the exact channel so every checkout builds with
the same compiler. Do not override with `rustup override` unless verifying
the compatibility matrix below.

## WASM optimization levels

Release profile (`Cargo.toml` `[profile.release]`):

- `opt-level = "z"` — optimize for size (cheaper contract upload, lower ledger footprint)
- `lto = true`, `codegen-units = 1` — single codegen unit for deterministic output
- `panic = "abort"`, `strip = "symbols"` — minimal binary, no unwind tables
- `overflow-checks = true` — keep arithmetic safety in release binaries

## Verifying compatibility

Run the verification script from the repo root:

```sh
sh scripts/verify-wasm-build.sh
```

It checks:

1. The pinned toolchain is installed (`rustup toolchain list`).
2. The `wasm32-unknown-unknown` target is present.
3. `cargo check --target wasm32-unknown-unknown` passes.
4. `stellar contract build` (or `cargo build --release --target
   wasm32-unknown-unknown`) produces a deterministic-size `.wasm` artifact.
5. Rebuilding twice yields byte-identical output (binary reproducibility —
   compares `sha256sum` of consecutive builds).

## Operating systems

Builds are OS-independent: the WASM artifact only depends on the pinned
Rust compiler + profile flags above, not the host OS. CI builds on
`ubuntu-latest`; macOS/Windows checkouts reproduce the same artifact hash
provided they use the pinned toolchain.

## Documenting results

Record `rustc --version`, host OS, artifact `sha256sum`, and build time in
the PR description when touching `Cargo.toml`, `rust-toolchain.toml`, or
release profile settings.
