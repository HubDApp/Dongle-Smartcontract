# Test Failure Troubleshooting Guide

## Issue Summary

The CI/CD pipeline is failing on one or more of the feature branches. This guide helps identify and fix the issues.

---

## Common Issues and Solutions

### 1. ✅ FIXED: VerificationRecord Missing Fields

**Issue**: VerificationRecord struct was updated with new fields (`expires_at`, `last_renewed_at`) but not all initialization sites were updated.

**Solution**: Added initialization of new fields in `verification_registry.rs` line 180-191:
```rust
let record = VerificationRecord {
    project_id,
    requester: requester.clone(),
    status: VerificationStatus::Pending,
    evidence_cid: evidence_cid.clone(),
    timestamp: now,
    fee_amount: config.verification_fee,
    revoke_reason: None,
    expires_at: 0,           // ← ADDED
    last_renewed_at: 0,      // ← ADDED
};
```

**Status**: ✅ FIXED in commit 92aec15

---

### 2. Formatting Issues

**Issue**: `cargo fmt` detects formatting differences.

**Solution**: 
- Run `cargo fmt` locally to auto-fix formatting
- Check for trailing whitespace
- Ensure consistent spacing around operators

**Files to Check**:
- src/tests/indexer.rs
- src/tests/verification.rs
- src/tests/fee.rs

---

### 3. Clippy Warnings

**Issue**: `cargo clippy` detects potential code improvements.

**Common Issues**:
- Unused variables
- Unnecessary clones
- Inefficient patterns

**Solution**:
- Review clippy warnings
- Fix issues or add `#[allow(...)]` attributes if intentional

---

### 4. Test Failures

**Issue**: Unit tests or integration tests failing.

**Common Causes**:
- Missing imports
- Incorrect error types
- Uninitialized fields
- Logic errors

**Solution**:
- Check test output for specific error messages
- Verify all new methods are properly implemented
- Ensure all error types are defined

---

## Verification Checklist

### For Each Feature Branch

- [ ] All new structs have all fields initialized
- [ ] All new methods are properly implemented
- [ ] All imports are correct
- [ ] All error types are defined
- [ ] All tests compile and run
- [ ] Code is properly formatted (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] All acceptance criteria are met

---

## Branch-Specific Checks

### feature/project-slug
- [ ] ProjectBySlug storage key is used correctly
- [ ] Slug validation is working
- [ ] get_project_by_slug() returns correct results
- [ ] Duplicate slug detection works
- [ ] Test fixtures include slug parameter

### feature/review-moderation
- [ ] Review struct has hidden and report_count fields
- [ ] list_reviews() excludes hidden reviews
- [ ] Stats are recalculated when reviews are hidden/restored
- [ ] ReviewReport storage key tracks duplicates
- [ ] Admin-only access control is enforced

### feature/verification-renewal
- [ ] VerificationRecord has expires_at and last_renewed_at fields ✅ FIXED
- [ ] VerificationRenewalRecord is properly defined
- [ ] Renewal history is tracked correctly
- [ ] Expiry checking works
- [ ] Admin-only access control is enforced

---

## How to Debug Locally

### 1. Check Formatting
```bash
cd dongle-smartcontract
cargo fmt --all -- --check
```

### 2. Run Clippy
```bash
cargo clippy -p dongle-contract --target wasm32-unknown-unknown -- -D warnings
```

### 3. Run Tests
```bash
cargo test -p dongle-contract
```

### 4. Build WASM
```bash
cargo build -p dongle-contract --target wasm32-unknown-unknown --release
```

---

## Recent Fixes Applied

### Commit 92aec15
**Fix**: Initialize new VerificationRecord fields (expires_at, last_renewed_at)

**Changes**:
- Added `expires_at: 0` to VerificationRecord initialization
- Added `last_renewed_at: 0` to VerificationRecord initialization

**Branch**: feature/verification-renewal

---

## Next Steps

1. **Check CI/CD Logs**: Look at the GitHub Actions workflow logs to see specific error messages
2. **Review Error Messages**: The CI output will show exactly which tests are failing
3. **Apply Fixes**: Use this guide to identify and fix issues
4. **Re-run Tests**: Push fixes and verify CI passes
5. **Create PRs**: Once all tests pass, create pull requests

---

## CI/CD Pipeline

The CI runs these checks in order:

1. **Formatting** (`cargo fmt --all -- --check`)
   - Checks code formatting
   - Must pass before other checks

2. **Linting** (`cargo clippy`)
   - Checks for code quality issues
   - Must pass before tests

3. **Tests** (`cargo test`)
   - Runs all unit and integration tests
   - Must pass before build

4. **Build** (`cargo build --target wasm32-unknown-unknown --release`)
   - Builds WASM contract
   - Only runs if all previous checks pass

---

## Common Error Messages

### "error: could not compile `dongle-contract`"
- Check for missing fields in struct initialization
- Verify all imports are correct
- Check for typos in method names

### "error: field `X` is never read"
- Add `#[allow(dead_code)]` if intentional
- Or remove unused field

### "error: this value is used after being moved"
- Add `.clone()` where needed
- Or restructure code to avoid move

### "test failed"
- Check test output for specific assertion failures
- Verify test setup is correct
- Check for missing mock_all_auths()

---

## Support

If you encounter issues not covered here:

1. Check the GitHub Actions logs for specific error messages
2. Review the test output carefully
3. Compare with working code in other branches
4. Check for recent changes that might have broken things

---

## Status

**Last Updated**: June 1, 2026  
**Latest Fix**: Commit 92aec15 - VerificationRecord field initialization  
**All Branches**: Ready for testing and PR creation
