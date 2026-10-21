# Implementation Plan: Consensus Rating

## Overview

This feature adds a reliability-weighted consensus rating layer on top of the existing Bayesian rating system. The implementation is entirely additive: a new `consensus_rating/` module with four source files, a new `ExtensionKey2` enum appended to `storage_keys.rs`, two new error codes in `errors.rs`, the `on_review_mutated` hook wired into the five mutating paths in `review_registry/storage.rs`, and six new entry points in `lib.rs`. Nothing else changes.

The design document in `.kiro/specs/consensus-rating/design.md` and the requirements in `.kiro/specs/consensus-rating/requirements.md` are the authoritative references for every task below.

## Tasks

- [ ] 1. Add `ExtensionKey2` storage enum and new error codes
  - In `dongle-smartcontract/src/storage_keys.rs`, append a new `#[contracttype]` enum `ExtensionKey2` with exactly these five variants matching the design:
    - `ReviewerStats(Address)`
    - `ConsensusStats(u64)`
    - `BrigadingWindow(u64)`
    - `ReviewQuarantine(u64, Address)`
    - `ConsensusRatingConfig`
  - Add `pub use crate::storage_keys::ExtensionKey2;` where other key enums are used (module root re-export if needed).
  - In `dongle-smartcontract/src/errors.rs`, append two new error codes after the existing last code (`AlreadyMaintainerAdded = 80`, `DisputeNotPending = 81`):
    - `ReviewAlreadyClearFlagged = 82` — admin calls `clear_brigading_flag` on a review that is not quarantined
    - `ConsensusConfigInvalid = 83` — `set_consensus_rating_config` receives invalid config (e.g. `brigading_count_threshold = 0`)
  - _Requirements: 5.1, 5.2, 5.5, Design §Data Models §ExtensionKey2, Design §Error Handling_

- [ ] 2. Create `consensus_rating/types.rs` — all new `#[contracttype]` structs and the event type
  - Create `dongle-smartcontract/src/consensus_rating/types.rs` with exactly the types specified in the design:
    - `ReviewerStats { total_reviews: u32, accuracy_sum: u64, reliability_score: u32 }`
    - `ConsensusStats { weighted_sum: u64, total_weight: u64, consensus_rating: u32, quarantined_count: u32 }`
    - `ConsensusRatingConfig { outlier_threshold: u32, brigading_window_seconds: u64, brigading_count_threshold: u32, trusted_reviewer_score_threshold: u32 }`
    - `BrigadingWindow { candidates: Vec<Address>, timestamps: Vec<u64>, window_start: u64 }`
    - `BrigadingDetectedEvent { project_id: u64, quarantined_count: u32, detection_timestamp: u64 }`
  - All types must derive `#[contracttype]`, `Clone`, `Debug`, `Eq`, `PartialEq`.
  - `BrigadingWindow` uses `soroban_sdk::Vec`; `BrigadingDetectedEvent` is emitted on-chain via `env.events().publish`.
  - _Requirements: 1.1, 2.4, 3.1, 4.1, Design §Data Models_

- [ ] 3. Create `consensus_rating/calculator.rs` — pure math, no storage
  - Create `dongle-smartcontract/src/consensus_rating/calculator.rs` implementing `ConsensusRatingCalculator`:
    - `compute_consensus(reviews: &[(u32, u64)]) -> u32`: weighted average of `(rating, weight)` pairs, scaled by 100; returns `WEIGHTED_RATING_PRIOR_MEAN` (350) when `total_weight == 0`
    - `weight_from_score(reliability_score: u32) -> u64`: returns `max(1, reliability_score / 100)`; score 0–99 → weight 1, score 100–199 → weight 1 (integer division), score 10000 → weight 100
    - `score_from_components(accuracy_sum: u64, total_reviews: u32) -> u32`: returns `max(0, 10000u64.saturating_sub(accuracy_sum / total_reviews as u64)) as u32`; guard against `total_reviews == 0` returning 5000 (neutral default)
  - No storage imports needed; use `saturating_*` arithmetic throughout matching the existing `RatingCalculator` convention.
  - _Requirements: 1.4, 2.1, 2.2, 2.3, Design §ConsensusRatingCalculator_

  - [ ]* 3.1 Write property tests for `ConsensusRatingCalculator`
    - Add a `#[cfg(test)] mod prop_tests` block at the bottom of `calculator.rs` using `proptest`.
    - **Property 3: Weight floor invariant** — for any `reliability_score` in `0..=10000`, `weight_from_score` must be `>= 1`.
    - **Property 2: Consensus rating formula consistency** — for any non-empty set of `(rating, weight)` pairs, `compute_consensus` must equal `weighted_sum / total_weight` (integer division).
    - **Property 1: Score formula consistency** — for any `accuracy_sum` and `total_reviews > 0`, `score_from_components` must equal `max(0, 10000 - accuracy_sum / total_reviews)`.
    - **Validates: Requirements 1.4, 1.7, 2.1, 2.2, 2.8**

- [ ] 4. Create `consensus_rating/registry.rs` — `ReviewerReliabilityRegistry` storage layer
  - Create `dongle-smartcontract/src/consensus_rating/registry.rs` implementing `ReviewerReliabilityRegistry`:
    - `get_reviewer_stats(env, reviewer) -> Option<ReviewerStats>`: reads `ExtensionKey2::ReviewerStats(reviewer)` from persistent storage; returns `None` if absent.
    - `init_reviewer(env, reviewer)`: writes `ReviewerStats { total_reviews: 1, accuracy_sum: 0, reliability_score: 5000 }` to `ExtensionKey2::ReviewerStats(reviewer)`. Only call if the key does not already exist (first review).
    - `on_review_deleted(env, reviewer)`: decrements `total_reviews` by 1. If result is 0, removes the key entirely (resets to neutral / no history). Otherwise recomputes `reliability_score = score_from_components(accuracy_sum, total_reviews)` and writes back.
    - `update_accuracy_for_project(env, project_id, new_consensus_rating)`: reads `StorageKey::ProjectReviews(project_id)` to get reviewer list; for each reviewer whose review is not quarantined (check `ExtensionKey2::ReviewQuarantine(project_id, reviewer)`), reads that reviewer's `Review` to get their `rating`, computes delta `|reviewer.rating * 100 - new_consensus_rating|` (as `u64`), accumulates into `accuracy_sum`, then recomputes and persists `reliability_score`. Perform at most N+2 persistent reads and N+1 persistent writes for N non-quarantined reviewers.
  - Extend TTL on all written entries using `LEDGER_THRESHOLD_REVIEW` / `LEDGER_BUMP_REVIEW` constants.
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 5.1, 5.3_

- [ ] 5. Create `consensus_rating/brigading.rs` — `BrigadingDetector`
  - Create `dongle-smartcontract/src/consensus_rating/brigading.rs` implementing `BrigadingDetector`:
    - `inspect(env, project_id, reviewer, rating, consensus_rating, config) -> bool`:
      1. Compute deviation `(rating as u64 * 100).abs_diff(consensus_rating as u64)`.
      2. If deviation `> config.outlier_threshold as u64`: load or create `BrigadingWindow` from `ExtensionKey2::BrigadingWindow(project_id)`; prune candidates whose timestamp is outside `now - config.brigading_window_seconds`; append `(reviewer, now)` to the window.
      3. If the number of in-window candidates exceeds `config.brigading_count_threshold`: for each candidate, check the reviewer's `reliability_score`; if score `<= config.trusted_reviewer_score_threshold`, set `ExtensionKey2::ReviewQuarantine(project_id, reviewer) = true`; emit `BrigadingDetectedEvent`; return `true`.
      4. Otherwise save the updated window and return `false`.
    - `clear_flag(env, project_id, reviewer)`: removes `ExtensionKey2::ReviewQuarantine(project_id, reviewer)`. If key was absent, callers should propagate `ContractError::ReviewAlreadyClearFlagged`.
    - `get_quarantined(env, project_id) -> Vec<Address>`: reads `StorageKey::ProjectReviews(project_id)`, filters addresses where `ExtensionKey2::ReviewQuarantine(project_id, addr)` is `true`.
    - `is_quarantined(env, project_id, reviewer) -> bool`: reads `ExtensionKey2::ReviewQuarantine(project_id, reviewer)`, returns `false` if absent.
  - Event emission: `env.events().publish((symbol_short!("BRIGADE"), symbol_short!("DETECT"), project_id), BrigadingDetectedEvent { ... })`.
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 3.8, 5.4_

- [ ] 6. Create `consensus_rating/mod.rs` — `ConsensusRatingRegistry` orchestrator and module root
  - Create `dongle-smartcontract/src/consensus_rating/mod.rs` with:
    - Module declarations: `mod calculator; mod registry; mod brigading; pub mod types;`
    - Re-exports: `pub use types::{ConsensusRatingConfig, ConsensusStats, ReviewerStats};` (and `BrigadingWindow`, `BrigadingDetectedEvent` as needed internally)
    - `pub struct ConsensusRatingRegistry;` with all six methods as specified in the design:
      - `on_review_mutated(env, project_id)`:
        1. Load config (or defaults).
        2. Load all reviews for the project via `StorageKey::ProjectReviews`.
        3. For each newly added review (context caller), run `BrigadingDetector::inspect`.
        4. Recompute `ConsensusStats` by iterating reviews, skipping hidden and quarantined, computing `weight = weight_from_score(reviewer_stats.reliability_score)` (default weight 1 if no stats yet), accumulating `weighted_sum += weight * rating * 100` and `total_weight += weight`.
        5. Set `consensus_rating = if total_weight > 0 { weighted_sum / total_weight } else { WEIGHTED_RATING_PRIOR_MEAN }`.
        6. Persist `ExtensionKey2::ConsensusStats(project_id)`.
        7. Call `ReviewerReliabilityRegistry::update_accuracy_for_project(env, project_id, new_consensus_rating)`.
      - `get_consensus_rating(env, project_id) -> u32`: reads `ExtensionKey2::ConsensusStats(project_id)`, returns `consensus_rating` field; returns 350 if absent (backward compat for legacy projects).
      - `get_reviewer_stats(env, reviewer) -> Option<ReviewerStats>`: delegates to `ReviewerReliabilityRegistry::get_reviewer_stats`.
      - `get_quarantined_reviews(env, project_id) -> Vec<Address>`: delegates to `BrigadingDetector::get_quarantined`.
      - `set_consensus_rating_config(env, caller, config) -> Result<(), ContractError>`: require auth, check admin, validate `config.brigading_count_threshold > 0` (else `ConsensusConfigInvalid`), persist to `ExtensionKey2::ConsensusRatingConfig`.
      - `get_consensus_rating_config(env) -> ConsensusRatingConfig`: reads `ExtensionKey2::ConsensusRatingConfig`; returns defaults `{ outlier_threshold: 200, brigading_window_seconds: 86400, brigading_count_threshold: 5, trusted_reviewer_score_threshold: 8000 }` if absent.
      - `clear_brigading_flag(env, admin, project_id, reviewer) -> Result<(), ContractError>`: require auth, check admin, verify review exists (else `ReviewNotFound`), call `BrigadingDetector::clear_flag` (propagate `ReviewAlreadyClearFlagged` if not quarantined), then call `on_review_mutated` to recompute.
  - _Requirements: 2.1–2.8, 3.7, 4.1–4.5, 6.1–6.4_

- [ ] 7. Checkpoint — module compiles cleanly
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 8. Wire `on_review_mutated` hook into `review_registry/storage.rs`
  - In `dongle-smartcontract/src/review_registry/storage.rs`, add a `use crate::consensus_rating::ConsensusRatingRegistry;` import.
  - At the end of each of the five mutating functions (after all stats writes and before `Ok(())`), add the single line:
    - `add_review`: after `publish_review_event` call → `ConsensusRatingRegistry::on_review_mutated(env, project_id);`
    - `update_review`: same position → `ConsensusRatingRegistry::on_review_mutated(env, project_id);`
    - `delete_review`: same position → `ConsensusRatingRegistry::on_review_mutated(env, project_id);`
    - `hide_review`: after `publish_review_hidden_event` call → `ConsensusRatingRegistry::on_review_mutated(env, project_id);`
    - `restore_review`: after `publish_review_restored_event` call → `ConsensusRatingRegistry::on_review_mutated(env, project_id);`
  - Also call `ReviewerReliabilityRegistry::init_reviewer(env, &reviewer)` inside `add_review` after the first-interaction recording (only when the reviewer's stats key does not yet exist — the `init_reviewer` function guards this internally).
  - Also call `ReviewerReliabilityRegistry::on_review_deleted(env, &reviewer)` inside `delete_review` before `publish_review_event`.
  - Do NOT modify `admin_delete_review` — that path does not require reviewer stat updates since it is an admin hard-delete, but it MUST call `ConsensusRatingRegistry::on_review_mutated` for consensus recomputation.
  - _Requirements: 1.2, 1.5, 2.5, Design §Integration with ReviewRegistry_

- [ ] 9. Register the `consensus_rating` module and add new entry points to `lib.rs`
  - In `dongle-smartcontract/src/lib.rs`:
    - Add `pub mod consensus_rating;` near the top with the other module declarations.
    - Add `use crate::consensus_rating::{ConsensusRatingRegistry, ConsensusRatingConfig, ReviewerStats};` to the use block.
    - Add `use crate::consensus_rating::types::ConsensusStats;` if needed by return types.
    - Add the six new `#[contractimpl]` methods under a `// --- Consensus Rating ---` comment section:
      ```rust
      pub fn get_consensus_rating(env: Env, project_id: u64) -> u32 {
          ConsensusRatingRegistry::get_consensus_rating(&env, project_id)
      }
      pub fn get_reviewer_stats(env: Env, reviewer: Address) -> Option<ReviewerStats> {
          ConsensusRatingRegistry::get_reviewer_stats(&env, reviewer)
      }
      pub fn get_quarantined_reviews(env: Env, project_id: u64) -> Vec<Address> {
          ConsensusRatingRegistry::get_quarantined_reviews(&env, project_id)
      }
      pub fn get_consensus_rating_config(env: Env) -> ConsensusRatingConfig {
          ConsensusRatingRegistry::get_consensus_rating_config(&env)
      }
      pub fn set_consensus_rating_config(
          env: Env,
          caller: Address,
          config: ConsensusRatingConfig,
      ) -> Result<(), ContractError> {
          ConsensusRatingRegistry::set_consensus_rating_config(&env, caller, config)
      }
      pub fn clear_brigading_flag(
          env: Env,
          admin: Address,
          project_id: u64,
          reviewer: Address,
      ) -> Result<(), ContractError> {
          ConsensusRatingRegistry::clear_brigading_flag(&env, admin, project_id, reviewer)
      }
      ```
  - All function names fit within Soroban's 32-character limit (`get_consensus_rating_config` = 26 chars, `set_consensus_rating_config` = 27 chars, `get_quarantined_reviews` = 23 chars — all safe).
  - _Requirements: 2.6, 3.8, 4.3, 4.5, 6.4_

- [ ] 10. Checkpoint — full build passes and existing tests are green
  - Run `cargo build` in `dongle-smartcontract/` and fix any compilation errors.
  - Run `cargo test` in `dongle-smartcontract/` and confirm all pre-existing tests still pass.
  - Ensure all tests pass, ask the user if questions arise.

- [ ] 11. Write integration tests in `src/tests/consensus_rating.rs`
  - Create `dongle-smartcontract/src/tests/consensus_rating.rs`.
  - Register it in `dongle-smartcontract/src/tests/mod.rs` with `mod consensus_rating;`.
  - Cover the following scenarios (unit tests under `#[cfg(test)]` using `Env::default()` + `mock_all_auths`):
    - **Legacy project backward compat**: register a project, add no reviews via the new path; call `get_consensus_rating` → must return 350 without panic. (_Requirements: 6.3_)
    - **Single review consensus**: one reviewer (rating 4) → `get_consensus_rating` returns 400; `get_project_stats` still returns its original `average_rating`; `get_weighted_rating` is unchanged. (_Requirements: 2.1, 6.1_)
    - **Multi-reviewer weighted average**: two reviewers, one with a high reliability score and one with low (verify by submitting several reviews first); after both reviews, confirm `ConsensusStats.weighted_sum / ConsensusStats.total_weight == consensus_rating`. (_Requirements: 2.1, 2.8_)
    - **Weight floor**: a new reviewer (score 0, first review) must still have weight ≥ 1. (_Requirements: 2.2_)
    - **Delete resets stats**: reviewer submits, then deletes; `get_reviewer_stats` returns `None` (or neutral). (_Requirements: 1.5_)
    - **Full brigading burst**: 6 reviewers each submit a 1-star review within the same window on a 5-star project (consensus ~500); all 6 should be quarantined; `get_quarantined_reviews` returns 6 addresses; consensus excludes them → returns 350. (_Requirements: 3.3, 3.4_)
    - **Trusted reviewer immunity**: reviewer with score > 8000 submits an outlier; they must not be quarantined. (_Requirements: 3.5_)
    - **Admin clear flag**: quarantine a review, then call `clear_brigading_flag`; review is no longer quarantined; consensus recomputes to include it. (_Requirements: 3.7_)
    - **Config round-trip**: `set_consensus_rating_config` followed by `get_consensus_rating_config` returns identical values. (_Requirements: 4.3_)
    - **Non-admin config rejected**: non-admin calling `set_consensus_rating_config` → `ContractError::AdminOnly`. (_Requirements: 4.4_)
    - **ReviewAlreadyClearFlagged**: calling `clear_brigading_flag` on a non-quarantined review → `ContractError::ReviewAlreadyClearFlagged`. (_Requirements: Design §Error Handling_)

  - [ ]* 11.1 Write property test: score formula consistency (Property 1)
    - Add `#[cfg(test)] mod prop_tests` block in `consensus_rating.rs` or `consensus_rating/mod.rs`.
    - **Property 1: Reviewer stats score formula consistency**
    - For any (`accuracy_sum: u64`, `total_reviews: 1..=1000`), `score_from_components(accuracy_sum, total_reviews)` must equal `max(0, 10000u64.saturating_sub(accuracy_sum / total_reviews as u64)) as u32`.
    - **Validates: Requirements 1.4, 1.7**

  - [ ]* 11.2 Write property test: consensus rating formula consistency (Property 2)
    - **Property 2: Consensus rating formula consistency**
    - For any non-empty `Vec<(rating: 1..=5, weight: 1..=100)>`, `compute_consensus` must equal `weighted_sum / total_weight` (integer division).
    - **Validates: Requirements 2.1, 2.8**

  - [ ]* 11.3 Write property test: weight floor invariant (Property 3)
    - **Property 3: Weight floor invariant**
    - For any `reliability_score: 0..=10000`, `weight_from_score(reliability_score)` must be `>= 1`.
    - **Validates: Requirements 2.2**

  - [ ]* 11.4 Write property test: outlier detection threshold (Property 5)
    - **Property 5: Outlier detection threshold**
    - For any `review_rating: 1..=5` and `consensus_rating: 100..=500`, if `(review_rating as u64 * 100).abs_diff(consensus_rating as u64) > outlier_threshold as u64`, then `BrigadingDetector::inspect` (called in isolation) must record the review as a candidate.
    - **Validates: Requirements 3.2**

  - [ ]* 11.5 Write property test: config round-trip (Property 9)
    - **Property 9: Config round-trip**
    - For any valid `ConsensusRatingConfig` (all fields non-zero, `brigading_count_threshold >= 1`), `set_consensus_rating_config` → `get_consensus_rating_config` must return identical field values.
    - **Validates: Requirements 4.3**

  - [ ]* 11.6 Write property test: reviewer deletion round-trip (Property 11)
    - **Property 11: Reviewer deletion round-trip**
    - For `total_reviews = 1`: after `on_review_deleted`, stats key must be absent (`get_reviewer_stats` returns `None`).
    - For `total_reviews > 1`: after `on_review_deleted`, `total_reviews` decrements by exactly 1.
    - **Validates: Requirements 1.5**

- [ ] 12. Final checkpoint — all tests pass
  - Run `cargo test` in `dongle-smartcontract/` and confirm all tests (including new ones) pass.
  - Ensure all tests pass, ask the user if questions arise.

## Notes

- Tasks marked with `*` are optional and can be skipped for a faster MVP.
- `ExtensionKey2` must be a fresh `#[contracttype]` enum in `storage_keys.rs` — do not add to `ExtensionKey` (already at 50 variants).
- The `on_review_mutated` hook is the single integration point; it is called unconditionally after each mutation regardless of the outcome of brigading detection.
- All weighted arithmetic uses `saturating_add` / `saturating_mul` to match the existing `RatingCalculator` convention and avoid panics from overflow.
- The consensus rating does not replace `ProjectStats` or `get_weighted_rating`; it is an entirely separate aggregate exposed through new entry points only.
- Property tests 4, 6, 7, and 8 (from the design) are covered by the integration-style unit tests in task 11 rather than proptest loops, because they require full `Env::default()` contract context.
