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
* **Topic:** `(Symbol("PROJECT"), Symbol("ACTIVE"), project_id: u64)`
* **Payload (`ProjectReactivatedEvent`):**
  * `project_id` (`u64`): The ID of the reactivated project.
  * `caller` (`Address`): Address of the admin or owner who reactivated the project.
  * `timestamp` (`u64`): Unix timestamp.

### Project Ownership Transferred
* **Topic:** `(Symbol("PROJECT"), Symbol("OWNER_TR"), project_id: u64)`
* **Payload (`ProjectOwnershipTransferredEvent`):**
  * `project_id` (`u64`): The ID of the project.
  * `caller` (`Address`): Address that executed the transfer.
  * `old_owner` (`Address`): The old owner address.
  * `new_owner` (`Address`): The new owner address.
  * `timestamp` (`u64`): Unix timestamp.

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
* **Topic:** `(Symbol("FEE"), Symbol("SET"))`
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
* **Topic:** `(Symbol("VERIFY"), Symbol("REQUEST"), project_id: u64)`
* **Payload (`VerificationRequestedEvent`):**
  * `project_id` (`u64`): The ID of the project requesting verification.
  * `requester` (`Address`): The requester's address.
  * `evidence_cid` (`String`): The IPFS/content CID containing supporting verification evidence.
  * `timestamp` (`u64`): Unix timestamp.

#### Verification Requested Event Example
```json
{
  "topics": ["VERIFY", "REQUEST", 1],
  "data": {
    "project_id": 1,
    "requester": "GDXX...XXXX",
    "evidence_cid": "QmZ4tUD4vC5P16G1sA1nemtYgPpHdWEz79ojWnPbdG",
    "timestamp": 1782390600
  }
}
```

### Verification Approved
* **Topic:** `(Symbol("VERIFY"), Symbol("APPROVED"), project_id: u64)`
* **Payload (`VerificationApprovedEvent`):**
  * `project_id` (`u64`): The ID of the verified project.
  * `admin` (`Address`): Admin address who approved the request.
  * `timestamp` (`u64`): Unix timestamp.

### Verification Rejected
* **Topic:** `(Symbol("VERIFY"), Symbol("REJECTED"), project_id: u64)`
* **Payload (`VerificationRejectedEvent`):**
  * `project_id` (`u64`): The ID of the project.
  * `admin` (`Address`): Admin address.
  * `timestamp` (`u64`): Unix timestamp.

### Verification Revoked
* **Topic:** `(Symbol("VERIFY"), Symbol("REVOKED"), project_id: u64)`
* **Payload (`VerificationRevokedEvent`):**
  * `project_id` (`u64`): The ID of the project.
  * `admin` (`Address`): Admin address.
  * `reason` (`String`): Explanation string for the revocation.
  * `timestamp` (`u64`): Unix timestamp.
