# Testing Guide for Dongle Smart Contract

This document provides guidance on running tests, formatting, linting, and building the Dongle Smart Contract locally.

---

## Prerequisites

Make sure you have:
- Rust installed (https://rustup.rs/)
- Cargo (comes with Rust)
- WASM target installed: `rustup target add wasm32-unknown-unknown`

---

## Complete Test Pipeline

Run these commands in order in your local environment, from the `dongle-smartcontract` directory:

### Step 1: Run Tests
```bash
cargo test --lib
```

**Expected Output**: All tests should pass ✅
- Test names appear with `test ... ok`
- Final message: `test result: ok` with count of passed tests
- No failures or errors

### Step 2: Run Cargo Clippy (Linter)
```bash
cargo clippy -p dongle-contract --target wasm32-unknown-unknown -- -D warnings
```

**Expected Output**: No warnings or errors ✅
- Completes successfully
- No `error` or `warning` messages

### Step 3: Check Formatting
```bash
cargo fmt --all -- --check
```

**Expected Output**: All files properly formatted ✅
- Should complete with no output (clean)
- Or show which files need formatting

### Step 4: Apply Formatting (if needed)
If Step 3 shows formatting issues, run:
```bash
cargo fmt --all
```

### Step 5: Build WASM Contract
```bash
cargo build -p dongle-contract --target wasm32-unknown-unknown --release
```

**Expected Output**: WASM contract built successfully ✅
- Compiles without errors
- Creates `.wasm` file in `target/wasm32-unknown-unknown/release/`

---

## Full Pipeline Script

### Bash (Linux/macOS)

Create a file `run_tests.sh`:

```bash
#!/bin/bash

set -e  # Exit on first error

cd dongle-smartcontract

echo "=== Step 1: Running Tests ==="
cargo test --lib

echo ""
echo "=== Step 2: Running Clippy ==="
cargo clippy -p dongle-contract --target wasm32-unknown-unknown -- -D warnings

echo ""
echo "=== Step 3: Checking Format ==="
cargo fmt --all -- --check

if [ $? -ne 0 ]; then
    echo ""
    echo "=== Step 3b: Applying Format ==="
    cargo fmt --all
fi

echo ""
echo "=== Step 4: Building WASM ==="
cargo build -p dongle-contract --target wasm32-unknown-unknown --release

echo ""
echo "✅ All checks passed!"
echo "WASM contract built successfully at:"
echo "target/wasm32-unknown-unknown/release/dongle_contract.wasm"
```

Then run:
```bash
bash run_tests.sh
```

### PowerShell (Windows)

Create a file `run_tests.ps1`:

```powershell
# Exit on first error
$ErrorActionPreference = "Stop"

Set-Location dongle-smartcontract

Write-Host "=== Step 1: Running Tests ===" -ForegroundColor Green
cargo test --lib
if ($LASTEXITCODE -ne 0) { exit 1 }

Write-Host "`n=== Step 2: Running Clippy ===" -ForegroundColor Green
cargo clippy -p dongle-contract --target wasm32-unknown-unknown -- -D warnings
if ($LASTEXITCODE -ne 0) { exit 1 }

Write-Host "`n=== Step 3: Checking Format ===" -ForegroundColor Green
cargo fmt --all -- --check
if ($LASTEXITCODE -ne 0) {
    Write-Host "`n=== Step 3b: Applying Format ===" -ForegroundColor Yellow
    cargo fmt --all
}

Write-Host "`n=== Step 4: Building WASM ===" -ForegroundColor Green
cargo build -p dongle-contract --target wasm32-unknown-unknown --release
if ($LASTEXITCODE -ne 0) { exit 1 }

Write-Host "`n✅ All checks passed!" -ForegroundColor Green
Write-Host "WASM contract built successfully at:" -ForegroundColor Green
Write-Host "target/wasm32-unknown-unknown/release/dongle_contract.wasm" -ForegroundColor Cyan
```

Then run:
```powershell
.\run_tests.ps1
```

---

## Common Issues and Solutions

### Issue: "cargo: command not found"
**Solution**: Install Rust from https://rustup.rs/

### Issue: "wasm32-unknown-unknown not found"
**Solution**: Run `rustup target add wasm32-unknown-unknown`

### Issue: Test failures
**Solution**: 
1. Check test output for specific errors
2. Verify all imports are present
3. Ensure all struct fields are properly initialized
4. Check that contract is properly registered in test setup

### Issue: Clippy warnings
**Solution**:
1. Review the warning message
2. Either fix the code or add `#[allow(...)]` attribute if justified
3. Re-run clippy

### Issue: Format check fails
**Solution**:
1. Run `cargo fmt --all` to auto-fix
2. Commit the formatting changes
3. Push to remote

---

## Test Structure

Tests use the Soroban SDK testing framework:

1. **Setup**: Create environment and register contract
2. **Configure**: Mock authentication and set test data
3. **Execute**: Call contract functions
4. **Verify**: Assert expected outcomes

Example test pattern:
```rust
#[test]
fn test_example() {
    let env = Env::default();
    let contract_id = env.register(DongleContract, ());
    let client = DongleContractClient::new(&env, &contract_id);
    
    env.mock_all_auths();
    
    let result = client.try_register_project(&params);
    assert!(result.is_ok());
}
```

---

## CI/CD Integration

The GitHub Actions CI pipeline runs the same checks automatically:

1. Tests pass (`cargo test --lib`)
2. Clippy passes (`cargo clippy -- -D warnings`)
3. Code is formatted (`cargo fmt -- --check`)

Check `.github/workflows/ci.yml` for the exact CI configuration.

---

## Next Steps After Successful Tests

Once all tests pass, clippy is clean, and code is formatted:

1. **Create Pull Request** (if working on a feature branch)
   - Push to remote: `git push origin [branch-name]`
   - Create PR on GitHub with detailed description

2. **Wait for CI/CD**
   - GitHub Actions will run same tests automatically
   - Verify all checks pass

3. **Code Review**
   - Request reviewers
   - Address feedback
   - Re-push if needed

4. **Merge to Main**
   - After approval and CI passes
   - Use "Squash and merge" or "Rebase and merge"

---

## Additional Resources

- [Soroban SDK Documentation](https://docs.rs/soroban-sdk/latest/)
- [Rust Book](https://doc.rust-lang.org/book/)
- [Cargo Documentation](https://doc.rust-lang.org/cargo/)
- `CONTRACT_INTERFACE.md` - Contract API reference
- `EVENTS_SCHEMA.md` - Event schema documentation

---

**Last Updated**: July 29, 2026  
**Status**: Active  
**Maintained By**: Development team
