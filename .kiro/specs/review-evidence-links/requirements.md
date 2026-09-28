# Requirements Document

## Introduction

This feature adds support for attaching external evidence links to reviews in the Dongle-Smartcontract (Soroban/Stellar) system. Reviewers can attach up to 5 URLs as proof or documentation alongside their review content. The contract stores, validates, and exposes these links on-chain. Reachability checking, dead-link detection, and link-preview/metadata extraction are off-chain responsibilities delegated to indexers or frontend clients, since Soroban smart contracts cannot make outbound network calls.

## Glossary

- **Contract**: The Soroban smart contract managing the review and project registry.
- **Reviewer**: An authenticated NEAR/Stellar address that has submitted a review for a project.
- **Review**: An on-chain record keyed by `(project_id, reviewer)` containing a rating, optional content CID, and optional evidence links.
- **EvidenceLink**: A single URL string (HTTPS or HTTP) attached to a review as supporting proof or documentation.
- **EvidenceLinks**: The ordered list of `EvidenceLink` items attached to a single review. Maximum cardinality: 5.
- **Link_Validator**: The on-chain validation logic that checks URL format and length constraints for evidence links.
- **ReviewRegistry**: The on-chain module (`review_registry`) responsible for creating, updating, deleting, and listing reviews.
- **Indexer**: An off-chain service that reads contract events and performs operations not possible on-chain (reachability checks, metadata extraction, dead-link detection).
- **Dead Link**: A URL that is no longer reachable, as determined by an off-chain Indexer.
- **ExtensionKey2**: A new Soroban `#[contracttype]` storage key enum introduced when `ExtensionKey` reaches its 50-variant cap.

---

## Requirements

### Requirement 1: Attach Evidence Links on Review Submission

**User Story:** As a Reviewer, I want to attach up to 5 external URLs to my review submission, so that I can provide verifiable proof or documentation supporting my assessment.

#### Acceptance Criteria

1. WHEN a reviewer calls `submit_review` or `add_review` with an optional `evidence_links` parameter containing a non-empty list, THE ReviewRegistry SHALL store the provided evidence links associated with the review, keyed by `(project_id, reviewer)`.
2. WHEN a reviewer submits a review with zero evidence links, THE ReviewRegistry SHALL create the review with an empty evidence links list and SHALL NOT return an error.
3. WHEN a reviewer submits a review with more than 5 evidence links, THE ReviewRegistry SHALL reject the submission and SHALL return `ContractError::TooManyEvidenceLinks`.
4. THE ReviewRegistry SHALL accept evidence links only when the parent review passes all existing validation rules (project existence, ownership check, duplicate check, eligibility check).

---

### Requirement 2: Evidence Link Format Validation

**User Story:** As a Reviewer, I want the contract to reject malformed URLs before storing them, so that the evidence links list contains only structurally valid references.

#### Acceptance Criteria

1. WHEN an evidence link is provided, THE Link_Validator SHALL verify that the URL begins with `https://` or `http://`.
2. WHEN an evidence link exceeds 512 bytes in length, THE Link_Validator SHALL reject it and THE ReviewRegistry SHALL return `ContractError::EvidenceLinkTooLong`.
3. WHEN an evidence link is an empty string, THE Link_Validator SHALL reject it and THE ReviewRegistry SHALL return `ContractError::InvalidEvidenceLink`.
4. WHEN all provided evidence links pass format validation, THE Link_Validator SHALL return no error and submission SHALL proceed.
5. THE Contract SHALL NOT perform reachability or liveness checks on evidence link URLs at submission time; such checks are the responsibility of the Indexer.

---

### Requirement 3: Update Evidence Links on Review Update

**User Story:** As a Reviewer, I want to update the evidence links attached to my review, so that I can replace outdated proof with current documentation.

#### Acceptance Criteria

1. WHEN a reviewer calls `update_review` with a new `evidence_links` value, THE ReviewRegistry SHALL replace the stored evidence links for that review with the new list.
2. WHEN a reviewer calls `update_review` with `None` for `evidence_links`, THE ReviewRegistry SHALL leave the existing evidence links unchanged.
3. WHEN a reviewer calls `update_review` with `Some([])` (an explicitly empty list) for `evidence_links`, THE ReviewRegistry SHALL clear all stored evidence links for the review.
4. WHEN a reviewer calls `update_review` with more than 5 evidence links, THE ReviewRegistry SHALL reject the update and SHALL return `ContractError::TooManyEvidenceLinks`.
5. WHEN evidence links are updated, THE ReviewRegistry SHALL apply the existing update cooldown (`REVIEW_UPDATE_COOLDOWN_SECONDS`) and SHALL enforce all existing `update_review` validation rules.

---

### Requirement 4: Read Evidence Links

**User Story:** As an Indexer or frontend client, I want to retrieve the evidence links for a specific review, so that I can display proof references and perform off-chain reachability checks.

#### Acceptance Criteria

1. WHEN `get_review_evidence_links` is called with a valid `(project_id, reviewer)` pair, THE ReviewRegistry SHALL return the stored `Vec<EvidenceLink>` for that review.
2. WHEN `get_review_evidence_links` is called for a review with no stored links, THE ReviewRegistry SHALL return an empty `Vec`.
3. WHEN `get_review_evidence_links` is called for a non-existent review, THE ReviewRegistry SHALL return an empty `Vec`.
4. WHEN `list_reviews` or `get_review` returns a `Review` record, THE ReviewRegistry SHALL include the `evidence_links` field in the returned struct so that callers can access links without a second round-trip.

---

### Requirement 5: Evidence Links Lifecycle — Deletion

**User Story:** As a Reviewer or admin, I want evidence links to be removed when a review is deleted, so that no orphaned link data persists in contract storage.

#### Acceptance Criteria

1. WHEN `delete_review` is called and succeeds, THE ReviewRegistry SHALL remove the stored evidence links entry for `(project_id, reviewer)`.
2. WHEN `admin_delete_review` is called and succeeds, THE ReviewRegistry SHALL remove the stored evidence links entry for `(project_id, reviewer)`.
3. AFTER a review is deleted, THE ReviewRegistry SHALL return an empty `Vec` for any subsequent `get_review_evidence_links` call on the same `(project_id, reviewer)` pair.

---

### Requirement 6: Evidence Links in Review Events

**User Story:** As an Indexer, I want review events to carry the current evidence links snapshot, so that I can maintain an off-chain mirror without querying the contract separately.

#### Acceptance Criteria

1. WHEN a `ReviewAction::Submitted` event is emitted, THE Contract SHALL include the submitted evidence links in the event payload.
2. WHEN a `ReviewAction::Updated` event is emitted, THE Contract SHALL include the new evidence links in the event payload.
3. WHEN a `ReviewAction::Deleted` event is emitted, THE Contract SHALL include the evidence links that were present at deletion time in the event payload.
4. THE Contract SHALL emit evidence link events only as part of the existing `publish_review_event` mechanism and SHALL NOT introduce a separate event channel for evidence links.

---

### Requirement 7: Storage Capacity and Key Namespacing

**User Story:** As a contract maintainer, I want evidence link data to be stored under a well-namespaced key that does not push any `#[contracttype]` enum past Soroban's 50-variant limit, so that the contract continues to compile and deploy without storage collisions.

#### Acceptance Criteria

1. THE Contract SHALL store evidence links under a new storage key `ReviewEvidenceLinks(u64, Address)` (keyed by `project_id` and `reviewer`).
2. IF adding `ReviewEvidenceLinks` to `ExtensionKey` would cause `ExtensionKey` to exceed 50 variants, THE Contract SHALL introduce a new `ExtensionKey2` enum following the same pattern documented in `storage_keys.rs`.
3. THE Contract SHALL use TTL extension for evidence link entries consistent with `LEDGER_THRESHOLD_REVIEW` / `LEDGER_BUMP_REVIEW` to keep evidence data live for the same duration as the parent review.
4. WHEN a review's TTL is extended by `StorageManager::extend_review_ttl`, THE StorageManager SHALL also extend the TTL of the corresponding evidence links storage entry.

---

### Requirement 8: Off-chain Dead Link Detection and Notification

**User Story:** As a user browsing reviews, I want to be notified when an evidence link is no longer reachable, so that I can assess the current validity of the reviewer's proof.

#### Acceptance Criteria

1. THE Contract SHALL NOT perform HTTP reachability checks; dead-link detection is the exclusive responsibility of the Indexer.
2. WHEN the Indexer detects that an evidence link URL returns a non-2xx response or connection failure, THE Indexer SHALL record the link as dead in its off-chain data store and expose a `dead_link` flag through the HubDApp frontend API.
3. THE Contract SHALL expose a `mark_evidence_link_dead` admin entry point that allows an admin to set a per-link `is_dead` boolean flag on-chain, enabling frontends to surface dead-link warnings without depending on an Indexer.
4. WHEN `mark_evidence_link_dead` is called by a non-admin address, THE Contract SHALL return `ContractError::AdminOnly`.
5. WHEN `mark_evidence_link_dead` is called for a review or link index that does not exist, THE Contract SHALL return `ContractError::ReviewNotFound` or `ContractError::InvalidInput` respectively.

---

### Requirement 9: Maximum Link Count Configurability

**User Story:** As a contract admin, I want the maximum number of evidence links per review to be a named constant, so that the limit can be adjusted in a future upgrade without hunting for magic numbers.

#### Acceptance Criteria

1. THE Contract SHALL define a constant `MAX_EVIDENCE_LINKS_PER_REVIEW: u32 = 5` in `constants.rs`.
2. WHEN validation logic checks the count of submitted evidence links, THE Link_Validator SHALL compare against `MAX_EVIDENCE_LINKS_PER_REVIEW` and not against a hard-coded literal.
3. THE Contract SHALL surface `MAX_EVIDENCE_LINKS_PER_REVIEW` in the `ContractLimits` struct returned by `get_config`, so that frontend clients can enforce the limit before submitting a transaction.
