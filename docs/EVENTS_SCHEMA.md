# Dongle Smart Contract Event Schema Reference

This document defines the schema of all events emitted by the Dongle smart contract. Indexers and off-chain clients can use this reference to correctly parse and process contract events.

---

## Compatibility Expectations

- **Backward Compatibility:** All existing fields in event structures are guaranteed to remain unchanged in name and type.
- **Forward Compatibility:** Events are serialized using Soroban contract type structures (which compile to XDR structures). Indexers should be implemented to:
  - Ignore any unrecognized fields appended to the end of events.
  - Handle extension fields gracefully without failing.
- **Topics:** Event topics are always a tuple of symbols and identifiers. The first element of the topic is always the event category.

---

## 1. Project Events

All project-related events start with the topic `PROJECT` (Symbol).

### Project Registration
* **Topic:** `(Symbol("PROJECT"), Symbol("CREATED"), project_id: u64)`
* **Payload (`ProjectRegisteredEvent`):**
  * `project_id` (`u64`): Unique, monotonically increasing ID of the project.
  * `owner` (`Address`): Account address of the project owner.
  * `name` (`String`): The name of the registered project.
  * `category` (`String`): The category under which the project is registered.
  * `timestamp` (`u64`): Unix timestamp (seconds) when the registration occurred.

#### Registration Event Example (JSON Representation)
```json
{
  "topics": ["PROJECT", "CREATED", 1],
  "data": {
    "project_id": 1,
    "owner": "GBXX...XXXX",
    "name": "SlingShot DEX",
    "category": "DeFi",
    "timestamp": 1782390400
  }
}
```

### Project Updated
* **Topic:** `(Symbol("PROJECT"), Symbol("UPDATED"), project_id: u64)`
* **Payload (`ProjectUpdatedEvent`):**
  * `project_id` (`u64`): The ID of the updated project.
  * `owner` (`Address`): The owner of the project.
  * `timestamp` (`u64`): Unix timestamp (seconds).

### Project Archived
* **Topic:** `(Symbol("PROJECT"), Symbol("ARCHIVED"), project_id: u64)`
* **Payload (`ProjectArchivedEvent`):**
  * `project_id` (`u64`): The ID of the archived project.
  * `archived_by` (`Address`): Address of the admin or owner who archived the project.
  * `timestamp` (`u64`): Unix timestamp.

### Project Reactivated
* **Topic:** `(Symbol("PROJECT"), Symbol("RESTORED"), project_id: u64)`
* **Payload (`ProjectReactivatedEvent`):**
  * `project_id` (`u64`): The ID of the reactivated project.
  * `caller` (`Address`): Address of the admin or owner who reactivated the project.
  * `timestamp` (`u64`): Unix timestamp.

### Project Ownership Transferred
* **Topic:** `(Symbol("PROJECT"), Symbol("TRANSFER"), project_id: u64)`
* **Payload (`ProjectOwnershipTransferredEvent`):**
  * `project_id` (`u64`): The ID of the project.
  * `caller` (`Address`): Address that executed the transfer.
  * `old_owner` (`Address`): The old owner address.
  * `new_owner` (`Address`): The new owner address.
  * `timestamp` (`u64`): Unix timestamp.

### Project Linked
* **Topic:** `(Symbol("PROJECT"), Symbol("LINKED"), project_id: u64)`
* **Payload (`ProjectLinkedEvent`):**
  * `project_id` (`u64`): The ID of the project that initiated the link.
  * `linked_project_id` (`u64`): The ID of the project being linked to.
  * `owner` (`Address`): Address of the project owner (or admin) who created the link.
  * `timestamp` (`u64`): Unix timestamp when the link was created.

#### Project Linked Event Example
```json
{
  "topics": ["PROJECT", "LINKED", 1],
  "data": {
    "project_id": 1,
    "linked_project_id": 2,
    "owner": "GBXX...XXXX",
    "timestamp": 1782390400
  }
}
```

### Project Unlinked
* **Topic:** `(Symbol("PROJECT"), Symbol("UNLINKED"), project_id: u64)`
* **Payload (`ProjectUnlinkedEvent`):**
  * `project_id` (`u64`): The ID of the project from which the link was removed.
  * `linked_project_id` (`u64`): The ID of the project that was unlinked.
  * `owner` (`Address`): Address of the project owner (or admin) who removed the link.
  * `timestamp` (`u64`): Unix timestamp when the link was removed.

#### Project Unlinked Event Example
```json
{
  "topics": ["PROJECT", "UNLINKED", 1],
  "data": {
    "project_id": 1,
    "linked_project_id": 2,
    "owner": "GBXX...XXXX",
    "timestamp": 1782390500
  }
}
```

---

## 2. Review Events

All review-related events start with the topic `REVIEW` (Symbol).

### Review Submitted / Updated / Deleted
* **Topic:** `(Symbol("REVIEW"), action: Symbol, project_id: u64, reviewer: Address)`
  * `action` can be: `SUBMITTED`, `UPDATED`, or `DELETED`.
* **Payload (`ReviewEventData`):**
  * `project_id` (`u64`): The ID of the project reviewed.
  * `reviewer` (`Address`): The reviewer's address.
  * `action` (`ReviewAction`): Enum (`Submitted`, `Updated`, `Deleted`).
  * `timestamp` (`u64`): Unix timestamp of the transaction.
  * `content_cid` (`Option<String>`): The IPFS/content CID containing off-chain review text and metadata.
  * `created_at` (`u64`): Creation timestamp of the review.
  * `updated_at` (`u64`): Last update timestamp of the review.
  * `owner_response` (`Option<String>`): Optional CID of the project owner's response.

#### Review Event Example
```json
{
  "topics": ["REVIEW", "SUBMITTED", 1, "GBXX...XXXX"],
  "data": {
    "project_id": 1,
    "reviewer": "GBXX...XXXX",
    "action": "Submitted",
    "timestamp": 1782390800,
    "content_cid": "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG",
    "created_at": 1782390800,
    "updated_at": 1782390800,
    "owner_response": null
  }
}
```

---

## 3. Fee Events

All fee-related events start with the topic `FEE` (Symbol).

### Fee Set
* **Topic:** `(Symbol("CONFIG"), Symbol("FEE"))`
* **Payload (`FeeSetEvent`):**
  * `admin` (`Address`): The admin who updated the fee configuration.
  * `token` (`Option<Address>`): Address of the SAC token contract used for payment (`None` = native/no token required).
  * `verification_fee` (`u128`): Fee amount required for verification requests.
  * `registration_fee` (`u128`): Fee amount required for project registrations.
  * `treasury` (`Address`): Address of the fee treasury account.
  * `timestamp` (`u64`): Unix timestamp.

### Fee Paid
* **Topic:** `(Symbol("FEE"), Symbol("PAID"), project_id: u64, operation: Symbol)`
  * `operation` is either `Verification` or `Registration`.
* **Payload (`FeePaidEvent`):**
  * `project_id` (`u64`): Project associated with the payment.
  * `payer` (`Address`): Address that paid the fee.
  * `token` (`Option<Address>`): Address of the payment token.
  * `operation` (`FeeOperation`): Enum (`Verification`, `Registration`).
  * `amount` (`u128`): Amount paid.
  * `timestamp` (`u64`): Unix timestamp.

#### Fee Paid Event Example
```json
{
  "topics": ["FEE", "PAID", 1, "Verification"],
  "data": {
    "project_id": 1,
    "payer": "GDXX...XXXX",
    "token": "CAS3...XXXX",
    "operation": "Verification",
    "amount": 100000000,
    "timestamp": 1782390500
  }
}
```

---

## 4. Verification Events

All verification-related events start with the topic `VERIFY` (Symbol).

### Verification Requested
* **Topic:** `(Symbol("VERIFY"), Symbol("REQ"), project_id: u64)`
* **Payload (`VerificationRequestedEvent`):**
  * `project_id` (`u64`): The ID of the project requesting verification.
  * `requester` (`Address`): The requester's address.
  * `evidence_cid` (`String`): The IPFS/content CID containing supporting verification evidence.
  * `timestamp` (`u64`): Unix timestamp.
  * `request_id` (`u64`): ID of the newly created `VerificationRecord` for this request.
  * `previous_request_id` (`Option<u64>`): ID of the project's previous verification
    request, if any. `None` for a project's first-ever request; `Some(id)` marks
    this as a re-request (e.g. after rejection or revocation) that **versions**
    rather than overwrites the record identified by `id` — see
    [`VERIFICATION.md`](VERIFICATION.md#requesting-verification-and-re-request-replacement-rules).

#### Verification Requested Event Example
```json
{
  "topics": ["VERIFY", "REQ", 1],
  "data": {
    "project_id": 1,
    "requester": "GDXX...XXXX",
    "evidence_cid": "QmZ4tUD4vC5P16G1sA1nemtYgPpHdWEz79ojWnPbdG",
    "timestamp": 1782390600,
    "request_id": 2,
    "previous_request_id": 1
  }
}
```

### Verification Approved
* **Topic:** `(Symbol("VERIFY"), Symbol("APP"), project_id: u64)`
* **Payload (`VerificationApprovedEvent`):**
  * `project_id` (`u64`): The ID of the verified project.
  * `admin` (`Address`): Admin address who approved the request.
  * `decided_at` (`u64`): Timestamp when the decision was made.
  * `timestamp` (`u64`): Unix timestamp.

### Verification Rejected
* **Topic:** `(Symbol("VERIFY"), Symbol("REJ"), project_id: u64)`
* **Payload (`VerificationRejectedEvent`):**
  * `project_id` (`u64`): The ID of the project.
  * `admin` (`Address`): Admin address.
  * `decided_at` (`u64`): Timestamp when the decision was made.
  * `timestamp` (`u64`): Unix timestamp.

### Verification Revoked
* **Topic:** `(Symbol("VERIFY"), Symbol("REVOKED"), project_id: u64)`
* **Payload (`VerificationRevokedEvent`):**
  * `project_id` (`u64`): The ID of the project.
  * `admin` (`Address`): Admin address.
  * `reason` (`String`): Explanation string for the revocation.
  * `timestamp` (`u64`): Unix timestamp.

### Verification Expired
* **Topic:** `(Symbol("VERIFY"), Symbol("EXPRD"), project_id: u64)`
* **Payload (`VerificationExpiredEvent`):**
  * `project_id` (`u64`): The ID of the project whose verification expired.
  * `expired_at` (`u64`): Expiry timestamp recorded for the verification.
  * `timestamp` (`u64`): Unix timestamp when the expiry event was emitted.

### Verification Renewed
* **Topic:** `(Symbol("VERIFY"), Symbol("RENEWD"), project_id: u64)`
* **Payload (`VerificationRenewedEvent`):**
  * `project_id` (`u64`): The ID of the renewed project.
  * `admin` (`Address`): Admin address that renewed the verification.
  * `new_expires_at` (`u64`): New verification expiry timestamp.
  * `timestamp` (`u64`): Unix timestamp.

### Verification Evidence Updated
* **Topic:** `(Symbol("VERIFY"), Symbol("EV_UPD"), project_id: u64)`
* **Payload (`VerificationEvidenceUpdatedEvent`):**
  * `project_id` (`u64`): The ID of the project whose evidence changed.
  * `requester` (`Address`): Address that submitted the update.
  * `old_evidence_cid` (`String`): Previous evidence CID.
  * `new_evidence_cid` (`String`): Replacement evidence CID.
  * `timestamp` (`u64`): Unix timestamp.

### Verification Assigned
* **Topic:** `(Symbol("VERIFY"), Symbol("ASSIGNED"), project_id: u64)`
* **Payload (`VerificationAssignedEvent`):**
  * `project_id` (`u64`): The ID of the project assigned for verification review.
  * `request_id` (`u64`): Verification request identifier.
  * `assigned_admin` (`Address`): Admin assigned to review the request.
  * `assigner` (`Address`): Address that made the assignment.
  * `timestamp` (`u64`): Unix timestamp.

### Verification History Cleared
* **Topic:** `(Symbol("VERIFY"), Symbol("HISTCLR"), project_id: u64)`
* **Payload (`VerificationHistoryClearedEvent`):**
  * `project_id` (`u64`): The ID of the project whose verification history was pruned.
  * `admin` (`Address`): Admin address that cleared the history.
  * `removed_count` (`u32`): Number of verification records removed.
  * `retained_count` (`u32`): Number of verification records retained.
  * `timestamp` (`u64`): Unix timestamp.

---

## 5. Renewal Events

Verification renewal events start with the topic `RENEW` (Symbol).

### Verification Renewal Requested
* **Topic:** `(Symbol("RENEW"), Symbol("REQUEST"), project_id: u64)`
* **Payload (`VerificationRenewalReqEvent`):**
  * `project_id` (`u64`): The ID of the project requesting renewal.
  * `requester` (`Address`): Address that requested renewal.
  * `evidence_cid` (`String`): Evidence CID supporting the renewal request.
  * `fee_amount` (`u128`): Fee amount associated with the renewal request.
  * `timestamp` (`u64`): Unix timestamp.

### Verification Renewal Approved
* **Topic:** `(Symbol("RENEW"), Symbol("APPROVED"), project_id: u64)`
* **Payload (`VerificationRenewalApprovedEvent`):**
  * `project_id` (`u64`): The ID of the renewed project.
  * `admin` (`Address`): Admin address that approved the renewal.
  * `expires_at` (`u64`): New verification expiry timestamp.
  * `timestamp` (`u64`): Unix timestamp.

### Verification Renewal Rejected
* **Topic:** `(Symbol("RENEW"), Symbol("REJECTED"), project_id: u64)`
* **Payload (`VerificationRenewalRejectedEvent`):**
  * `project_id` (`u64`): The ID of the project whose renewal was rejected.
  * `admin` (`Address`): Admin address that rejected the renewal.
  * `timestamp` (`u64`): Unix timestamp.

### Renewal History Cleared
* **Topic:** `(Symbol("RENEW"), Symbol("HISTCLR"), project_id: u64)`
* **Payload (`RenewalHistoryClearedEvent`):**
  * `project_id` (`u64`): The ID of the project whose renewal history was cleared.
  * `admin` (`Address`): Admin address that cleared the renewal history.
  * `removed_count` (`u32`): Number of renewal records removed.
  * `timestamp` (`u64`): Unix timestamp.

---

## 6. Verification Configuration Events

Verification configuration events use the `CONFIG` topic namespace.

### Minimum Project Age Set
* **Topic:** `(Symbol("CONFIG"), Symbol("MIN_AGE"))`
* **Payload (`MinProjectAgeSetEvent`):**
  * `admin` (`Address`): Admin address that changed the setting.
  * `previous_min_age_seconds` (`u64`): Previous minimum project age.
  * `min_age_seconds` (`u64`): New minimum project age.
  * `timestamp` (`u64`): Unix timestamp.

### Verification Duration Set
* **Topic:** `(Symbol("CONFIG"), Symbol("DURATION"))`
* **Payload (`VerificationDurationSetEvent`):**
  * `admin` (`Address`): Admin address that changed the setting.
  * `previous_duration_seconds` (`u64`): Previous verification duration.
  * `duration_seconds` (`u64`): New verification duration.
  * `timestamp` (`u64`): Unix timestamp.

---

## 7. Reserved Name Events

Reserved project-name changes use the `CONFIG` topic namespace. See
[`RESERVED_NAMES.md`](./RESERVED_NAMES.md) for the feature overview.

### Reserved Name Added
* **Topic:** `(Symbol("CONFIG"), Symbol("RSVD_ADD"))`
* **Payload (`ReservedNameAddedEvent`):**
  * `name` (`String`): The name that was added to the reserved list.
  * `admin` (`Address`): Admin address that added the name.
  * `timestamp` (`u64`): Unix timestamp.
* Not emitted when the name was already reserved (idempotent no-op).

### Reserved Name Removed
* **Topic:** `(Symbol("CONFIG"), Symbol("RSVD_REM"))`
* **Payload (`ReservedNameRemovedEvent`):**
  * `name` (`String`): The name that was removed from the reserved list.
  * `admin` (`Address`): Admin address that removed the name.
  * `timestamp` (`u64`): Unix timestamp.
* Not emitted when the name was not present (idempotent no-op).

## 8. Smart Collection Events (issue #906)

Rule-based collections use the `SMARTCOL` topic namespace. See
[`SMART_COLLECTION_CURATION.md`](./SMART_COLLECTION_CURATION.md).

### Smart Collection Created
* **Topic:** `(Symbol("SMARTCOL"), Symbol("CREATED"), collection_id: u64)`
* **Payload (`SmartCollectionCreatedEvent`):**
  * `collection_id` (`u64`): The smart collection that was created.
  * `name` (`String`): Display name of the collection.
  * `owner` (`Address`): Admin that owns the collection.
  * `rule_count` (`u32`): Number of curation rules attached.
  * `rule_version` (`u32`): Version of the initial rule set (always `1`).
  * `timestamp` (`u64`): Unix timestamp.

### Smart Collection Rules Updated
* **Topic:** `(Symbol("SMARTCOL"), Symbol("RULES"), collection_id: u64)`
* **Payload (`SmartCollectionRulesUpdatedEvent`):**
  * `collection_id` (`u64`): The smart collection.
  * `admin` (`Address`): Admin that changed the rule set.
  * `previous_version` (`u32`): Version that was active before the change.
  * `new_version` (`u32`): Version that is active after the change.
  * `rule_count` (`u32`): Number of rules in the new rule set.
  * `timestamp` (`u64`): Unix timestamp.

### Smart Collection Rolled Back
* **Topic:** `(Symbol("SMARTCOL"), Symbol("ROLLBACK"), collection_id: u64)`
* **Payload (`SmartCollectionRolledBackEvent`):**
  * `collection_id` (`u64`): The smart collection.
  * `admin` (`Address`): Admin that performed the rollback.
  * `restored_from_version` (`u32`): Stored version whose rules were restored.
  * `new_version` (`u32`): Version created by the rollback.
  * `timestamp` (`u64`): Unix timestamp.

### Smart Collection Refreshed
* **Topic:** `(Symbol("SMARTCOL"), Symbol("REFRESH"), collection_id: u64)`
* **Payload (`SmartCollectionRefreshedEvent`):**
  * `collection_id` (`u64`): The smart collection.
  * `caller` (`Address`): Address that triggered the refresh.
  * `rule_version` (`u32`): Rule version that was applied.
  * `added` (`u32`): Projects added by this refresh.
  * `removed` (`u32`): Projects removed by this refresh.
  * `unchanged` (`u32`): Auto-managed projects that still matched.
  * `total_matched` (`u32`): Projects that matched the rules.
  * `scanned` (`u32`): Projects that were scanned.
  * `timestamp` (`u64`): Unix timestamp.

## 9. Collection Membership Vote Events (issue #907)

Community votes use the `COLVOTE` topic namespace. See
[`COLLECTION_MEMBERSHIP_VOTING.md`](./COLLECTION_MEMBERSHIP_VOTING.md).

### Membership Vote Cast
* **Topic:** `(Symbol("COLVOTE"), Symbol("CAST"), collection_id: u64, project_id: u64)`
* **Payload (`CollectionMembershipVoteCastEvent`):**
  * `collection_id` (`u64`): The collection that was voted on.
  * `project_id` (`u64`): The project that was voted on.
  * `voter` (`Address`): Address that cast the vote.
  * `direction` (`VoteDirection`): `Approve` or `Reject`.
  * `approve_votes` (`u32`): Running approve-vote count.
  * `reject_votes` (`u32`): Running reject-vote count.
  * `timestamp` (`u64`): Unix timestamp.

### Membership Vote Resolved
* **Topic:** `(Symbol("COLVOTE"), Symbol("RESOLVED"), collection_id: u64, project_id: u64)`
* **Payload (`CollectionMembershipResolvedEvent`):**
  * `collection_id` (`u64`): The collection that was voted on.
  * `project_id` (`u64`): The project that was voted on.
  * `outcome` (`MembershipVoteOutcome`): `Approved` or `Rejected`.
  * `approve_votes` (`u32`): Running approve-vote count.
  * `reject_votes` (`u32`): Running reject-vote count.
  * `approval_bps` (`u32`): Approval ratio in basis points.
  * `timestamp` (`u64`): Unix timestamp.

## 10. Pause Recovery Events (issue #903)

Staged pause recovery uses the `PAUSE` topic namespace. See
[`PAUSE_RECOVERY_PHASES.md`](./PAUSE_RECOVERY_PHASES.md).

### Pause Phase Changed
* **Topic:** `(Symbol("PAUSE"), Symbol("PHASE"))`
* **Payload (`PausePhaseChangedEvent`):**
  * `phase` (`u32`): The phase that is current after the change (`0` = nothing live).
  * `previous_phase` (`u32`): The phase that was current before the change.
  * `enabled_phases` (`Vec<u32>`): All live recovery phases.
  * `completed_phases` (`Vec<u32>`): Phases of the plan already moved past.
  * `admin` (`Address`): Admin that performed the change.
  * `rolled_back` (`bool`): `true` when the change was a rollback.
  * `timestamp` (`u64`): Unix timestamp.

## 11. Admin Rotation Events (issue #900)

Automatic admin-key rotation uses the `ADMINROT` topic namespace. See
[`ADMIN_ROTATION_SCHEDULES.md`](./ADMIN_ROTATION_SCHEDULES.md).

### Admin Rotation Scheduled
* **Topic:** `(Symbol("ADMINROT"), Symbol("SCHEDULED"))`
* **Payload (`AdminRotationScheduledEvent`):**
  * `interval_days` (`u32`): Rotation interval (30, 60 or 90 days).
  * `next_rotation_at` (`u64`): When the next rotation becomes due.
  * `admin` (`Address`): Admin that installed the schedule.
  * `replacement_pool_size` (`u32`): Number of replacement candidates.
  * `timestamp` (`u64`): Unix timestamp.

### Admin Rotation Preflight Confirmed
* **Topic:** `(Symbol("ADMINROT"), Symbol("PREFLIGHT"))`
* **Payload (`AdminRotationPreflightEvent`):**
  * `rotation_id` (`u64`): Rotation the confirmation applies to.
  * `admin` (`Address`): Admin that confirmed the checklist.
  * `checklist_size` (`u32`): Number of checklist items.
  * `all_satisfied` (`bool`): Whether every checklist item passed.
  * `timestamp` (`u64`): Unix timestamp.

### Admin Rotation Executed
* **Topic:** `(Symbol("ADMINROT"), Symbol("EXECUTED"), rotation_id: u64)`
* **Payload (`AdminRotationExecutedEvent`):**
  * `rotation_id` (`u64`): The executed rotation.
  * `executed_by` (`Address`): Keeper that executed the rotation.
  * `removed_admins` (`Vec<Address>`): Admins that were removed.
  * `added_admins` (`Vec<Address>`): Admins that were promoted.
  * `next_rotation_at` (`u64`): When the next rotation becomes due.
  * `timestamp` (`u64`): Unix timestamp.

### Admin Rotation Notification
* **Topic:** `(Symbol("ADMINROT"), Symbol("NOTIFY"), rotation_id: u64)`
* **Payload (`AdminRotationNotificationEvent`):**
  * `rotation_id` (`u64`): The rotation the notification belongs to.
  * `admin` (`Address`): The affected admin key holder.
  * `removed` (`bool`): `true` for the outgoing key, `false` for the incoming one.
  * `timestamp` (`u64`): Unix timestamp.