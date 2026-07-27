# Verification Renewal Feature

## Overview

The Verification Renewal feature enables project owners to renew their verification before or after expiry, and allows administrators to approve or reject renewal requests. This ensures verified projects can maintain their status with updated evidence.

## Acceptance Criteria

✅ Verified projects can request renewal before or after expiry  
✅ Renewal uses a separate state or record history  
✅ Admin approval extends verification validity  
✅ Tests cover renewal request, approval, rejection, and invalid transitions  

## Implementation Details

### Data Model Changes

#### VerificationRecord Struct (src/types.rs)
Added two new fields:
- `expires_at: u64` - Unix timestamp when verification expires (0 = no expiry)
- `last_renewed_at: u64` - Unix timestamp when verification was last renewed

#### VerificationRenewalRecord Struct (src/types.rs)
New struct for tracking renewal requests:
```rust
pub struct VerificationRenewalRecord {
    pub project_id: u64,
    pub requester: Address,
    pub status: VerificationStatus,
    pub evidence_cid: String,
    pub timestamp: u64,
    pub fee_amount: u128,
    pub expires_at: u64,
}
```

### Storage Keys (src/storage_keys.rs)

Added three new storage keys:
- `VerificationRenewal(u64)` - Current renewal request for a project
- `VerificationRenewalHistory(u64, u32)` - Historical renewal records (project_id, renewal_index)
- `VerificationRenewalCount(u64)` - Number of renewals for a project

### Error Types (src/errors.rs)

Added four new error types:
- `VerificationRenewalNotFound` (42) - Renewal request not found
- `VerificationRenewalAlreadyPending` (43) - Renewal already pending
- `CannotRenewUnverified` (44) - Cannot renew unverified project
- `VerificationNotExpired` (45) - Verification has not expired yet

### Events (src/events.rs)

Added three new event types:
- `VerificationRenewalRequestedEvent` - Emitted when renewal is requested
- `VerificationRenewalApprovedEvent` - Emitted when renewal is approved
- `VerificationRenewalRejectedEvent` - Emitted when renewal is rejected

### Constants (src/constants.rs)

Added verification validity period:
- `VERIFICATION_VALIDITY_PERIOD: u64 = 365 * 24 * 60 * 60` - 365 days in seconds

### Core Implementation (src/verification_registry.rs)

#### 1. request_renewal()
```rust
pub fn request_renewal(
    env: &Env,
    project_id: u64,
    requester: Address,
    evidence_cid: String,
) -> Result<(), ContractError>
```

**Behavior:**
- Requires project owner authentication
- Validates project exists
- Validates project is verified
- Prevents duplicate renewal requests
- Validates evidence CID
- Consumes fee payment
- Creates renewal record
- Emits VerificationRenewalRequestedEvent

**Errors:**
- `ProjectNotFound` - If project doesn't exist
- `Unauthorized` - If caller is not project owner
- `CannotRenewUnverified` - If project is not verified
- `VerificationRenewalAlreadyPending` - If renewal already pending
- `InvalidProjectData` - If evidence CID is invalid

#### 2. approve_renewal()
```rust
pub fn approve_renewal(
    env: &Env,
    project_id: u64,
    admin: Address,
) -> Result<(), ContractError>
```

**Behavior:**
- Requires admin authentication
- Validates project exists
- Validates project is verified
- Validates renewal request exists
- Sets expiry to current_time + VERIFICATION_VALIDITY_PERIOD
- Updates main verification record with new expiry
- Stores renewal in history
- Increments renewal count
- Removes renewal request
- Emits VerificationRenewalApprovedEvent

**Errors:**
- `AdminOnly` - If caller is not an admin
- `ProjectNotFound` - If project doesn't exist
- `CannotRenewUnverified` - If project is not verified
- `VerificationRenewalNotFound` - If renewal request doesn't exist

#### 3. reject_renewal()
```rust
pub fn reject_renewal(
    env: &Env,
    project_id: u64,
    admin: Address,
) -> Result<(), ContractError>
```

**Behavior:**
- Requires admin authentication
- Validates project exists
- Validates renewal request exists
- Removes renewal request
- Emits VerificationRenewalRejectedEvent

**Errors:**
- `AdminOnly` - If caller is not an admin
- `ProjectNotFound` - If project doesn't exist
- `VerificationRenewalNotFound` - If renewal request doesn't exist

#### 4. get_renewal_request()
```rust
pub fn get_renewal_request(
    env: &Env,
    project_id: u64,
) -> Result<VerificationRenewalRecord, ContractError>
```

**Behavior:**
- Retrieves current renewal request for a project
- Returns error if no renewal pending

**Errors:**
- `VerificationRenewalNotFound` - If no renewal request exists

#### 5. get_renewal_history()
```rust
pub fn get_renewal_history(
    env: &Env,
    project_id: u64,
    start_index: u32,
    limit: u32,
) -> Vec<VerificationRenewalRecord>
```

**Behavior:**
- Retrieves historical renewal records with pagination
- Clamped to MAX_PAGE_LIMIT (100) entries
- Returns empty vector if start_index >= total renewals

#### 6. is_verification_expired()
```rust
pub fn is_verification_expired(
    env: &Env,
    project_id: u64,
) -> Result<bool, ContractError>
```

**Behavior:**
- Checks if verification has expired
- Returns false if expires_at == 0 (no expiry set)
- Returns true if current_time > expires_at

**Errors:**
- `VerificationNotFound` - If verification doesn't exist

### API Changes (src/lib.rs)

Added six new contract methods:
- `request_renewal(project_id, requester, evidence_cid) -> Result<(), ContractError>`
- `approve_renewal(project_id, admin) -> Result<(), ContractError>`
- `reject_renewal(project_id, admin) -> Result<(), ContractError>`
- `get_renewal_request(project_id) -> Result<VerificationRenewalRecord, ContractError>`
- `get_renewal_history(project_id, start_index, limit) -> Vec<VerificationRenewalRecord>`
- `is_verification_expired(project_id) -> Result<bool, ContractError>`

## Test Coverage

Comprehensive test suite in `src/tests/renewal.rs` with 20+ test cases:

### Request Renewal Tests
- ✅ `test_request_renewal_success` - Basic renewal request
- ✅ `test_request_renewal_unverified_fails` - Cannot renew unverified project
- ✅ `test_request_renewal_duplicate_fails` - Duplicate renewal prevention
- ✅ `test_request_renewal_not_owner_fails` - Only owner can request

### Approve Renewal Tests
- ✅ `test_approve_renewal_success` - Basic approval
- ✅ `test_approve_renewal_sets_expiry` - Expiry is set correctly
- ✅ `test_approve_renewal_non_admin_fails` - Only admin can approve
- ✅ `test_approve_renewal_not_found_fails` - Cannot approve non-existent renewal

### Reject Renewal Tests
- ✅ `test_reject_renewal_success` - Basic rejection
- ✅ `test_reject_renewal_non_admin_fails` - Only admin can reject
- ✅ `test_reject_renewal_not_found_fails` - Cannot reject non-existent renewal

### Renewal History Tests
- ✅ `test_renewal_history_single` - Single renewal in history
- ✅ `test_renewal_history_multiple` - Multiple renewals in history
- ✅ `test_renewal_history_pagination` - Pagination works correctly

### Expiry Checking Tests
- ✅ `test_is_verification_expired_not_expired` - Not expired check
- ✅ `test_is_verification_expired_no_expiry` - No expiry set

### Complex Scenario Tests
- ✅ `test_renewal_after_rejection` - Can renew after rejection
- ✅ `test_multiple_projects_independent_renewal` - Independent renewal per project
- ✅ `test_renewal_preserves_verification_status` - Status remains verified
- ✅ `test_renewal_updates_last_renewed_at` - Timestamp updated

## Usage Examples

### Request Renewal
```rust
client.request_renewal(
    &project_id,
    &owner_address,
    &new_evidence_cid
)?;
```

### Approve Renewal
```rust
client.approve_renewal(&project_id, &admin_address)?;
```

### Reject Renewal
```rust
client.reject_renewal(&project_id, &admin_address)?;
```

### Check Renewal Status
```rust
let renewal = client.get_renewal_request(&project_id)?;
println!("Renewal status: {:?}", renewal.status);
```

### Get Renewal History
```rust
let history = client.get_renewal_history(&project_id, &0, &100);
for renewal in history.iter() {
    println!("Renewal at: {}", renewal.timestamp);
}
```

### Check Expiry
```rust
let is_expired = client.is_verification_expired(&project_id)?;
if is_expired {
    println!("Verification has expired, renewal needed");
}
```

## Key Design Decisions

1. **Separate renewal records**: Renewal requests are stored separately from main verification, allowing for clean state management.

2. **Renewal history tracking**: All approved renewals are stored in history with indices, enabling audit trails and analytics.

3. **Expiry timestamp**: Verification records now include expiry timestamp, allowing for time-based checks.

4. **Fee consumption**: Renewal requests consume fees like initial verification, ensuring consistent monetization.

5. **Owner-initiated renewal**: Only project owners can request renewal, maintaining ownership control.

6. **Admin approval required**: Admins must approve renewals, ensuring quality control.

7. **Rejection allows retry**: Rejected renewals can be requested again, providing flexibility.

8. **Verification status preserved**: Renewal doesn't change verification status, only extends validity.

## Integration Points

- **Admin Manager**: Uses `AdminManager::is_admin()` for access control
- **Project Registry**: Validates project existence and ownership
- **Fee Manager**: Consumes fees for renewal requests
- **Storage Manager**: Extends TTL for renewal data
- **Events**: Publishes renewal events for indexing

## State Transitions

```
Verified Project
    ↓
Request Renewal (creates VerificationRenewal record)
    ↓
    ├─→ Approve Renewal → Update expires_at, store in history, remove request
    │
    └─→ Reject Renewal → Remove request (can request again)
```

## Future Enhancements

1. **Auto-renewal**: Automatically renew verification before expiry
2. **Renewal reminders**: Notify owners when renewal is approaching
3. **Bulk renewal**: Renew multiple projects at once
4. **Renewal analytics**: Track renewal patterns and success rates
5. **Conditional renewal**: Require additional evidence for renewal
6. **Renewal fees**: Different fees for renewal vs initial verification

## Files Modified

- `src/types.rs` - Added renewal record types
- `src/errors.rs` - Added renewal error types
- `src/events.rs` - Added renewal event types
- `src/storage_keys.rs` - Added renewal storage keys
- `src/constants.rs` - Added verification validity period
- `src/verification_registry.rs` - Implemented renewal methods
- `src/lib.rs` - Exposed renewal methods
- `src/tests/mod.rs` - Registered renewal test module
- `src/tests/renewal.rs` - Comprehensive test suite

## Deployment Notes

- No database migrations required
- New fields default to safe values (expires_at=0, last_renewed_at=0)
- Backward compatible with existing verifications
- Renewal history starts empty for existing projects
