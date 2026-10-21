# Requirements Document

## Introduction

This feature extends the existing Bayesian rating system in `rating_calculator.rs` with a **consensus rating** layer that weights each review by the submitting reviewer's historical accuracy/reliability. The goal is to surface a more trustworthy aggregate rating for each project by down-weighting reviews from unreliable or potentially colluding reviewers while up-weighting reviews from reviewers whose past ratings align with eventual community consensus.

The feature adds three capabilities to the smart contract:
1. **Reviewer reliability tracking** — record per-reviewer accuracy statistics as reviews accumulate.
2. **Weighted average rating** — compute a project's consensus rating using reliability-adjusted weights instead of a flat average.
3. **Brigading detection** — flag and quarantine coordinated bursts of outlier reviews that are statistically inconsistent with the reviewer's prior behavior.

The existing `ProjectStats` (raw sum + count + simple Bayesian average) is preserved unchanged. The consensus rating is an additional, separate aggregate.

## Glossary

- **Consensus_Rating_Calculator**: The new module responsible for computing reliability-weighted project ratings.
- **Reviewer_Reliability_Registry**: The storage and computation layer that tracks each reviewer's accuracy statistics.
- **Brigading_Detector**: The component that identifies and flags coordinated review patterns inconsistent with normal reviewer behavior.
- **Reliability_Score**: A per-reviewer value in the range [0, 100] representing how closely the reviewer's past ratings have matched subsequent community consensus. Stored scaled by 100 internally (range 0–10 000).
- **Consensus_Rating**: The project aggregate rating computed by weighting each review by its reviewer's Reliability_Score. Scaled by 100 like the existing `average_rating` (e.g., 425 = 4.25 stars).
- **Reviewer_Stats**: A stored record holding the reviewer's cumulative accuracy data: total reviews submitted, sum of accuracy deltas, and current Reliability_Score.
- **Brigading_Window**: A configurable time window (in seconds) used to detect unusually dense clusters of coordinated reviews.
- **Outlier_Threshold**: A configurable margin (scaled by 100) beyond which a single review's rating is considered an outlier relative to the reviewer's historical mean.
- **Quarantined_Review**: A review flagged by brigading detection whose weight is set to zero in consensus rating calculations until an admin clears the flag.
- **Weight**: A u32 value derived from a reviewer's Reliability_Score used to scale that reviewer's rating contribution in the weighted sum. Minimum weight is 1 (so every non-quarantined review contributes at least something).

## Requirements

### Requirement 1: Reviewer Reliability Tracking

**User Story:** As a project evaluator, I want each reviewer's historical rating accuracy tracked on-chain, so that reviews from more reliable reviewers carry more influence in the consensus rating.

#### Acceptance Criteria

1. THE Reviewer_Reliability_Registry SHALL store a `ReviewerStats` record for every address that has submitted at least one review, keyed by reviewer address.
2. WHEN a reviewer submits their first review, THE Reviewer_Reliability_Registry SHALL initialize that reviewer's `ReviewerStats` with `total_reviews = 1`, `accuracy_sum = 0`, and `reliability_score = 5000` (representing a neutral 50/100 starting score).
3. WHEN the community consensus for a project's rating changes (i.e., after any review is added, updated, or removed for that project), THE Reviewer_Reliability_Registry SHALL update the `accuracy_sum` for every reviewer who has a non-quarantined review on that project by computing `|reviewer_rating * 100 - new_consensus_rating|` and accumulating it.
4. THE Reviewer_Reliability_Registry SHALL recompute each affected reviewer's `reliability_score` as `max(0, 10000 - (accuracy_sum / total_reviews))`, clamped to the range [0, 10000].
5. WHEN a reviewer deletes their review, THE Reviewer_Reliability_Registry SHALL decrement `total_reviews` by 1, and IF `total_reviews` reaches 0, THEN THE Reviewer_Reliability_Registry SHALL reset the reviewer's `ReviewerStats` to the initial neutral values.
6. THE Reviewer_Reliability_Registry SHALL expose a read-only query `get_reviewer_stats(reviewer: Address) -> Option<ReviewerStats>` returning the current stats or `None` if the reviewer has no history.
7. FOR ALL reviewers with `total_reviews > 0`, the stored `reliability_score` SHALL equal `max(0, 10000 - (accuracy_sum / total_reviews))` (round-trip consistency between stored score and derivable score from stored components).

### Requirement 2: Weighted Average Rating Calculation

**User Story:** As a DApp user browsing projects, I want to see a consensus rating that reflects reviewer reliability, so that I can trust the displayed rating more than a simple average.

#### Acceptance Criteria

1. THE Consensus_Rating_Calculator SHALL compute a project's consensus rating as the reliability-weighted sum of non-quarantined reviews divided by the total weight of those reviews, scaled by 100.
2. WHEN computing the consensus rating, THE Consensus_Rating_Calculator SHALL assign each non-quarantined reviewer a weight equal to `max(1, reviewer.reliability_score / 100)`, so that every review contributes at least 1 unit of weight regardless of reliability score.
3. WHEN a project has zero non-quarantined reviews, THE Consensus_Rating_Calculator SHALL return the Bayesian prior mean (350, representing 3.50 stars) as the consensus rating.
4. THE Consensus_Rating_Calculator SHALL store the computed consensus rating in a `ConsensusStats` record per project containing: `weighted_sum: u64`, `total_weight: u64`, `consensus_rating: u32`, and `quarantined_count: u32`.
5. WHEN any review for a project is added, updated, removed, or quarantine-flagged, THE Consensus_Rating_Calculator SHALL recompute and persist the updated `ConsensusStats` for that project within the same transaction.
6. THE Consensus_Rating_Calculator SHALL expose a read-only query `get_consensus_rating(project_id: u64) -> u32` returning the current consensus rating scaled by 100 (e.g., 425 = 4.25 stars).
7. WHEN a reviewer's `reliability_score` changes, THE Consensus_Rating_Calculator SHALL NOT retroactively recompute all project consensus ratings in the same transaction; consensus ratings SHALL be recomputed lazily on the next review mutation for each affected project to bound gas costs.
8. FOR ALL projects with at least one non-quarantined review, the stored `consensus_rating` SHALL equal `weighted_sum / total_weight` (round-trip consistency between stored rating and derivable value from stored components).

### Requirement 3: Brigading Detection

**User Story:** As a project owner, I want coordinated fake-review attacks detected and neutralized automatically, so that my project's consensus rating is not manipulated by bad actors.

#### Acceptance Criteria

1. THE Brigading_Detector SHALL inspect every newly submitted review for a project against the project's current consensus rating and the submitting reviewer's historical mean rating.
2. WHEN a review's rating deviates from the project's current consensus rating by more than the configured `Outlier_Threshold` (default: 200, representing 2.00 stars scaled by 100), THE Brigading_Detector SHALL record that review as a candidate outlier.
3. WHEN the number of candidate outlier reviews submitted to a single project within the configured `Brigading_Window` (default: 86400 seconds / 24 hours) exceeds the configured brigading count threshold (default: 5), THE Brigading_Detector SHALL mark all reviews in that burst window as `quarantined = true`.
4. WHEN a review is quarantined, THE Brigading_Detector SHALL set its weight to 0 in consensus rating calculations and increment `ConsensusStats.quarantined_count` for the affected project.
5. IF a quarantined review is submitted by a reviewer whose `reliability_score` is above 8000 (i.e., a trusted reviewer), THEN THE Brigading_Detector SHALL NOT quarantine that review.
6. THE Brigading_Detector SHALL emit a contract event of type `BrigadingDetected` containing: `project_id`, `quarantined_count`, and `detection_timestamp` when a brigading burst is first flagged for a project within a given window.
7. WHEN an admin calls `clear_brigading_flag(project_id, reviewer)`, THE Brigading_Detector SHALL set `quarantined = false` for the specified review and trigger a consensus rating recomputation for the affected project.
8. THE Brigading_Detector SHALL expose a read-only query `get_quarantined_reviews(project_id: u64) -> Vec<Address>` returning the list of reviewer addresses whose reviews are currently quarantined for the given project.

### Requirement 4: Brigading and Reliability Configuration

**User Story:** As a contract admin, I want to configure brigading detection thresholds and reliability parameters, so that the system can be tuned without redeployment.

#### Acceptance Criteria

1. THE Consensus_Rating_Calculator SHALL store a `ConsensusRatingConfig` record containing: `outlier_threshold: u32`, `brigading_window_seconds: u64`, `brigading_count_threshold: u32`, and `trusted_reviewer_score_threshold: u32`.
2. WHEN no `ConsensusRatingConfig` has been set by an admin, THE Consensus_Rating_Calculator SHALL use the following defaults: `outlier_threshold = 200`, `brigading_window_seconds = 86400`, `brigading_count_threshold = 5`, `trusted_reviewer_score_threshold = 8000`.
3. WHEN an admin calls `set_consensus_rating_config(config: ConsensusRatingConfig)`, THE Consensus_Rating_Calculator SHALL persist the new configuration and apply it to all subsequent review submissions and brigading checks.
4. IF a caller of `set_consensus_rating_config` is not a registered admin, THEN THE Consensus_Rating_Calculator SHALL return `ContractError::AdminOnly`.
5. THE Consensus_Rating_Calculator SHALL expose a read-only query `get_consensus_rating_config() -> ConsensusRatingConfig` returning the current configuration (or defaults if none set).

### Requirement 5: Storage and Gas Efficiency

**User Story:** As a contract developer, I want the consensus rating feature to stay within Soroban ledger entry and CPU budget constraints, so that the contract remains deployable and callable within gas limits.

#### Acceptance Criteria

1. THE Reviewer_Reliability_Registry SHALL store one ledger entry per reviewer address (keyed by a new `ExtensionKey2` variant to avoid exceeding the 50-variant cap on `ExtensionKey`).
2. THE Consensus_Rating_Calculator SHALL store one `ConsensusStats` ledger entry per project (also using `ExtensionKey2` variants).
3. WHEN updating reviewer accuracy after a consensus rating change for a project with N non-quarantined reviewers, THE Reviewer_Reliability_Registry SHALL perform at most N+2 persistent storage reads and N+1 persistent storage writes in a single transaction.
4. THE Brigading_Detector SHALL store per-project outlier candidate data in a single compacted ledger entry (not one entry per candidate review) to bound storage growth.
5. WHEN the `ExtensionKey` enum approaches 45 variants, THE Consensus_Rating_Calculator SHALL use a new `ExtensionKey2` enum following the same split pattern documented in `storage_keys.rs`, ensuring the 50-variant cap is never exceeded.

### Requirement 6: Backward Compatibility

**User Story:** As an integrator using the existing rating API, I want the new consensus rating to be purely additive, so that existing `ProjectStats` and `get_weighted_rating` calls are unaffected.

#### Acceptance Criteria

1. THE Consensus_Rating_Calculator SHALL NOT modify the existing `ProjectStats` struct or the `RatingCalculator` module.
2. THE Consensus_Rating_Calculator SHALL NOT change the behavior of `add_rating`, `update_rating`, `remove_rating`, or `calculate_weighted` in `RatingCalculator`.
3. WHEN consensus rating storage is uninitialized for a project (e.g., projects registered before this feature is deployed), THE Consensus_Rating_Calculator SHALL return the Bayesian prior mean (350) from `get_consensus_rating` without error.
4. THE Consensus_Rating_Calculator SHALL expose all new data through new contract entry points (`get_consensus_rating`, `get_reviewer_stats`, `get_quarantined_reviews`, `get_consensus_rating_config`) that do not conflict with any existing function signatures.
