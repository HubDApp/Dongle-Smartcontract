//! Contract API compatibility tests (issue #257).
//!
//! These tests ensure the public contract interface does not change
//! unexpectedly. A snapshot of all exported Soroban function names is
//! stored in `contractsapi`. When the public API changes, the snapshot
//! must be regenerated and the change reviewed.
//!
//! # Regenerating the snapshot
//!
//! 1. `cargo build --target wasm32-unknown-unknown -p dongle-contract`
//! 2. `cargo run -p dongle-contract --example gen_api_snapshot` (or see
//!    the `gen_api_snapshot` helper below).
//! 3. Commit the updated `api_snapshot.txt`.
//!
//! CI will fail if the snapshot drifts from the compiled WASM exports.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

/// Path to the API snapshot file, relative to the crate root.
const SNAPSHOT_PATH: &str = "api_snapshot.txt";

/// Parse exported function names from a Soroban WASM binary.
///
/// Soroban `#[contractimpl]` exports each public method as a WASM
/// function named `__method_<name>`. We extract those names and sort
/// them for stable comparison.
fn extract_exported_functions(wasm_bytes: &[u8]) -> Vec<String> {
    let mut names = BTreeSet::new();

    // Soroban contracts export functions with the naming convention
    // `__method_<function_name>`. We scan the WASM export section.
    // Simple byte-level scan: look for export entries whose names start
    // with `__method_`.
    let marker = b"__method_";

    // Walk through the byte slice looking for the marker
    let mut i = 0;
    while i + marker.len() <= wasm_bytes.len() {
        if &wasm_bytes[i..i + marker.len()] == marker {
            // Found a marker. Extract the function name after it.
            let start = i + marker.len();
            let mut end = start;
            while end < wasm_bytes.len() && wasm_bytes[end] != 0 && wasm_bytes[end] != b'\n' {
                end += 1;
            }
            if let Ok(name) = std::str::from_utf8(&wasm_bytes[start..end]) {
                if !name.is_empty() {
                    names.insert(name.to_string());
                }
            }
            i = end;
        } else {
            i += 1;
        }
    }

    names.into_iter().collect()
}

/// Build the contract WASM and return the bytes.
fn build_contract_wasm() -> Vec<u8> {
    // Try to find a pre-built WASM in the target directory first.
    let target_dirs = [
        "target/wasm32-unknown-unknown/release",
        "../target/wasm32-unknown-unknown/release",
    ];

    for dir in &target_dirs {
        let wasm_path = PathBuf::from(dir).join("dongle_contract.wasm");
        if wasm_path.exists() {
            return fs::read(&wasm_path)
                .unwrap_or_else(|e| panic!("Failed to read WASM at {:?}: {}", wasm_path, e));
        }
    }

    // If no pre-built WASM found, build it.
    let output = std::process::Command::new("cargo")
        .args([
            "build",
            "--target",
            "wasm32-unknown-unknown",
            "-p",
            "dongle-contract",
            "--release",
        ])
        .current_dir("../")
        .output()
        .expect("Failed to run cargo build");

    if !output.status.success() {
        panic!(
            "cargo build failed:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let wasm_path = PathBuf::from("../target/wasm32-unknown-unknown/release/dongle_contract.wasm");
    fs::read(&wasm_path)
        .unwrap_or_else(|e| panic!("Failed to read built WASM at {:?}: {}", wasm_path, e))
}

/// Generate the snapshot content from WASM bytes.
fn generate_snapshot(wasm_bytes: &[u8]) -> String {
    let mut functions = extract_exported_functions(wasm_bytes);
    functions.sort();
    let mut snapshot = String::from(
        "# Contract API snapshot — auto-generated. Do not edit manually.\n\
         # Regenerate: see tests/api_compat.rs for instructions.\n\n",
    );
    for name in &functions {
        snapshot.push_str(name);
        snapshot.push('\n');
    }
    snapshot
}

#[test]
fn test_api_snapshot_matches() {
    let wasm_bytes = build_contract_wasm();
    let current = generate_snapshot(&wasm_bytes);

    let snapshot_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(SNAPSHOT_PATH);

    if snapshot_path.exists() {
        let expected = fs::read_to_string(&snapshot_path)
            .unwrap_or_else(|e| panic!("Failed to read snapshot at {:?}: {}", snapshot_path, e));
        assert_eq!(
            current, expected,
            "\n\nContract API has changed!\n\
             If this is intentional, regenerate the snapshot:\n\
             1. cargo build --target wasm32-unknown-unknown -p dongle-contract\n\
             2. Update {}\n\
             3. Commit the updated snapshot\n",
            snapshot_path.display()
        );
    } else {
        // First run — write the snapshot.
        fs::write(&snapshot_path, &current)
            .unwrap_or_else(|e| panic!("Failed to write snapshot at {:?}: {}", snapshot_path, e));
        eprintln!(
            "API snapshot created at {}. Commit this file.",
            snapshot_path.display()
        );
    }
}

#[test]
fn test_api_snapshot_is_sorted() {
    let snapshot_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(SNAPSHOT_PATH);
    if !snapshot_path.exists() {
        return; // Skip if snapshot doesn't exist yet.
    }

    let content = fs::read_to_string(&snapshot_path).unwrap();
    let lines: Vec<&str> = content
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .collect();

    for window in lines.windows(2) {
        assert!(
            window[0] <= window[1],
            "API snapshot is not sorted. `{}` should come before or equal to `{}`.\n\
             Regenerate the snapshot to fix ordering.",
            window[0],
            window[1]
        );
    }
}

/// Allow generating the snapshot from a test runner.
///
/// Run with: `cargo test test_generate_api_snapshot -- --ignored`
#[test]
#[ignore]
fn test_generate_api_snapshot() {
    let wasm_bytes = build_contract_wasm();
    let snapshot = generate_snapshot(&wasm_bytes);

    let snapshot_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(SNAPSHOT_PATH);
    fs::write(&snapshot_path, &snapshot).expect("Failed to write snapshot");
    eprintln!("Snapshot written to {:?}", snapshot_path);
}
