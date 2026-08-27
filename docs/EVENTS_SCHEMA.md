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
