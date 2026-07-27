# Verification Renewal

Verification renewal lets the owner of a verified project submit updated
evidence and lets a contract administrator approve or reject that request. An
approved request extends the verification period and becomes part of the
project's renewal history.

This guide reflects the current implementation under
`dongle-smartcontract/src`. The removed root-level summaries described older
branches and contained error codes and behavior that no longer match `main`.

## Public API

| Method | Caller | Result |
|--------|--------|--------|
| `request_renewal(project_id, requester, evidence_cid)` | Project owner | Creates one pending renewal request and consumes the configured verification fee when required |
| `approve_renewal(project_id, admin)` | Contract administrator | Extends verification, records approved history, and clears the pending request |
| `reject_renewal(project_id, admin)` | Contract administrator | Clears the pending request so the owner can submit another |
| `get_renewal_request(project_id)` | Public | Returns the current pending request |
| `get_renewal_history(project_id, start_index, limit)` | Public | Returns approved renewal records with pagination |
| `is_verification_expired(project_id)` | Public | Reports whether a nonzero expiry is earlier than the current ledger time |

The entrypoints are defined in
[`lib.rs`](../dongle-smartcontract/src/lib.rs), and the workflow is implemented
by
[`VerificationRegistry`](../dongle-smartcontract/src/verification_registry.rs).

## Renewal Record

Pending and historical renewals use the same data type:

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

The current pending record has `Pending` status. Approval copies it into
history with `Verified` status and the final expiry timestamp.

## Requesting Renewal

`request_renewal` performs these steps:

1. loads the project and requires authorization from its owner;
2. requires the project status to be `Verified`;
3. rejects a second request while one is already pending;
4. validates the evidence CID;
5. consumes the configured verification fee when the fee is greater than zero;
6. creates a pending record with a proposed expiry; and
7. emits `VerificationRenewalReqEvent`.

Renewal can be requested before or after the current verification expires. The
project must still have `Verified` status.

```rust
client.request_renewal(&project_id, &owner, &evidence_cid);
let pending = client.get_renewal_request(&project_id);
assert_eq!(pending.status, VerificationStatus::Pending);
```

The default validity period is 365 days. The registry reads the configured
verification duration first and falls back to
`VERIFICATION_VALIDITY_PERIOD` when none has been set.

## Approving Renewal

`approve_renewal` requires an authenticated contract administrator. It:

1. loads the pending request, verification record, and project;
2. calculates a new expiry from the current ledger time;
3. keeps the verification status as `Verified` and updates `expires_at` and
   `last_renewed_at`;
4. updates the project's timestamp and current verification reference;
5. appends an approved record to renewal history;
6. increments the history count and removes the pending request;
7. emits `VerificationRenewalApprovedEvent`; and
8. records `AdminActionType::VerificationRenewalApproved`.

```rust
client.approve_renewal(&project_id, &admin);
let history = client.get_renewal_history(&project_id, &0, &100);
assert_eq!(history.len(), 1);
```

## Rejecting Renewal

`reject_renewal` also requires an authenticated administrator. It verifies that
a pending request exists, removes it, emits
`VerificationRenewalRejectedEvent`, and records the administrator action.

Rejected requests are not added to `VerificationRenewalHistory` by the current
implementation. The owner can submit a new request afterward.

## Storage

Renewal data uses three persistent keys:

```rust
StorageKey::VerificationRenewal(project_id)
StorageKey::VerificationRenewalHistory(project_id, history_index)
StorageKey::VerificationRenewalCount(project_id)
```

The first key stores the single pending request. The indexed keys store approved
records, and the count supplies the next index and pagination boundary. These
keys are defined in
[`storage_keys.rs`](../dongle-smartcontract/src/storage_keys.rs).

## Events

| Event | Trigger | Key fields |
|-------|---------|------------|
| `VerificationRenewalReqEvent` | Owner submits a request | project, requester, evidence CID, fee, timestamp |
| `VerificationRenewalApprovedEvent` | Admin approves | project, admin, new expiry, timestamp |
| `VerificationRenewalRejectedEvent` | Admin rejects | project, admin, timestamp |

Definitions and publishers live in
[`events.rs`](../dongle-smartcontract/src/events.rs).

## Pagination and Expiry

`get_renewal_history` accepts a zero-based start index and a limit. A limit of
zero or a value above `MAX_PAGE_LIMIT` uses the maximum of 100. A start index at
or beyond the stored count returns an empty vector.

`is_verification_expired` returns:

- `false` when `expires_at` is zero, which represents no expiry;
- `false` while the current ledger timestamp is at or before the expiry; and
- `true` only after the expiry timestamp has passed.

## Errors

The current implementation deliberately reuses the contract's existing errors:

| Error | Renewal scenario |
|-------|------------------|
| `ProjectNotFound` | The project does not exist |
| `Unauthorized` | The requester is not the project owner |
| `AdminOnly` | The approval or rejection caller is not an administrator |
| `InvalidStatus` | The project is not verified or another renewal is pending |
| `InvalidProjectData` | The evidence CID is empty, malformed, or too long |
| `VerificationNotFound` | Verification data or a pending request is missing |
| Fee errors | A required fee payment is missing, invalid, or expired |

The old documents assigned codes 42 through 45 to renewal-specific variants,
but those codes conflict with the current `ContractError` enum and the variants
do not exist on `main`. Consumers should use the canonical error reference and
generated contract interface rather than the removed numbers.

## Test Coverage

The focused suite in
[`tests/renewal.rs`](../dongle-smartcontract/src/tests/renewal.rs) contains 21
tests covering:

- successful owner requests;
- unverified, duplicate, and non-owner failures;
- administrator approval and rejection authorization;
- expiry and `last_renewed_at` updates;
- approved-history ordering and pagination;
- rejection followed by a new request;
- project isolation and status preservation; and
- requests made after expiry.

Cleanup and verification-feature suites exercise additional renewal paths. Run
the contract tests from the workspace root with:

```bash
cargo test -p dongle-contract renewal
```

## Operational Notes

- A pending request does not change the project's current verification status.
- Approval starts the new validity period at approval time, not request time.
- Rejection removes the request and does not retain an on-chain rejected record;
  indexers should retain the rejection event if that history is required.
- Fee configuration can make a request free or require a previously recorded
  verification-fee payment.
- Expiry checks are read-only. They do not automatically change project status.

## Documentation Cleanup Note

The removed `VERIFICATION_CHECKLIST.md` was misnamed: its contents described the
project archive/reactivate feature rather than verification renewal. That
material was not copied here. Archive behavior belongs in the canonical archive
feature guide tracked separately by issue #370.
