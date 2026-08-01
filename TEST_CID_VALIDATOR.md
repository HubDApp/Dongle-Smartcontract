# CID Validator Test Cases

## Test Plan for Consolidated CID Validator

The new consolidated CID validator should handle both CIDv0 and CIDv1 formats consistently.

### CIDv0 Test Cases
- ✅ `Qm...` (exactly 46 chars, base58btc characters only)
- ❌ `Qm...` (45 chars) - too short
- ❌ `Qm...` (47 chars) - too long  
- ❌ `qm...` (46 chars) - lowercase Q
- ❌ `Qn...` (46 chars) - wrong second character
- ❌ `Qm...` with invalid characters (non-base58btc)

### CIDv1 Test Cases
- ✅ `b...` (40-128 chars, alphanumeric/dash/underscore)
- ✅ `bafy...` (common prefix)
- ✅ `bafk...` (common prefix)
- ✅ `ba...` (common prefix)
- ❌ `b...` (39 chars) - too short
- ❌ `b...` (129 chars) - too long
- ❌ `c...` (valid length) - doesn't start with 'b'
- ❌ `b...` with invalid characters (non-alphanumeric, not dash/underscore)

### Edge Cases
- Empty string - invalid
- Very long string (>128 chars) - invalid
- Mixed case - CIDv0 must be uppercase Q, CIDv1 lowercase b
- Special characters - only dash and underscore allowed for CIDv1

## Testing the Changes

To test the changes, we should:
1. Run existing CID validation tests
2. Add new test cases for edge cases
3. Verify both bounty CIDs and logo/metadata CIDs use the same validator
4. Ensure timelock functions return proper errors instead of panics
5. Test bookmark and endorsement functions with new error types