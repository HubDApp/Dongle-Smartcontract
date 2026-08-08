# Pull Request: Verification Renewal Feature

## Title
feat: implement verification renewal feature

## Description

This PR implements the Verification Renewal feature, enabling project owners to renew their verification before or after expiry, and allowing administrators to approve or reject renewal requests.

## Changes

### Core Implementation
- Add `request_renewal()` method for owners to request renewal
- Add `approve_renewal()` method for admins to approve renewals
- Add `reject_renewal()` method for admins to reject renewals
- Add `get_renewal_request()` to retrieve current renewal request
- Add `get_renewal_history()` to retrieve renewal history with pagination
- Add `is_verification_expired()` to check verification expiry status

### Data Model
- Added `expires_at: u64` field to VerificationRecord
- Added `last_renewed_at: u64` field to VerificationRecord
- Added VerificationRenewalRecord struct for tracking renewals

### Storage
- Added VerificationRenewal storage key for current renewal requests
- Added VerificationRenewalHistory storage key for historical renewals
- Added VerificationRenewalCount storage key for renewal count tracking

### Error Handling
- VerificationRenewalNotFound (42) - Renewal request not found
- VerificationRenewalAlreadyPending (43) - Renewal already pending
- CannotRenewUnverified (44) - Cannot renew unverified project
- VerificationNotExpired (45) - Verification has not expired yet

### Events
- VerificationRenewalRequestedEvent - Emitted when renewal is requested
- VerificationRenewalApprovedEvent - Emitted when renewal is approved
- VerificationRenewalRejectedEvent - Emitted when renewal is rejected

### Constants
- VERIFICATION_VALIDITY_PERIOD = 365 days

## Test Coverage

Comprehensive test suite with 20+ test cases:

### Request Renewal Tests (4 tests)
- ✅ test_request_renewal_success
- ✅ test_request_renewal_unverified_fails
- ✅ test_request_renewal_duplicate_fails
- ✅ test_request_renewal_not_owner_fails

### Approve Renewal Tests (4 tests)
- ✅ test_approve_renewal_success
- ✅ test_approve_renewal_sets_expiry
- ✅ test_approve_renewal_non_admin_fails
- ✅ test_approve_renewal_not_found_fails

### Reject Renewal Tests (3 tests)
- ✅ test_reject_renewal_success
- ✅ test_reject_renewal_non_admin_fails
- ✅ test_reject_renewal_not_found_fails

### Renewal History Tests (3 tests)
- ✅ test_renewal_history_single
- ✅ test_renewal_history_multiple
- ✅ test_renewal_history_pagination

### Expiry Checking Tests (2 tests)
- ✅ test_is_verification_expired_not_expired
- ✅ test_is_verification_expired_no_expiry

### Complex Scenario Tests (4 tests)
- ✅ test_renewal_after_rejection
- ✅ test_multiple_projects_independent_renewal
- ✅ test_renewal_preserves_verification_status
- ✅ test_renewal_updates_last_renewed_at

## Acceptance Criteria

✅ **Verified projects can request renewal before or after expiry**
- request_renewal() method implemented
- Works for verified projects
- Can be called before or after expiry

✅ **Renewal uses a separate state or record history**
- VerificationRenewalRecord struct created
- Separate storage keys for renewal requests and history
- Renewal history tracked with indices

✅ **Admin approval extends verification validity**
- approve_renewal() method implemented (admin-only)
- Sets expires_at to current_time + VERIFICATION_VALIDITY_PERIOD
- Updates last_renewed_at timestamp

✅ **Tests cover renewal request, approval, rejection, and invalid transitions**
- 20+ comprehensive test cases
- All scenarios covered
- Edge cases handled
- Error conditions tested

## Files Changed

1. **src/types.rs**
   - Added expires_at and last_renewed_at to VerificationRecord
   - Added VerificationRenewalRecord struct

2. **src/errors.rs**
   - Added VerificationRenewalNotFound (42)
   - Added VerificationRenewalAlreadyPending (43)
   - Added CannotRenewUnverified (44)
   - Added VerificationNotExpired (45)

3. **src/events.rs**
   - Added VerificationRenewalRequestedEvent
   - Added VerificationRenewalApprovedEvent
   - Added VerificationRenewalRejectedEvent
   - Added publish functions for renewal events

4. **src/storage_keys.rs**
   - Added VerificationRenewal(u64)
   - Added VerificationRenewalHistory(u64, u32)
   - Added VerificationRenewalCount(u64)

5. **src/constants.rs**
   - Added VERIFICATION_VALIDITY_PERIOD constant

6. **src/verification_registry.rs**
   - Added request_renewal() method
   - Added approve_renewal() method
   - Added reject_renewal() method
   - Added get_renewal_request() method
   - Added get_renewal_history() method
   - Added is_verification_expired() method

7. **src/lib.rs**
   - Added request_renewal() contract method
   - Added approve_renewal() contract method
   - Added reject_renewal() contract method
   - Added get_renewal_request() contract method
   - Added get_renewal_history() contract method
   - Added is_verification_expired() contract method

8. **src/tests/renewal.rs** (NEW)
   - Comprehensive test suite with 20+ test cases

9. **src/tests/mod.rs**
   - Registered renewal test module

10. **VERIFICATION_RENEWAL_FEATURE.md** (NEW)
    - Complete feature documentation

## Key Design Decisions

1. **Separate renewal records**: Renewal requests stored separately from main verification for clean state management.

2. **Renewal history tracking**: All approved renewals stored in history with indices for audit trails.

3. **Expiry timestamp**: Verification records include expiry timestamp for time-based checks.

4. **Fee consumption**: Renewal requests consume fees like initial verification.

5. **Owner-initiated renewal**: Only project owners can request renewal.

6. **Admin approval required**: Admins must approve renewals for quality control.

7. **Rejection allows retry**: Rejected renewals can be requested again.

8. **Verification status preserved**: Renewal doesn't change status, only extends validity.

## Integration Points

- **Admin Manager**: Verifies admin status
- **Project Registry**: Validates project existence and ownership
- **Fee Manager**: Consumes fees for renewal requests
- **Storage Manager**: Extends TTL for renewal data
- **Events**: Publishes renewal events for indexing

## Branch Information

- **Branch**: feature/verification-renewal
- **Base**: main
- **Commit**: cefdb89
- **Files Changed**: 10
- **Insertions**: 1223
- **Deletions**: 0

## How to Test

1. Checkout the feature branch:
   ```bash
   git checkout feature/verification-renewal
   ```

2. Run the test suite:
   ```bash
   cargo test --lib renewal
   ```

3. Run all tests:
   ```bash
   cargo test
   ```

## Deployment Notes

- No database migrations required
- No breaking changes to existing APIs
- Backward compatible with existing verifications
- New fields default to safe values (expires_at=0, last_renewed_at=0)
- Renewal history starts empty for existing projects

## Future Enhancements

1. Auto-renewal before expiry
2. Renewal reminders for owners
3. Bulk renewal for multiple projects
4. Renewal analytics and tracking
5. Conditional renewal with additional evidence
6. Different fees for renewal vs initial verification
