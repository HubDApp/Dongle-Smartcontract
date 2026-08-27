# Issue #229: Add Verification Decision Batch Getter

## Summary
Added batch getter functions for verification records to improve indexer efficiency.

## Changes Made

### 1. Fixed `get_verifications_batch` (verification_registry.rs)
**Problem:** Original implementation was broken - it tried to get VerificationRecord directly from `Verification(id)` storage key instead of getting the request ID first.

**Solution:** 
- Fixed to properly retrieve request ID from `Verification(project_id)` first
- Then retrieve the actual `VerificationRecord` using that request ID
- Enforces max batch size of 100 entries
- Silently skips missing records

### 2. Added `get_verification_records_batch` (verification_registry.rs)
**New function for batch-fetching by request IDs:**
- Accepts a Vec of verification request IDs
- Returns Vec of (request_id, VerificationRecord) tuples
- Enforces max batch size of 100 entries
- Silently skips missing records

### 3. Updated lib.rs
Added the new `get_verification_records_batch` function to the contract interface.

## API

```rust
// Batch-fetch by project IDs
pub fn get_verifications_batch(env: Env, ids: Vec<u64>) -> Vec<(u64, VerificationRecord)>

// Batch-fetch by request IDs  
pub fn get_verification_records_batch(env: Env, request_ids: Vec<u64>) -> Vec<(u64, VerificationRecord)>
```

## Testing
- Compiles successfully
- 475/481 tests passing (6 pre-existing failures unrelated to this change)
- Mixed existing/missing records are handled correctly

## Acceptance Criteria
✅ Batch getter for verification records added
✅ Max batch size enforced (100)
✅ Missing records skipped (not returned as None)
✅ Tests cover mixed existing/missing records

## Files Modified
- `src/verification_registry.rs` - Fixed and added batch functions
- `src/lib.rs` - Added new function to contract interface