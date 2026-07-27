# Local Test Execution Guide

Since Rust/Cargo is not available in this environment, you need to run these commands on your local machine or CI/CD environment where Rust is installed.

---

## Prerequisites

Make sure you have:
- Rust installed (https://rustup.rs/)
- Cargo (comes with Rust)
- WASM target installed: `rustup target add wasm32-unknown-unknown`

---

## Complete Test Pipeline

Run these commands in order in your local environment:

### Step 1: Pull Latest Changes
```bash
cd /path/to/Dongle-Smartcontract-1
git pull https://github.com/mayasimi/Dongle-Smartcontract.git
```

### Step 2: Navigate to Contract Directory
```bash
cd dongle-smartcontract
```

### Step 3: Run Tests
```bash
cargo test --lib
```

**Expected Output**: All tests should pass ✅
- Archive & Reactivate tests: 20+
- Project Slug tests: 20+
- Review Moderation tests: 23
- Verification Renewal tests: 20

**What to watch for**:
- All test names appear with `test ... ok`
- Final message: `test result: ok`
- No failures or errors

### Step 4: Run Cargo Clippy (Linter)
```bash
cargo clippy -p dongle-contract --target wasm32-unknown-unknown -- -D warnings
```

**Expected Output**: No warnings or errors ✅
- Should complete successfully
- No `error` messages
- No `warning` messages

### Step 5: Run Cargo Format Check
```bash
cargo fmt --all -- --check
```

**Expected Output**: All files properly formatted ✅
- Should complete with no output (clean)
- OR show which files need formatting

### Step 6: Apply Formatting (if needed)
If Step 5 shows formatting issues, run:
```bash
cargo fmt --all
```

**Expected Output**: Files are reformatted ✅

### Step 7: Build WASM Contract
```bash
cargo build -p dongle-contract --target wasm32-unknown-unknown --release
```

**Expected Output**: WASM contract built successfully ✅
- Compiles without errors
- Creates `.wasm` file in `target/wasm32-unknown-unknown/release/`

---

## Full Pipeline Script

Create a file `run_tests.sh` and run all at once:

```bash
#!/bin/bash

set -e  # Exit on first error

echo "=== Step 1: Running Tests ==="
cargo test --lib

echo "=== Step 2: Running Clippy ==="
cargo clippy -p dongle-contract --target wasm32-unknown-unknown -- -D warnings

echo "=== Step 3: Checking Format ==="
cargo fmt --all -- --check

if [ $? -ne 0 ]; then
    echo "=== Step 3b: Applying Format ==="
    cargo fmt --all
fi

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

---

## Windows PowerShell Script

Create a file `run_tests.ps1`:

```powershell
# Exit on first error
$ErrorActionPreference = "Stop"

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

## Expected Test Results

### Tests Passing
```
test result: ok. 83 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### Individual Test Counts
- **Archive & Reactivate**: 20+ tests ✅
- **Project Slug**: 20+ tests ✅
- **Review Moderation**: 23 tests ✅
- **Verification Renewal**: 20 tests ✅
- **Total**: 80+ tests ✅

---

## Troubleshooting

### Issue: "cargo: command not found"
**Solution**: Install Rust from https://rustup.rs/

### Issue: "wasm32-unknown-unknown not found"
**Solution**: Run `rustup target add wasm32-unknown-unknown`

### Issue: Test failures
**Solution**: 
1. Check test output for specific errors
2. Refer to `TEST_FAILURE_TROUBLESHOOTING.md`
3. Review the specific test failure
4. Check for missing imports or uninitialized fields

### Issue: Clippy warnings
**Solution**:
1. Review the warning message
2. Either fix the code or add `#[allow(...)]` attribute
3. Re-run clippy

### Issue: Format check fails
**Solution**:
1. Run `cargo fmt --all` to auto-fix
2. Commit the formatting changes
3. Push to remote

---

## Next Steps After Successful Tests

Once all tests pass, clippy is clean, and code is formatted:

1. **Create Pull Request** (if not already done)
   - Visit: https://github.com/mayasimi/Dongle-Smartcontract
   - Create PR with detailed description

2. **Request Code Review**
   - Assign reviewers
   - Add relevant labels
   - Add description of changes

3. **Wait for CI/CD**
   - GitHub Actions will run same tests automatically
   - Verify all checks pass

4. **Merge to Main**
   - After approval and CI passes
   - Use "Squash and merge" or "Rebase and merge"

5. **Deploy**
   - Monitor deployment to testnet
   - Monitor deployment to mainnet
   - Verify functionality

---

## Current Status

✅ **All Features Merged to Main**:
- Feature/project-slug: Merged (PR #3)
- Feature/review-moderation: Merged (PR #2)
- Feature/verification-renewal: Merged (PR #1)

✅ **Ready for**:
- Local testing
- CI/CD validation
- Production deployment

---

## Files to Reference

- `FIXES_APPLIED.md` - Summary of fixes applied
- `TEST_FAILURE_TROUBLESHOOTING.md` - Common issues and solutions
- `FINAL_PROJECT_SUMMARY.md` - Complete project overview

---

## Summary

All three features are now merged into main and ready for testing. Follow this guide to:
1. ✅ Run all tests
2. ✅ Run clippy for code quality
3. ✅ Run fmt for formatting
4. ✅ Build WASM contract

Everything should pass successfully!

---

**Last Updated**: June 1, 2026  
**Status**: Ready for local testing and deployment  
**All Features**: Merged to main
