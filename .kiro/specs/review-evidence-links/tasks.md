# Implementation Plan: Review Evidence Links

## Overview

Extend the Dongle-Smartcontract review system to allow reviewers to attach up to 5 external URLs as supporting evidence alongside their reviews. Changes are distributed across `constants.rs`, `errors.rs`, `types.rs`, `storage_keys.rs`, `review_registry/validation.rs`, `review_registry/storage.rs`, `storage_manager.rs`, `events.rs`, and `lib.rs`. A new `ExtensionKey2` enum is required because `ExtensionKey` is at the 50-variant Soroban cap.

## Tasks

- [x] 1. Add constants, new error variants, and the `EvidenceLink` type
  - [x] 1.1 Add `MAX_EVIDENCE_LINKS_PER_REVIEW: u32 = 5` and `MAX_EVIDENCE_LINK_URL_LEN: usize = 512` to `constants.rs`
    - Place them alongside the existing review-related constants
    - _Requirements: 9.1_
  - [x] 1.2 Add three new error variants to `errors.rs`
    - `TooManyEvidenceLinks = 82`
    - `InvalidEvidenceLink = 83`
    - `EvidenceLinkTooLong = 84`
    - _Requirements: 2.2, 2.3, 1.3_
  - [x] 1.3 Add the `EvidenceLink` struct to `types.rs`
    - Fields: `url: String`, `is_dead: bool`
    - Annotate with `#[contracttype]`, `#[derive(Clone, Debug, Eq, PartialEq)]`
    - _Requirements: 8.3_
  - [x] 1.4 Add `evidence_links: Vec<EvidenceLink>` field to the `Review` struct in `types.rs`
    - Append after all existing fields to preserve ABI forward-compatibility
    - _Requirements: 4.4_
  - [x] 1.5 Add `evidence_links: Vec<EvidenceLink>` field to `ReviewEventData` in `types.rs`
    - Append after all existing fields
    - _Requirements: 6.1, 6.2, 6.3_
  - [x] 1.6 Add `max_evidence_links_per_review: u32` field to `ContractLimits` in `types.rs`
    - Append after all existing fields
    - _Requirements: 9.3_

- [x] 2. Introduce `ExtensionKey2` storage enum and update the storage key uniqueness test
  - [x] 2.1 Add `ExtensionKey2` enum to `storage_keys.rs` with a single variant: `ReviewEvidenceLinks(u64, Address)`
    - Follow the same pattern as `ExtensionKey` (comment block, `#[contracttype]`, `#[derive(Clone, Debug, Eq, PartialEq)]`)
    - Document that `ExtensionKey` is at the 50-variant Soroban cap so this new enum is required
    - _Requirements: 7.1, 7.2_
  - [ ]* 2.2 Update `storage_key_uniqueness.rs` to add `ExtensionKey2` variant count tracking
    - Add `extension_key2_variant_count()` function listing all variants
    - Add `extension_key2_variant_count_within_soroban_cap` and `extension_key2_variant_count_below_warn_threshold` tests
    - Add a cross-enum isolation test confirming `ExtensionKey2::ReviewEvidenceLinks` does not collide with `ExtensionKey` or `StorageKey`
    - _Requirements: 7.2_

- [x] 3. Add evidence link URL validation to `review_registry/validation.rs`
  - [x] 3.1 Implement `ReviewValidation::validate_evidence_link_url(url: &String) -> Result<(), ContractError>`
    - Reject empty strings with `InvalidEvidenceLink`
    - Reject URLs exceeding `MAX_EVIDENCE_LINK_URL_LEN` with `EvidenceLinkTooLong`
    - Copy the URL bytes into a stack buffer using `url.copy_into_slice`; check for `https://` or `http://` prefix; reject with `InvalidEvidenceLink` if neither matches
    - _Requirements: 2.1, 2.2, 2.3_
  - [x] 3.2 Implement `ReviewValidation::validate_evidence_links(links: &Vec<EvidenceLink>) -> Result<(), ContractError>`
    - Reject if `links.len() as u32 > MAX_EVIDENCE_LINKS_PER_REVIEW` with `TooManyEvidenceLinks`
    - Call `validate_evidence_link_url` on each link URL; propagate any error
    - _Requirements: 1.3, 3.4_
  - [ ]* 3.3 Write property test for `validate_evidence_link_url` (Property 3)
    - **Property 3: URL scheme and length validation**
    - **Validates: Requirements 2.1, 2.2, 2.3**
    - Use `proptest` to generate strings with valid/invalid prefixes and lengths; assert accept iff prefix ∈ {`http://`, `https://`} AND `len ∈ [1, 512]`
    - Tag: `// Feature: review-evidence-links, Property 3: URL scheme and length validation`

- [x] 4. Extend `StorageManager` to bump evidence link TTL alongside review TTL
  - [x] 4.1 Add `use crate::storage_keys::ExtensionKey2;` import to `storage_manager.rs`
  - [x] 4.2 Extend `StorageManager::extend_review_ttl` to also call `extend_if_exists` on `ExtensionKey2::ReviewEvidenceLinks(project_id, reviewer.clone())` using `LEDGER_THRESHOLD_REVIEW` / `LEDGER_BUMP_REVIEW`
    - _Requirements: 7.3, 7.4_

- [x] 5. Implement evidence link storage helpers in `review_registry/storage.rs`
  - [x] 5.1 Add `use crate::storage_keys::ExtensionKey2;` and `use crate::types::EvidenceLink;` imports
  - [x] 5.2 Implement private helper `ReviewRegistry::store_evidence_links(env, project_id, reviewer, links: &Vec<EvidenceLink>)` that writes the list under `ExtensionKey2::ReviewEvidenceLinks(project_id, reviewer.clone())`
  - [x] 5.3 Implement private helper `ReviewRegistry::load_evidence_links(env, project_id, reviewer) -> Vec<EvidenceLink>` that reads the list, returning an empty `Vec` when absent
  - [x] 5.4 Implement private helper `ReviewRegistry::delete_evidence_links(env, project_id, reviewer)` that removes the key from persistent storage (no-op if absent)
    - _Requirements: 5.1, 5.2, 5.3_
  - [x] 5.5 Implement `ReviewRegistry::get_review_evidence_links(env, project_id, reviewer) -> Vec<EvidenceLink>` (public) delegating to `load_evidence_links`
    - Return empty `Vec` for non-existent review or missing key
    - _Requirements: 4.1, 4.2, 4.3_
  - [x] 5.6 Implement `ReviewRegistry::mark_evidence_link_dead_impl(env, admin, project_id, reviewer, link_index) -> Result<(), ContractError>`
    - Validate admin: `admin.require_auth()` + `AdminManager::is_admin` check; return `AdminOnly` on failure
    - Load review; return `ReviewNotFound` if absent
    - Load links; return `InvalidInput` if `link_index >= links.len()`
    - Flip `links[link_index].is_dead = true`; write back (idempotent)
    - _Requirements: 8.3, 8.4, 8.5_

- [ ] 6. Update `add_review` and `submit_review` to accept and store evidence links
  - [ ] 6.1 Update `ReviewRegistry::add_review` signature to add `evidence_links: Option<Vec<EvidenceLink>>` as final parameter
    - When `Some(links)`, call `ReviewValidation::validate_evidence_links(&links)?` before any mutations
    - When `None`, treat as empty list
    - After persisting the `Review` struct, call `store_evidence_links` with the resolved list
    - Populate the `evidence_links` field in the `Review` struct before storing it
    - Update `publish_review_event` call to pass the evidence links snapshot in the `ReviewEventData`
    - _Requirements: 1.1, 1.2, 1.4, 6.1_
  - [ ] 6.2 Update `ReviewRegistry::submit_review` signature to add `evidence_links: Option<Vec<EvidenceLink>>` as final parameter and forward it to `add_review`
    - _Requirements: 1.1_
  - [ ]* 6.3 Write property test for evidence link round-trip on submission (Property 1)
    - **Property 1: Evidence links round-trip on submission**
    - **Validates: Requirements 1.1, 4.1**
    - Generate random valid `(project_id, reviewer, rating, evidence_links)` tuples; assert `get_review_evidence_links == submitted`
    - Tag: `// Feature: review-evidence-links, Property 1: Evidence links round-trip on submission`
  - [ ]* 6.4 Write property test for over-limit submission rejection (Property 2)
    - **Property 2: Over-limit submission is always rejected**
    - **Validates: Requirements 1.3, 9.2**
    - Generate lists of length `6..=20`; assert `add_review` / `submit_review` returns `TooManyEvidenceLinks` and the review list is unchanged
    - Tag: `// Feature: review-evidence-links, Property 2: Over-limit submission is always rejected`

- [~] 7. Update `update_review` to accept and apply evidence link changes
  - [ ] 7.1 Update `ReviewRegistry::update_review` signature to add `evidence_links: Option<Vec<EvidenceLink>>` as final parameter
    - `None` → leave stored links unchanged
    - `Some(links)` where `links.is_empty()` → clear stored links via `delete_evidence_links`
    - `Some(links)` (non-empty) → validate then replace via `store_evidence_links`
    - Populate `evidence_links` field on the updated `Review` struct by calling `load_evidence_links`
    - Update `publish_review_event` call to carry the post-update evidence links snapshot
    - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 6.2_
  - [ ]* 7.2 Write property test: update with `Some(replacement)` replaces links (Property 4)
    - **Property 4: Update replaces links**
    - **Validates: Requirements 3.1**
    - Tag: `// Feature: review-evidence-links, Property 4: Update replaces links`
  - [ ]* 7.3 Write property test: update with `None` preserves links (Property 5)
    - **Property 5: Update with None preserves links**
    - **Validates: Requirements 3.2**
    - Tag: `// Feature: review-evidence-links, Property 5: Update with None preserves links`
  - [ ]* 7.4 Write property test: update with `Some([])` clears links (Property 6)
    - **Property 6: Update with Some([]) clears links**
    - **Validates: Requirements 3.3**
    - Tag: `// Feature: review-evidence-links, Property 6: Update with Some([]) clears links`

- [~] 8. Checkpoint — ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [~] 9. Update `delete_review` and `admin_delete_review` to clean up evidence links
  - [ ] 9.1 In `ReviewRegistry::delete_review`, after removing the review key, call `delete_evidence_links(env, project_id, &reviewer)` so no orphaned link data remains
    - Update the `publish_review_event` (Deleted) call to pass the evidence links snapshot captured before deletion
    - _Requirements: 5.1, 5.3, 6.3_
  - [ ] 9.2 In `ReviewRegistry::admin_delete_review`, after removing the review key, call `delete_evidence_links(env, project_id, &reviewer)` similarly
    - _Requirements: 5.2, 5.3_
  - [ ]* 9.3 Write property test: deletion removes evidence links (Property 7)
    - **Property 7: Deletion removes evidence links**
    - **Validates: Requirements 5.1, 5.2, 5.3**
    - Test both `delete_review` (owner) and `admin_delete_review` paths; assert `get_review_evidence_links` returns empty after each
    - Tag: `// Feature: review-evidence-links, Property 7: Deletion removes evidence links`

- [~] 10. Update `get_review` to hydrate `evidence_links` from the separate storage key
  - [ ] 10.1 In `ReviewRegistry::get_review`, after loading the `Review` struct, call `load_evidence_links(env, project_id, reviewer)` and assign the result to `review.evidence_links` before returning
    - This ensures `list_reviews`, `list_reviews_sorted`, and `get_reviews_by_ids` also return hydrated reviews without code duplication (they all delegate to `get_review`)
    - _Requirements: 4.4_

- [~] 11. Update `publish_review_event` to carry the evidence links snapshot
  - [ ] 11.1 Add `evidence_links: Vec<EvidenceLink>` parameter to `publish_review_event` in `events.rs`
    - Assign it to `ReviewEventData::evidence_links` in the struct literal
    - Update all call sites in `review_registry/storage.rs` to pass the appropriate snapshot
    - _Requirements: 6.1, 6.2, 6.3, 6.4_
  - [ ]* 11.2 Write property test: review events carry the correct evidence links snapshot (Property 8)
    - **Property 8: Review events carry evidence links snapshot**
    - **Validates: Requirements 6.1, 6.2, 6.3**
    - Generate random links and random actions; inspect emitted `ReviewEventData.evidence_links` equals action-time state
    - Tag: `// Feature: review-evidence-links, Property 8: Review events carry evidence links snapshot`

- [~] 12. Expose new entry points in `lib.rs`
  - [ ] 12.1 Add `use crate::types::EvidenceLink;` to the use block in `lib.rs`
  - [ ] 12.2 Update `DongleContract::add_review` signature to add `evidence_links: Option<Vec<EvidenceLink>>` and forward to `ReviewRegistry::add_review`
    - _Requirements: 1.1_
  - [ ] 12.3 Update `DongleContract::submit_review` signature to add `evidence_links: Option<Vec<EvidenceLink>>` and forward to `ReviewRegistry::submit_review`
    - _Requirements: 1.1_
  - [ ] 12.4 Update `DongleContract::update_review` signature to add `evidence_links: Option<Vec<EvidenceLink>>` and forward to `ReviewRegistry::update_review`
    - _Requirements: 3.1, 3.2, 3.3_
  - [ ] 12.5 Add `DongleContract::get_review_evidence_links(env, project_id, reviewer) -> Vec<EvidenceLink>` delegating to `ReviewRegistry::get_review_evidence_links`
    - _Requirements: 4.1, 4.2, 4.3_
  - [ ] 12.6 Add `DongleContract::mark_evidence_link_dead(env, admin, project_id, reviewer, link_index) -> Result<(), ContractError>` delegating to `ReviewRegistry::mark_evidence_link_dead_impl`
    - _Requirements: 8.3, 8.4, 8.5_

- [~] 13. Update `get_config` to surface `max_evidence_links_per_review` in `ContractLimits`
  - [ ] 13.1 In `config_registry.rs`, set `ContractLimits::max_evidence_links_per_review = MAX_EVIDENCE_LINKS_PER_REVIEW` in the `ContractLimits` struct literal returned by `get_config`
    - _Requirements: 9.3_

- [~] 14. Update existing test call sites to pass `None` for the new evidence links parameter
  - [ ] 14.1 Search all occurrences of `add_review(`, `submit_review(`, and `update_review(` in `dongle-smartcontract/src/tests/` and append `, None` to each call so the existing test suite continues to compile and pass
    - _Requirements: backward compatibility_

- [~] 15. Add `review_evidence_links` test module
  - [ ] 15.1 Create `dongle-smartcontract/src/tests/review_evidence_links.rs` with unit tests covering:
    - Happy-path: submit review with zero links; `get_review_evidence_links` returns empty `Vec`
    - Happy-path: submit review with exactly `MAX_EVIDENCE_LINKS_PER_REVIEW` (5) links; links are stored and retrieved correctly
    - `update_review` with `None` leaves existing links unchanged
    - `update_review` with `Some([])` clears all links
    - `get_review_evidence_links` on a non-existent review returns empty `Vec`
    - `mark_evidence_link_dead` by admin on valid index succeeds; flag is `true` for that index only
    - `mark_evidence_link_dead` on second call is idempotent (flag stays `true`)
    - `admin_delete_review` removes evidence links storage entry
    - `get_config` returns `ContractLimits` with `max_evidence_links_per_review == 5`
    - `extend_review_ttl` touches the `ReviewEvidenceLinks` storage entry (no panic under normal operation)
    - _Requirements: 1.1, 1.2, 2.1, 3.2, 3.3, 4.1, 4.2, 4.3, 5.2, 8.3, 9.3_
  - [ ] 15.2 Register `mod review_evidence_links;` in `dongle-smartcontract/src/tests/mod.rs`
  - [ ]* 15.3 Write property test: `mark_evidence_link_dead` requires admin (Property 9)
    - **Property 9: mark_evidence_link_dead requires admin**
    - **Validates: Requirements 8.4**
    - Generate random non-admin addresses; assert `mark_evidence_link_dead` returns `AdminOnly` and no link is modified
    - Tag: `// Feature: review-evidence-links, Property 9: mark_evidence_link_dead requires admin`
  - [ ]* 15.4 Write property test: `mark_evidence_link_dead` sets flag precisely (Property 10)
    - **Property 10: mark_evidence_link_dead sets flag precisely**
    - **Validates: Requirements 8.3**
    - Generate random valid links list (1–5 links) and random index in range; assert `links[i].is_dead == true` while all others unchanged; assert second call is idempotent
    - Tag: `// Feature: review-evidence-links, Property 10: mark_evidence_link_dead sets flag precisely`

- [~] 16. Final checkpoint — ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for faster MVP delivery; the core feature will work without property-based tests.
- All property-based tests use `proptest` following the pattern in `src/tests/proptest_validation.rs`.
- Each task references specific acceptance criteria for traceability.
- `ExtensionKey2` must be introduced before any evidence link storage operation is compiled; Task 2 must be completed before Tasks 4–12.
- The `evidence_links` field is stored **separately** from the `Review` struct (under `ExtensionKey2::ReviewEvidenceLinks`); `get_review` hydrates the combined view at read time (Task 10).
- All existing `add_review` / `submit_review` / `update_review` call sites in the test suite must be updated in Task 14 before running tests.
