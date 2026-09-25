//! Review registry storage mutations: CRUD, moderation, aggregates, and listing.

use crate::admin_action_log::AdminActionLog;
use crate::config_registry::ConfigRegistry;
use crate::constants::{
    DEFAULT_MIN_REVIEWER_AGE_SECONDS, DEFAULT_REQUIRE_ENDORSEMENT, DEFAULT_REVIEW_FEE,
    LEDGER_BUMP_ARCHIVED_REVIEW, LEDGER_BUMP_REVIEW, LEDGER_THRESHOLD_ARCHIVED_REVIEW,
    LEDGER_THRESHOLD_REVIEW, MAX_ARCHIVE_BATCH_SIZE, MAX_PAGE_LIMIT, MAX_REVIEWS_PER_USER,
    MAX_REVIEW_REVISIONS, REVIEW_ARCHIVE_AGE_SECONDS, REVIEW_UPDATE_COOLDOWN_SECONDS,
};
use crate::errors::ContractError;
use crate::events::{
    publish_review_archived_event, publish_review_event, publish_review_integrity_sealed_event,
    publish_review_integrity_violation_event, publish_review_revision_event,
};
use crate::project_registry::ProjectRegistry;
use crate::rating_calculator::RatingCalculator;
use crate::review_registry::validation::ReviewValidation;
use crate::storage_keys::{ExtensionKey, ExtensionKey2, ReviewIntegrityKey, StorageKey};
use crate::storage_manager::StorageManager;
use crate::types::{
    AdminActionType, ArchivedReview, EvidenceLink, Project, ProjectStats, Review, ReviewAction,
    ReviewEligibilityConfig, ReviewIntegrityRecord, ReviewIntegrityStatus, ReviewRevision,
    ReviewSortMode, ReviewTombstone,
};
use soroban_sdk::xdr::ToXdr;
use soroban_sdk::{Address, Env, String, Vec};

pub struct ReviewRegistry;

impl ReviewRegistry {
    // ── Anti-Sybil Review Eligibility ───────────────────────────────────

    /// Retrieve the current review eligibility configuration.
    /// Returns the default (fully permissive) config if never set by an admin.
    pub fn get_review_eligibility_config(env: &Env) -> ReviewEligibilityConfig {
        env.storage()
            .persistent()
            .get(&ExtensionKey::ReviewEligibilityConfig)
            .unwrap_or(ReviewEligibilityConfig {
                min_reviewer_age_seconds: DEFAULT_MIN_REVIEWER_AGE_SECONDS,
                require_endorsement: DEFAULT_REQUIRE_ENDORSEMENT,
                review_fee: DEFAULT_REVIEW_FEE,
            })
    }

    /// Admin-only: set the review eligibility configuration.
    ///
    /// Passing a zero-valued config restores the default (permissive) behaviour.
    pub fn set_review_eligibility_config(
        env: &Env,
        admin: Address,
        config: ReviewEligibilityConfig,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        if !crate::admin_manager::AdminManager::is_admin(env, &admin) {
            return Err(ContractError::AdminOnly);
        }
        env.storage()
            .persistent()
            .set(&ExtensionKey::ReviewEligibilityConfig, &config);
        Ok(())
    }

    // ── Evidence Link Storage Helpers ────────────────────────────────────

    /// Store a list of evidence links for a (project_id, reviewer) pair.
    ///
    /// Writes under `ExtensionKey2::ReviewEvidenceLinks` and extends the TTL.
    /// Requirements: 5.2
    fn store_evidence_links(
        env: &Env,
        project_id: u64,
        reviewer: &Address,
        links: &Vec<EvidenceLink>,
    ) {
        let key = ExtensionKey2::ReviewEvidenceLinks(project_id, reviewer.clone());
        env.storage().persistent().set(&key, links);
        env.storage()
            .persistent()
            .extend_ttl(&key, LEDGER_THRESHOLD_REVIEW, LEDGER_BUMP_REVIEW);
    }

    /// Load evidence links for a (project_id, reviewer) pair.
    ///
    /// Returns an empty `Vec` when no entry exists.
    /// Requirements: 5.3
    fn load_evidence_links(env: &Env, project_id: u64, reviewer: &Address) -> Vec<EvidenceLink> {
        let key = ExtensionKey2::ReviewEvidenceLinks(project_id, reviewer.clone());
        env.storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| Vec::new(env))
    }

    /// Remove evidence links for a (project_id, reviewer) pair from persistent storage.
    ///
    /// No-op if the entry does not exist.
    /// Requirements: 5.1
    fn delete_evidence_links(env: &Env, project_id: u64, reviewer: &Address) {
        let key = ExtensionKey2::ReviewEvidenceLinks(project_id, reviewer.clone());
        if env.storage().persistent().has(&key) {
            env.storage().persistent().remove(&key);
        }
    }

    /// Return the evidence links for a review.
    ///
    /// Returns an empty `Vec` for non-existent reviews or when no links are stored.
    /// Requirements: 4.1, 4.2, 4.3
    pub fn get_review_evidence_links(
        env: &Env,
        project_id: u64,
        reviewer: &Address,
    ) -> Vec<EvidenceLink> {
        Self::load_evidence_links(env, project_id, reviewer)
    }

    /// Admin-only: mark a specific evidence link as dead (set `is_dead = true`).
    ///
    /// - `admin.require_auth()` is called then admin role is checked.
    /// - Returns `ReviewNotFound` if no review exists for (project_id, reviewer).
    /// - Returns `InvalidInput` if `link_index >= links.len()`.
    /// - Idempotent: calling again on an already-dead link leaves it dead.
    ///
    /// Requirements: 8.3, 8.4, 8.5
    pub fn mark_evidence_link_dead_impl(
        env: &Env,
        admin: Address,
        project_id: u64,
        reviewer: Address,
        link_index: u32,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        if !crate::admin_manager::AdminManager::is_admin(env, &admin) {
            return Err(ContractError::AdminOnly);
        }

        // Ensure the review exists.
        let review_key = StorageKey::Review(project_id, reviewer.clone());
        if !env.storage().persistent().has(&review_key) {
            return Err(ContractError::ReviewNotFound);
        }

        let links = Self::load_evidence_links(env, project_id, &reviewer);
        if link_index >= links.len() as u32 {
            return Err(ContractError::InvalidInput);
        }

        // Build a new Vec with the targeted link's is_dead flag set to true.
        let mut new_links: Vec<EvidenceLink> = Vec::new(env);
        for i in 0..links.len() {
            let link = links.get(i).unwrap();
            if i as u32 == link_index {
                new_links.push_back(EvidenceLink {
                    url: link.url,
                    is_dead: true,
                });
            } else {
                new_links.push_back(link);
            }
        }

        Self::store_evidence_links(env, project_id, &reviewer, &new_links);
        Ok(())
    }

    /// Record the first-interaction timestamp for an address if not yet set.
    /// Called automatically whenever an address performs an action that should
    /// count toward the "minimum account age" eligibility check.
    pub fn record_first_interaction(env: &Env, address: &Address) {
        let key = ExtensionKey2::FirstInteraction(address.clone());
        if !env.storage().persistent().has(&key) {
            let now = env.ledger().timestamp();
            env.storage().persistent().set(&key, &now);
        }
    }

    /// Check whether `reviewer` is eligible to review `project_id` according
    /// to the currently configured eligibility rules.
    ///
    /// # Errors
    /// - `ContractError::ReviewerNotEligible` if any constraint is not satisfied.
    pub fn check_review_eligibility(
        env: &Env,
        project_id: u64,
        reviewer: &Address,
    ) -> Result<(), ContractError> {
        let config = Self::get_review_eligibility_config(env);

        // 1. Minimum account age check
        if config.min_reviewer_age_seconds > 0 {
            let first_interaction: u64 = env
                .storage()
                .persistent()
                .get(&ExtensionKey2::FirstInteraction(reviewer.clone()))
                .unwrap_or(0);
            let now = env.ledger().timestamp();
            if first_interaction == 0
                || now.saturating_sub(first_interaction) < config.min_reviewer_age_seconds
            {
                return Err(ContractError::ReviewerNotEligible);
            }
        }

        // 2. Endorsement requirement check
        if config.require_endorsement
            && !crate::endorsement_registry::EndorsementRegistry::has_endorsed(
                env, project_id, reviewer,
            )
        {
            return Err(ContractError::ReviewerNotEligible);
        }

        // 3. Review fee check
        if config.review_fee > 0 && !crate::fee_manager::FeeManager::is_fee_paid(env, project_id) {
            return Err(ContractError::ReviewFeeRequired);
        }

        Ok(())
    }

    pub fn add_review(
        env: &Env,
        project_id: u64,
        reviewer: Address,
        rating: u32,
        comment_cid: Option<String>,
        evidence_links: Option<soroban_sdk::Vec<EvidenceLink>>,
    ) -> Result<(), ContractError> {
        if let Some(cid) = comment_cid.as_ref() {
            ReviewValidation::validate_review_cid(cid)?;
        }

        // Resolve evidence links — None means empty list
        let resolved_links = evidence_links.unwrap_or_else(|| Vec::new(env));

        // Validate evidence links before any mutations
        ReviewValidation::validate_evidence_links(&resolved_links)?;

        // Validation phase
        reviewer.require_auth();

        // Check if project exists
        let project = match ProjectRegistry::get_project(env, project_id) {
            Some(p) => p,
            None => return Err(ContractError::ProjectNotFound),
        };

        // Nobody who controls the project may review it (issue #478):
        // owners and maintainers alike.
        ReviewValidation::ensure_can_review(env, project_id, &project, &reviewer)?;

        // Check if reviews are enabled for this project
        if !Self::get_reviews_enabled(env, project_id) {
            return Err(ContractError::ReviewsDisabled);
        }

        ReviewValidation::validate_rating(rating)?;

        // Anti-sybil eligibility check
        Self::check_review_eligibility(env, project_id, &reviewer)?;

        let review_key = StorageKey::Review(project_id, reviewer.clone());
        if env.storage().persistent().has(&review_key) {
            return Err(ContractError::DuplicateReview);
        }

        let user_reviews: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::UserReviews(reviewer.clone()))
            .unwrap_or_else(|| Vec::new(env));
        let project_reviews: Vec<Address> = env
            .storage()
            .persistent()
            .get(&StorageKey::ProjectReviews(project_id))
            .unwrap_or_else(|| Vec::new(env));

        if project_reviews.len() >= ConfigRegistry::get_max_reviews_per_project(env) {
            return Err(ContractError::MaxProjectsExceeded);
        }
        if user_reviews.len() >= MAX_REVIEWS_PER_USER {
            return Err(ContractError::MaxProjectsExceeded);
        }

        // Record first interaction for account-age tracking
        Self::record_first_interaction(env, &reviewer);

        // Mutation phase
        let now = env.ledger().timestamp();
        let review = Review {
            project_id,
            reviewer: reviewer.clone(),
            rating,
            content_cid: comment_cid.clone(),
            owner_response: None,
            created_at: now,
            updated_at: now,
            last_updated_at: 0,
            hidden: false,
            report_count: 0,
            evidence_links: Vec::new(env),
        };

        // Get current state for mutations
        let mut user_reviews = user_reviews;
        let mut project_reviews = project_reviews;
        let stats: ProjectStats = env
            .storage()
            .persistent()
            .get(&StorageKey::ProjectStats(project_id))
            .unwrap_or(ProjectStats {
                rating_sum: 0,
                review_count: 0,
                average_rating: 0,
            });

        // Calculate new stats
        let (new_sum, new_count, new_avg) =
            RatingCalculator::add_rating(stats.rating_sum, stats.review_count, rating);

        // Perform all storage mutations
        env.storage().persistent().set(&review_key, &review);

        // Write integrity seal for tamper detection (#809).
        Self::store_review_integrity_seal(env, project_id, &reviewer, rating, &comment_cid);

        user_reviews.push_back(project_id);
        env.storage()
            .persistent()
            .set(&StorageKey::UserReviews(reviewer.clone()), &user_reviews);

        project_reviews.push_back(reviewer.clone());
        env.storage()
            .persistent()
            .set(&StorageKey::ProjectReviews(project_id), &project_reviews);

        env.storage().persistent().set(
            &StorageKey::ProjectStats(project_id),
            &ProjectStats {
                rating_sum: new_sum,
                review_count: new_count,
                average_rating: new_avg,
            },
        );

        // Extend TTL for review-related data
        StorageManager::extend_review_ttl(env, project_id, &reviewer);
        StorageManager::extend_user_reviews_ttl(env, &reviewer);
        StorageManager::extend_project_reviews_ttl(env, project_id);
        StorageManager::extend_project_stats_ttl(env, project_id);

        let evidence_links = Self::load_evidence_links(env, project_id, &reviewer);
        publish_review_event(
            env,
            project_id,
            reviewer,
            ReviewAction::Submitted,
            comment_cid.clone(),
            None,
            now,
            now,
            evidence_links,
        );
        Ok(())
    }

    pub fn submit_review(
        env: &Env,
        project_id: u64,
        reviewer: Address,
        rating: u32,
        review_cid: String,
    ) -> Result<(), ContractError> {
        ReviewValidation::validate_review_cid(&review_cid)?;
        Self::add_review(env, project_id, reviewer, rating, Some(review_cid), None)
    }

    pub fn update_review(
        env: &Env,
        project_id: u64,
        reviewer: Address,
        rating: u32,
        comment_cid: Option<String>,
    ) -> Result<(), ContractError> {
        if let Some(cid) = comment_cid.as_ref() {
            ReviewValidation::validate_review_cid(cid)?;
        }

        // Validation phase
        reviewer.require_auth();

        // Check if project exists
        let project = match ProjectRegistry::get_project(env, project_id) {
            Some(p) => p,
            None => return Err(ContractError::ProjectNotFound),
        };

        // Issue #478: re-check on update, not just on create. A reviewer who
        // has since become the owner or a maintainer must not be able to keep
        // editing their own rating.
        ReviewValidation::ensure_can_review(env, project_id, &project, &reviewer)?;

        ReviewValidation::validate_rating(rating)?;

        let review_key = StorageKey::Review(project_id, reviewer.clone());
        let mut review: Review = env
            .storage()
            .persistent()
            .get(&review_key)
            .ok_or(ContractError::ReviewNotFound)?;

        if review.reviewer != reviewer {
            return Err(ContractError::NotReviewOwner);
        }

        // Cooldown: reject update if within the cooldown window of the last update.
        let now_ts = env.ledger().timestamp();
        if review.last_updated_at > 0
            && now_ts.saturating_sub(review.last_updated_at) < REVIEW_UPDATE_COOLDOWN_SECONDS
        {
            return Err(ContractError::InvalidStatus);
        }

        // Mutation phase — archive prior revision before applying changes
        let old_rating = review.rating;
        let old_content_cid = review.content_cid.clone();
        let now = env.ledger().timestamp();
        let revision_index = Self::append_review_revision(
            env,
            project_id,
            &reviewer,
            old_rating,
            old_content_cid.clone(),
            now,
        );

        review.rating = rating;
        review.content_cid = comment_cid.clone();
        review.updated_at = now;
        review.last_updated_at = now;

        // Get current stats
        let stats: ProjectStats = env
            .storage()
            .persistent()
            .get(&StorageKey::ProjectStats(project_id))
            .unwrap_or(ProjectStats {
                rating_sum: 0,
                review_count: 0,
                average_rating: 0,
            });

        // Calculate new stats
        let (new_sum, _new_count, new_avg) = RatingCalculator::update_rating(
            stats.rating_sum,
            stats.review_count,
            old_rating,
            rating,
        );

        // Perform mutations
        env.storage().persistent().set(&review_key, &review);

        // Update integrity seal to reflect the new content (#809).
        Self::store_review_integrity_seal(env, project_id, &reviewer, rating, &comment_cid);
        env.storage().persistent().set(
            &StorageKey::ProjectStats(project_id),
            &ProjectStats {
                rating_sum: new_sum,
                review_count: stats.review_count,
                average_rating: new_avg,
            },
        );

        publish_review_event(
            env,
            project_id,
            reviewer.clone(),
            ReviewAction::Updated,
            comment_cid.clone(),
            review.owner_response.clone(),
            review.created_at,
            now,
            Self::load_evidence_links(env, project_id, &reviewer),
        );

        publish_review_revision_event(
            env,
            project_id,
            reviewer,
            revision_index,
            old_rating,
            old_content_cid,
            rating,
            comment_cid,
        );
        Ok(())
    }

    fn append_review_revision(
        env: &Env,
        project_id: u64,
        reviewer: &Address,
        rating: u32,
        content_cid: Option<String>,
        revised_at: u64,
    ) -> u32 {
        let count_key = ExtensionKey::ReviewRevisionCount(project_id, reviewer.clone());
        let revision_count: u32 = env.storage().persistent().get(&count_key).unwrap_or(0);

        if revision_count < MAX_REVIEW_REVISIONS {
            env.storage().persistent().set(
                &ExtensionKey::ReviewRevision(project_id, reviewer.clone(), revision_count),
                &ReviewRevision {
                    revision_index: revision_count,
                    rating,
                    content_cid,
                    revised_at,
                },
            );
            let new_count = revision_count.saturating_add(1);
            env.storage().persistent().set(&count_key, &new_count);
            revision_count
        } else {
            let start_idx = revision_count.saturating_sub(MAX_REVIEW_REVISIONS - 1);
            for j in 0..(MAX_REVIEW_REVISIONS - 1) {
                let src_idx = start_idx + j;
                let from_key = ExtensionKey::ReviewRevision(project_id, reviewer.clone(), src_idx);
                if let Some(mut rev) = env
                    .storage()
                    .persistent()
                    .get::<_, ReviewRevision>(&from_key)
                {
                    rev.revision_index = j;
                    let to_key = ExtensionKey::ReviewRevision(project_id, reviewer.clone(), j);
                    env.storage().persistent().set(&to_key, &rev);
                }
            }

            let new_index = MAX_REVIEW_REVISIONS - 1;
            env.storage().persistent().set(
                &ExtensionKey::ReviewRevision(project_id, reviewer.clone(), new_index),
                &ReviewRevision {
                    revision_index: new_index,
                    rating,
                    content_cid,
                    revised_at,
                },
            );
            env.storage()
                .persistent()
                .set(&count_key, &MAX_REVIEW_REVISIONS);

            if revision_count > MAX_REVIEW_REVISIONS {
                for i in MAX_REVIEW_REVISIONS..revision_count {
                    env.storage()
                        .persistent()
                        .remove(&ExtensionKey::ReviewRevision(
                            project_id,
                            reviewer.clone(),
                            i,
                        ));
                }
            }

            new_index
        }
    }

    pub fn get_review_revision_count(env: &Env, project_id: u64, reviewer: Address) -> u32 {
        env.storage()
            .persistent()
            .get(&ExtensionKey::ReviewRevisionCount(project_id, reviewer))
            .unwrap_or(0)
    }

    /// Clear all stored revision entries and the revision count for a reviewer.
    ///
    /// Called during review deletion so that `get_review_revision_count` returns 0
    /// and `get_review_history` returns an empty list after a review is removed.
    /// This prevents stale revision data from persisting beyond the review's lifetime.
    fn clear_review_revisions(env: &Env, project_id: u64, reviewer: &Address) {
        let count_key = ExtensionKey::ReviewRevisionCount(project_id, reviewer.clone());
        let revision_count: u32 = env.storage().persistent().get(&count_key).unwrap_or(0);

        // Remove each stored revision entry (indices are always 0..count after pruning)
        for i in 0..revision_count {
            env.storage()
                .persistent()
                .remove(&ExtensionKey::ReviewRevision(
                    project_id,
                    reviewer.clone(),
                    i,
                ));
        }

        // Remove the count key itself
        if revision_count > 0 {
            env.storage().persistent().remove(&count_key);
        }
    }

    /// Returns prior review revisions in ascending order (oldest revision first).
    pub fn get_review_history(
        env: &Env,
        project_id: u64,
        reviewer: Address,
        start_index: u32,
        limit: u32,
    ) -> Vec<ReviewRevision> {
        let effective_limit = if limit == 0 || limit > MAX_PAGE_LIMIT {
            MAX_PAGE_LIMIT
        } else {
            limit
        };

        let total = Self::get_review_revision_count(env, project_id, reviewer.clone());
        let mut history = Vec::new(env);
        if start_index >= total {
            return history;
        }

        let end = core::cmp::min(start_index.saturating_add(effective_limit), total);
        for i in start_index..end {
            if let Some(revision) = env
                .storage()
                .persistent()
                .get(&ExtensionKey::ReviewRevision(
                    project_id,
                    reviewer.clone(),
                    i,
                ))
            {
                history.push_back(revision);
            }
        }
        history
    }

    /// Bayesian weighted rating for a project (scaled by 100). Uses O(1) aggregate stats.
    pub fn get_weighted_rating(env: &Env, project_id: u64) -> u32 {
        let stats = Self::get_project_stats(env, project_id);
        RatingCalculator::calculate_weighted(stats.rating_sum, stats.review_count)
    }

    pub fn delete_review(
        env: &Env,
        project_id: u64,
        reviewer: Address,
    ) -> Result<(), ContractError> {
        // Validation phase
        reviewer.require_auth();

        // Check if project exists
        if ProjectRegistry::get_project(env, project_id).is_none() {
            return Err(ContractError::ProjectNotFound);
        }

        let review_key = StorageKey::Review(project_id, reviewer.clone());
        let existing: Review = env
            .storage()
            .persistent()
            .get(&review_key)
            .ok_or(ContractError::ReviewNotFound)?;

        if existing.reviewer != reviewer {
            return Err(ContractError::NotReviewOwner);
        }

        // Mutation phase
        // Get current data
        let stats: ProjectStats = env
            .storage()
            .persistent()
            .get(&StorageKey::ProjectStats(project_id))
            .unwrap_or(ProjectStats {
                rating_sum: 0,
                review_count: 0,
                average_rating: 0,
            });
        let user_reviews: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::UserReviews(reviewer.clone()))
            .unwrap_or_else(|| Vec::new(env));
        let project_reviews: Vec<Address> = env
            .storage()
            .persistent()
            .get(&StorageKey::ProjectReviews(project_id))
            .unwrap_or_else(|| Vec::new(env));

        // Calculate new stats
        let (new_sum, new_count, new_avg) = if stats.review_count > 0 {
            RatingCalculator::remove_rating(stats.rating_sum, stats.review_count, existing.rating)
        } else {
            (stats.rating_sum, stats.review_count, stats.average_rating)
        };

        // Create new user reviews list
        let mut new_user_reviews = Vec::new(env);
        for i in 0..user_reviews.len() {
            if let Some(id) = user_reviews.get(i) {
                if id != project_id {
                    new_user_reviews.push_back(id);
                }
            }
        }

        // Create new project reviews list
        let mut new_project_reviews = Vec::new(env);
        for i in 0..project_reviews.len() {
            if let Some(addr) = project_reviews.get(i) {
                if addr != reviewer {
                    new_project_reviews.push_back(addr);
                }
            }
        }

        // Perform all mutations
        env.storage().persistent().remove(&review_key);
        // Clear revision history so deleted reviews leave no stale history entries.
        Self::clear_review_revisions(env, project_id, &reviewer);
        // Drop stored evidence links so deleted reviews leave no orphaned data.
        Self::delete_evidence_links(env, project_id, &reviewer);
        // Store a tombstone so indexers can distinguish deleted vs never-existed.
        let now = env.ledger().timestamp();
        env.storage().persistent().set(
            &ExtensionKey::ReviewTombstone(project_id, reviewer.clone()),
            &ReviewTombstone {
                project_id,
                reviewer: reviewer.clone(),
                deleted_at: now,
            },
        );
        env.storage().persistent().set(
            &StorageKey::ProjectStats(project_id),
            &ProjectStats {
                rating_sum: new_sum,
                review_count: new_count,
                average_rating: new_avg,
            },
        );
        env.storage().persistent().set(
            &StorageKey::UserReviews(reviewer.clone()),
            &new_user_reviews,
        );
        env.storage().persistent().set(
            &StorageKey::ProjectReviews(project_id),
            &new_project_reviews,
        );

        // Clean up any ReviewReport dedup keys for this review so that
        // the storage doesn't accumulate dangling report entries after deletion.
        // We iterate up to the stored report_count to remove known reporter keys.
        // Since we don't store the list of reporters separately, we just remove
        // the review's own report_count worth of possible keys by clearing the
        // report_count marker — the actual per-reporter keys will expire via TTL.
        // For immediate consistency we remove the count itself from the review record
        // (already done by removing the review_key above).
        // No separate cleanup needed beyond removing the review record itself,
        // as ReviewReport keys are dedup guards keyed by (project_id, reviewer, reporter)
        // and those will become orphaned but harmless once the review is gone.

        // Evidence links for a deleted review are dropped from primary storage
        // along with the review, so the tombstoning event carries none.
        publish_review_event(
            env,
            project_id,
            reviewer,
            ReviewAction::Deleted,
            None,
            existing.owner_response.clone(),
            existing.created_at,
            now,
            Vec::new(env),
        );
        Ok(())
    }

    /// Admin hard-delete a review. Admins can permanently remove any review,
    /// updating stats and indexes just like a reviewer-initiated delete.
    pub fn admin_delete_review(
        env: &Env,
        project_id: u64,
        reviewer: Address,
        admin: Address,
    ) -> Result<(), ContractError> {
        // Validation phase
        admin.require_auth();

        // Admin check
        if !crate::admin_manager::AdminManager::is_admin(env, &admin) {
            return Err(ContractError::AdminOnly);
        }

        // Check if project exists
        if ProjectRegistry::get_project(env, project_id).is_none() {
            return Err(ContractError::ProjectNotFound);
        }

        let review_key = StorageKey::Review(project_id, reviewer.clone());
        let existing: Review = env
            .storage()
            .persistent()
            .get(&review_key)
            .ok_or(ContractError::ReviewNotFound)?;

        // Mutation phase — same index/stats cleanup as delete_review
        let stats: ProjectStats = env
            .storage()
            .persistent()
            .get(&StorageKey::ProjectStats(project_id))
            .unwrap_or(ProjectStats {
                rating_sum: 0,
                review_count: 0,
                average_rating: 0,
            });
        let user_reviews: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::UserReviews(reviewer.clone()))
            .unwrap_or_else(|| Vec::new(env));
        let project_reviews: Vec<Address> = env
            .storage()
            .persistent()
            .get(&StorageKey::ProjectReviews(project_id))
            .unwrap_or_else(|| Vec::new(env));

        // Recalculate stats — exclude hidden reviews that were already excluded
        let (new_sum, new_count, new_avg) = if stats.review_count > 0 && !existing.hidden {
            RatingCalculator::remove_rating(stats.rating_sum, stats.review_count, existing.rating)
        } else {
            (stats.rating_sum, stats.review_count, stats.average_rating)
        };

        // Rebuild user reviews list without this project
        let mut new_user_reviews = Vec::new(env);
        for i in 0..user_reviews.len() {
            if let Some(id) = user_reviews.get(i) {
                if id != project_id {
                    new_user_reviews.push_back(id);
                }
            }
        }

        // Rebuild project reviews list without this reviewer
        let mut new_project_reviews = Vec::new(env);
        for i in 0..project_reviews.len() {
            if let Some(addr) = project_reviews.get(i) {
                if addr != reviewer {
                    new_project_reviews.push_back(addr);
                }
            }
        }

        // Apply all mutations
        env.storage().persistent().remove(&review_key);
        // Clear revision history so deleted reviews leave no stale history entries.
        Self::clear_review_revisions(env, project_id, &reviewer);
        // Drop stored evidence links so deleted reviews leave no orphaned data.
        Self::delete_evidence_links(env, project_id, &reviewer);
        // Store a tombstone so indexers can distinguish deleted vs never-existed.
        let now = env.ledger().timestamp();
        env.storage().persistent().set(
            &ExtensionKey::ReviewTombstone(project_id, reviewer.clone()),
            &ReviewTombstone {
                project_id,
                reviewer: reviewer.clone(),
                deleted_at: now,
            },
        );
        env.storage().persistent().set(
            &StorageKey::ProjectStats(project_id),
            &ProjectStats {
                rating_sum: new_sum,
                review_count: new_count,
                average_rating: new_avg,
            },
        );
        env.storage().persistent().set(
            &StorageKey::UserReviews(reviewer.clone()),
            &new_user_reviews,
        );
        env.storage().persistent().set(
            &StorageKey::ProjectReviews(project_id),
            &new_project_reviews,
        );

        crate::events::publish_review_deleted_by_admin_event(
            env,
            project_id,
            reviewer.clone(),
            admin.clone(),
        );

        AdminActionLog::record_action(
            env,
            admin,
            AdminActionType::ReviewDeletedByAdmin,
            Some(project_id),
            Some(reviewer),
            None,
        );

        Ok(())
    }

    pub fn get_reviews_by_ids(env: &Env, ids: Vec<(u64, Address)>) -> Vec<Review> {
        let mut reviews = Vec::new(env);
        let len = ids.len();
        for i in 0..len {
            if let Some((project_id, reviewer)) = ids.get(i) {
                if let Some(review) = Self::get_review(env, project_id, reviewer) {
                    // Exclude hidden reviews from bulk listing (issue #658).
                    // Direct per-reviewer lookups via `get_review` still return
                    // the full record so admins can inspect hidden reviews.
                    if !review.hidden {
                        reviews.push_back(review);
                    }
                }
            }
        }
        reviews
    }

    pub fn respond_to_review(
        env: &Env,
        project_id: u64,
        caller: Address,
        reviewer: Address,
        response: String,
    ) -> Result<(), ContractError> {
        // Validation phase
        caller.require_auth();

        let project: Project = env
            .storage()
            .persistent()
            .get(&StorageKey::Project(project_id))
            .ok_or(ContractError::ProjectNotFound)?;

        if project.owner != caller {
            return Err(ContractError::Unauthorized);
        }

        let review_key = StorageKey::Review(project_id, reviewer.clone());
        let mut review: Review = env
            .storage()
            .persistent()
            .get(&review_key)
            .ok_or(ContractError::ReviewNotFound)?;

        // Mutation phase
        let now = env.ledger().timestamp();
        review.owner_response = Some(response);
        review.updated_at = now;

        env.storage().persistent().set(&review_key, &review);

        let evidence_links = Self::load_evidence_links(env, project_id, &reviewer);
        publish_review_event(
            env,
            project_id,
            reviewer,
            ReviewAction::Updated,
            review.content_cid.clone(),
            review.owner_response.clone(),
            review.created_at,
            now,
            evidence_links,
        );
        Ok(())
    }

    pub fn get_review_response(env: &Env, project_id: u64, reviewer: Address) -> Option<String> {
        Self::get_review(env, project_id, reviewer).and_then(|review| review.owner_response)
    }

    pub fn get_review(env: &Env, project_id: u64, reviewer: Address) -> Option<Review> {
        env.storage()
            .persistent()
            .get(&StorageKey::Review(project_id, reviewer))
    }

    pub fn get_review_cid(env: &Env, project_id: u64, reviewer: Address) -> Option<String> {
        Self::get_review(env, project_id, reviewer).and_then(|review| {
            // Return None for hidden reviews — callers should not get CIDs for
            // moderated content (issue #658). Admins needing the CID of a hidden
            // review can call get_review directly.
            if review.hidden {
                None
            } else {
                review.content_cid
            }
        })
    }

    pub fn get_project_review_cids(env: &Env, project_id: u64) -> Vec<(Address, String)> {
        let reviewers: Vec<Address> = env
            .storage()
            .persistent()
            .get(&StorageKey::ProjectReviews(project_id))
            .unwrap_or_else(|| Vec::new(env));

        let mut cids = Vec::new(env);
        let len = reviewers.len();
        for i in 0..len {
            if let Some(reviewer) = reviewers.get(i) {
                // Only include CIDs for non-hidden reviews (issue #658).
                if let Some(review) = Self::get_review(env, project_id, reviewer.clone()) {
                    if !review.hidden {
                        if let Some(cid) = review.content_cid {
                            cids.push_back((reviewer, cid));
                        }
                    }
                }
            }
        }
        cids
    }

    pub fn get_project_stats(env: &Env, project_id: u64) -> ProjectStats {
        env.storage()
            .persistent()
            .get(&StorageKey::ProjectStats(project_id))
            .unwrap_or(ProjectStats {
                rating_sum: 0,
                review_count: 0,
                average_rating: 0,
            })
    }

    /// Batch-fetch stats for multiple project IDs. Returns one entry per ID (defaults to zero stats
    /// for projects with no reviews). Clamped to MAX_PAGE_LIMIT entries.
    pub fn get_stats_batch(env: &Env, ids: Vec<u64>) -> Vec<(u64, ProjectStats)> {
        let len = core::cmp::min(ids.len(), MAX_PAGE_LIMIT);
        let mut out = Vec::new(env);
        for i in 0..len {
            if let Some(id) = ids.get(i) {
                out.push_back((id, Self::get_project_stats(env, id)));
            }
        }
        out
    }

    pub fn list_reviews(env: &Env, project_id: u64, start_index: u32, limit: u32) -> Vec<Review> {
        // Enforce pagination limits: limit must be 1..=MAX_PAGE_LIMIT
        let effective_limit = if limit == 0 || limit > MAX_PAGE_LIMIT {
            MAX_PAGE_LIMIT
        } else {
            limit
        };

        let reviewers: Vec<Address> = env
            .storage()
            .persistent()
            .get(&StorageKey::ProjectReviews(project_id))
            .unwrap_or_else(|| Vec::new(env));

        let mut reviews = Vec::new(env);
        let len = reviewers.len();
        if start_index >= len {
            return reviews;
        }
        let end = core::cmp::min(start_index.saturating_add(effective_limit), len);

        for i in start_index..end {
            if let Some(reviewer) = reviewers.get(i) {
                if let Some(review) = Self::get_review(env, project_id, reviewer) {
                    // Exclude hidden reviews from default listings
                    if !review.hidden {
                        reviews.push_back(review);
                    }
                }
            }
        }
        reviews
    }

    /// Enable or disable reviews for a project. Only the project owner may call this.
    pub fn set_reviews_enabled(
        env: &Env,
        project_id: u64,
        caller: Address,
        enabled: bool,
    ) -> Result<(), ContractError> {
        caller.require_auth();

        let project: Project = env
            .storage()
            .persistent()
            .get(&StorageKey::Project(project_id))
            .ok_or(ContractError::ProjectNotFound)?;

        if project.owner != caller {
            return Err(ContractError::Unauthorized);
        }

        env.storage()
            .persistent()
            .set(&StorageKey::ReviewsEnabled(project_id), &enabled);

        crate::events::publish_project_reviews_enabled_set_event(env, project_id, caller, enabled);

        Ok(())
    }

    /// Returns whether reviews are enabled for a project. Defaults to `true` if never set.
    pub fn get_reviews_enabled(env: &Env, project_id: u64) -> bool {
        env.storage()
            .persistent()
            .get(&StorageKey::ReviewsEnabled(project_id))
            .unwrap_or(true)
    }

    pub fn report_review(
        env: &Env,
        project_id: u64,
        reviewer: Address,
        reporter: Address,
    ) -> Result<(), ContractError> {
        // Validation phase
        reporter.require_auth();

        // Check if project exists
        if ProjectRegistry::get_project(env, project_id).is_none() {
            return Err(ContractError::ProjectNotFound);
        }

        let review_key = StorageKey::Review(project_id, reviewer.clone());
        let mut review: Review = env
            .storage()
            .persistent()
            .get(&review_key)
            .ok_or(ContractError::ReviewNotFound)?;

        // Check if reporter has already reported this review
        let report_key = StorageKey::ReviewReport(project_id, reviewer.clone(), reporter.clone());
        if env.storage().persistent().has(&report_key) {
            return Err(ContractError::AlreadyReported);
        }

        // Mutation phase
        review.report_count = review.report_count.saturating_add(1);
        env.storage().persistent().set(&review_key, &review);

        // Track this report
        env.storage().persistent().set(&report_key, &true);

        // Extend TTL
        StorageManager::extend_review_ttl(env, project_id, &reviewer);

        crate::events::publish_review_reported_event(env, project_id, reviewer, reporter);

        Ok(())
    }

    pub fn hide_review(
        env: &Env,
        project_id: u64,
        reviewer: Address,
        admin: Address,
    ) -> Result<(), ContractError> {
        // Validation phase
        admin.require_auth();

        // Check if admin
        if !crate::admin_manager::AdminManager::is_admin(env, &admin) {
            return Err(ContractError::AdminOnly);
        }

        // Check if project exists
        if ProjectRegistry::get_project(env, project_id).is_none() {
            return Err(ContractError::ProjectNotFound);
        }

        let review_key = StorageKey::Review(project_id, reviewer.clone());
        let mut review: Review = env
            .storage()
            .persistent()
            .get(&review_key)
            .ok_or(ContractError::ReviewNotFound)?;

        if review.hidden {
            return Err(ContractError::ReviewAlreadyHidden);
        }

        // Mutation phase
        review.hidden = true;
        env.storage().persistent().set(&review_key, &review);

        // Update project stats to exclude this review
        let stats: ProjectStats = env
            .storage()
            .persistent()
            .get(&StorageKey::ProjectStats(project_id))
            .unwrap_or(ProjectStats {
                rating_sum: 0,
                review_count: 0,
                average_rating: 0,
            });

        // Recalculate stats without this review
        let (new_sum, new_count, new_avg) = if stats.review_count > 0 {
            RatingCalculator::remove_rating(stats.rating_sum, stats.review_count, review.rating)
        } else {
            (stats.rating_sum, stats.review_count, stats.average_rating)
        };

        env.storage().persistent().set(
            &StorageKey::ProjectStats(project_id),
            &ProjectStats {
                rating_sum: new_sum,
                review_count: new_count,
                average_rating: new_avg,
            },
        );

        // Extend TTL
        StorageManager::extend_review_ttl(env, project_id, &reviewer);
        StorageManager::extend_project_stats_ttl(env, project_id);

        crate::events::publish_review_hidden_event(
            env,
            project_id,
            reviewer.clone(),
            admin.clone(),
        );

        AdminActionLog::record_action(
            env,
            admin,
            AdminActionType::ReviewHidden,
            Some(project_id),
            Some(reviewer),
            None,
        );

        Ok(())
    }

    pub fn restore_review(
        env: &Env,
        project_id: u64,
        reviewer: Address,
        admin: Address,
    ) -> Result<(), ContractError> {
        // Validation phase
        admin.require_auth();

        // Check if admin
        if !crate::admin_manager::AdminManager::is_admin(env, &admin) {
            return Err(ContractError::AdminOnly);
        }

        // Check if project exists
        if ProjectRegistry::get_project(env, project_id).is_none() {
            return Err(ContractError::ProjectNotFound);
        }

        let review_key = StorageKey::Review(project_id, reviewer.clone());
        let mut review: Review = env
            .storage()
            .persistent()
            .get(&review_key)
            .ok_or(ContractError::ReviewNotFound)?;

        if !review.hidden {
            return Err(ContractError::ReviewNotHidden);
        }

        // Mutation phase
        review.hidden = false;
        env.storage().persistent().set(&review_key, &review);

        // Update project stats to include this review again
        let stats: ProjectStats = env
            .storage()
            .persistent()
            .get(&StorageKey::ProjectStats(project_id))
            .unwrap_or(ProjectStats {
                rating_sum: 0,
                review_count: 0,
                average_rating: 0,
            });

        // Recalculate stats with this review
        let (new_sum, new_count, new_avg) =
            RatingCalculator::add_rating(stats.rating_sum, stats.review_count, review.rating);

        env.storage().persistent().set(
            &StorageKey::ProjectStats(project_id),
            &ProjectStats {
                rating_sum: new_sum,
                review_count: new_count,
                average_rating: new_avg,
            },
        );

        // Extend TTL
        StorageManager::extend_review_ttl(env, project_id, &reviewer);
        StorageManager::extend_project_stats_ttl(env, project_id);

        crate::events::publish_review_restored_event(
            env,
            project_id,
            reviewer.clone(),
            admin.clone(),
        );

        AdminActionLog::record_action(
            env,
            admin,
            AdminActionType::ReviewRestored,
            Some(project_id),
            Some(reviewer),
            None,
        );

        Ok(())
    }

    /// Retrieve the deletion tombstone for a review, if one exists.
    /// Returns `Some` when the review was deleted; `None` when it never existed.
    pub fn get_review_tombstone(
        env: &Env,
        project_id: u64,
        reviewer: Address,
    ) -> Option<ReviewTombstone> {
        env.storage()
            .persistent()
            .get(&ExtensionKey::ReviewTombstone(project_id, reviewer))
    }

    /// List a bounded review page for client-side sorting.
    ///
    /// `sort_mode` is retained for ABI compatibility. Sorting must be performed
    /// by the caller after fetching pages with `list_reviews` semantics.
    pub fn list_reviews_sorted(
        env: &Env,
        project_id: u64,
        start_index: u32,
        limit: u32,
        _sort_mode: ReviewSortMode,
    ) -> Vec<Review> {
        Self::list_reviews(env, project_id, start_index, limit)
    }

    // ── Review content integrity (#809) ─────────────────────────────────────

    /// Canonical payload format for review integrity seals (v1).
    ///
    /// The payload is:
    ///   `review-integrity-v1|<project_id_be8>|<reviewer_xdr>|<rating_be4>|<content_cid_or_NONE>`
    ///
    /// Using `project_id` and reviewer address bytes makes every seal unique
    /// across reviews; including `rating` and `content_cid` means any change
    /// to either field changes the hash.
    fn compute_review_integrity_hash(
        env: &Env,
        project_id: u64,
        reviewer: &Address,
        rating: u32,
        content_cid: &Option<soroban_sdk::String>,
    ) -> soroban_sdk::Bytes {
        let mut buf = soroban_sdk::Bytes::new(env);

        // Version prefix
        for b in b"review-integrity-v1|" {
            buf.push_back(*b);
        }

        // project_id as 8 big-endian bytes
        let pid_bytes = project_id.to_be_bytes();
        for b in &pid_bytes {
            buf.push_back(*b);
        }
        buf.push_back(b'|');

        // reviewer address XDR bytes
        let reviewer_bytes = reviewer.to_xdr(env);
        let rlen = reviewer_bytes.len();
        for i in 0..rlen {
            buf.push_back(reviewer_bytes.get(i).unwrap_or(0));
        }
        buf.push_back(b'|');

        // rating as 4 big-endian bytes
        let rating_bytes = rating.to_be_bytes();
        for b in &rating_bytes {
            buf.push_back(*b);
        }
        buf.push_back(b'|');

        // content_cid or sentinel "NONE"
        match content_cid {
            Some(ref cid) => {
                let cid_bytes = cid.clone().to_xdr(env);
                // XDR-encoded String has a 4-byte length prefix; skip it.
                let xdr_len = cid_bytes.len();
                let cid_char_len = cid.len();
                let skip = xdr_len.saturating_sub(cid_char_len);
                for i in skip..xdr_len {
                    buf.push_back(cid_bytes.get(i).unwrap_or(0));
                }
            }
            None => {
                for b in b"NONE" {
                    buf.push_back(*b);
                }
            }
        }

        let hash = env.crypto().sha256(&buf);
        soroban_sdk::Bytes::from_array(env, &hash.to_array())
    }

    /// Compute and persist the integrity seal for a review.
    ///
    /// Called automatically on every `add_review` and `update_review`.
    /// Emits `ReviewIntegritySealedEvent` so indexers can track seal history.
    fn store_review_integrity_seal(
        env: &Env,
        project_id: u64,
        reviewer: &Address,
        rating: u32,
        content_cid: &Option<soroban_sdk::String>,
    ) {
        let hash =
            Self::compute_review_integrity_hash(env, project_id, reviewer, rating, content_cid);
        let record = ReviewIntegrityRecord {
            integrity_hash: hash,
            sealed_at: env.ledger().timestamp(),
            sealed_content_cid: content_cid.clone(),
            sealed_rating: rating,
        };
        env.storage().persistent().set(
            &ReviewIntegrityKey::ReviewIntegrityHash(project_id, reviewer.clone()),
            &record,
        );
        StorageManager::extend_review_integrity_seal_ttl(env, project_id, reviewer);

        publish_review_integrity_sealed_event(
            env,
            project_id,
            reviewer.clone(),
            rating,
            content_cid.is_some(),
        );
    }

    /// Return the stored integrity seal for a review, if any.
    pub fn get_review_integrity_record(
        env: &Env,
        project_id: u64,
        reviewer: Address,
    ) -> Option<ReviewIntegrityRecord> {
        env.storage()
            .persistent()
            .get(&ReviewIntegrityKey::ReviewIntegrityHash(
                project_id, reviewer,
            ))
    }

    /// Verify the content integrity of a stored review.
    ///
    /// Recomputes the SHA-256 seal over the review's current on-chain data and
    /// compares it against the stored seal.
    ///
    /// Returns:
    /// - `ReviewIntegrityStatus::Valid` — hash matches; content is unmodified.
    /// - `ReviewIntegrityStatus::Tampered` — hash mismatch; emits
    ///   `ReviewIntegrityViolationEvent` as a tamper warning.
    /// - `ReviewIntegrityStatus::Unverifiable` — no seal exists for this review
    ///   (submitted before integrity sealing was enabled).
    pub fn verify_review_integrity(
        env: &Env,
        project_id: u64,
        reviewer: Address,
    ) -> ReviewIntegrityStatus {
        let review: Review = match env
            .storage()
            .persistent()
            .get(&StorageKey::Review(project_id, reviewer.clone()))
        {
            Some(r) => r,
            None => return ReviewIntegrityStatus::Unverifiable,
        };

        let record: ReviewIntegrityRecord =
            match env
                .storage()
                .persistent()
                .get(&ReviewIntegrityKey::ReviewIntegrityHash(
                    project_id,
                    reviewer.clone(),
                )) {
                Some(r) => r,
                None => return ReviewIntegrityStatus::Unverifiable,
            };

        let current_hash = Self::compute_review_integrity_hash(
            env,
            project_id,
            &reviewer,
            review.rating,
            &review.content_cid,
        );

        if current_hash == record.integrity_hash {
            ReviewIntegrityStatus::Valid
        } else {
            // Emit tamper-warning event before returning.
            publish_review_integrity_violation_event(
                env,
                project_id,
                reviewer,
                review.rating,
                record.sealed_rating,
                review.content_cid.is_some(),
                record.sealed_content_cid.is_some(),
            );
            ReviewIntegrityStatus::Tampered
        }
    }

    // ── Review Archival (#804) ────────────────────────────────────────────

    /// Admin-only: archive reviews older than `REVIEW_ARCHIVE_AGE_SECONDS` for a project.
    ///
    /// Iterates over the active reviewer list for `project_id`, checking each
    /// review's `created_at` timestamp. Reviews whose age exceeds the threshold
    /// are:
    ///
    /// 1. Moved to a compact `ArchivedReview` record stored at a shorter TTL
    ///    (see `LEDGER_THRESHOLD_ARCHIVED_REVIEW`).
    /// 2. Removed from primary persistent storage (`StorageKey::Review`).
    /// 3. Removed from the `ProjectReviews` and `UserReviews` index so they
    ///    no longer appear in regular listing calls.
    /// 4. Added to the `ProjectArchivedReviews` index so callers can enumerate
    ///    archived reviews via `list_archived_reviews`.
    /// 5. Reported via `ReviewArchivedEvent` for off-chain indexers to persist
    ///    the full payload to permanent storage (e.g., Arweave/IPFS).
    ///
    /// Project stats (`rating_sum`, `review_count`, `average_rating`) are **not**
    /// modified — archived reviews continue to contribute to the aggregate rating.
    ///
    /// `batch_size` caps the number of reviews processed per call
    /// (max `MAX_ARCHIVE_BATCH_SIZE`). Call repeatedly to archive large projects.
    ///
    /// Returns the number of reviews archived in this call.
    ///
    /// # Errors
    /// - `ContractError::AdminOnly` — caller is not a registered admin.
    /// - `ContractError::ProjectNotFound` — `project_id` does not exist.
    pub fn archive_old_reviews(
        env: &Env,
        admin: Address,
        project_id: u64,
        batch_size: u32,
    ) -> Result<u32, ContractError> {
        admin.require_auth();
        if !crate::admin_manager::AdminManager::is_admin(env, &admin) {
            return Err(ContractError::AdminOnly);
        }
        if ProjectRegistry::get_project(env, project_id).is_none() {
            return Err(ContractError::ProjectNotFound);
        }

        let effective_batch = if batch_size == 0 || batch_size > MAX_ARCHIVE_BATCH_SIZE {
            MAX_ARCHIVE_BATCH_SIZE
        } else {
            batch_size
        };

        let now = env.ledger().timestamp();
        let cutoff = now.saturating_sub(REVIEW_ARCHIVE_AGE_SECONDS);

        // Load current active reviewer list for the project.
        let project_reviews: Vec<Address> = env
            .storage()
            .persistent()
            .get(&StorageKey::ProjectReviews(project_id))
            .unwrap_or_else(|| Vec::new(env));

        // Load current archived reviewer list (append-only index).
        let mut archived_reviewers: Vec<Address> = env
            .storage()
            .persistent()
            .get(&ExtensionKey2::ProjectArchivedReviews(project_id))
            .unwrap_or_else(|| Vec::new(env));

        // Track which reviewers remain active after this batch.
        let mut remaining: Vec<Address> = Vec::new(env);
        let mut archived_count: u32 = 0;

        let len = project_reviews.len();
        for i in 0..len {
            let reviewer = match project_reviews.get(i) {
                Some(r) => r,
                None => continue,
            };

            // If we've hit the batch cap, keep everything else active.
            if archived_count >= effective_batch {
                remaining.push_back(reviewer);
                continue;
            }

            let review_key = StorageKey::Review(project_id, reviewer.clone());
            let review: Review = match env.storage().persistent().get(&review_key) {
                Some(r) => r,
                None => continue, // orphaned index entry — skip silently
            };

            // Only archive if the review is old enough and not already archived.
            if review.created_at > cutoff {
                remaining.push_back(reviewer);
                continue;
            }

            // --- Archive this review ---

            // 1. Build and store the compact ArchivedReview record.
            let archived = ArchivedReview {
                project_id,
                reviewer: reviewer.clone(),
                rating: review.rating,
                content_cid: review.content_cid.clone(),
                created_at: review.created_at,
                updated_at: review.updated_at,
                archived_at: now,
                arweave_tx_id: None,
            };
            let archive_key = ExtensionKey2::ArchivedReview(project_id, reviewer.clone());
            env.storage().persistent().set(&archive_key, &archived);
            env.storage().persistent().extend_ttl(
                &archive_key,
                LEDGER_THRESHOLD_ARCHIVED_REVIEW,
                LEDGER_BUMP_ARCHIVED_REVIEW,
            );

            // 2. Remove from primary persistent storage.
            env.storage().persistent().remove(&review_key);
            // Also remove revision history to reclaim storage.
            Self::clear_review_revisions(env, project_id, &reviewer);

            // 3. Remove from the reviewer's own UserReviews index.
            let user_reviews_key = StorageKey::UserReviews(reviewer.clone());
            let user_reviews: Vec<u64> = env
                .storage()
                .persistent()
                .get(&user_reviews_key)
                .unwrap_or_else(|| Vec::new(env));
            let mut new_user_reviews: Vec<u64> = Vec::new(env);
            for j in 0..user_reviews.len() {
                if let Some(pid) = user_reviews.get(j) {
                    if pid != project_id {
                        new_user_reviews.push_back(pid);
                    }
                }
            }
            env.storage()
                .persistent()
                .set(&user_reviews_key, &new_user_reviews);

            // 4. Add to the project archived reviewers index.
            archived_reviewers.push_back(reviewer.clone());

            // 5. Emit archival event for off-chain indexers.
            publish_review_archived_event(
                env,
                project_id,
                reviewer,
                review.rating,
                review.content_cid,
                review.created_at,
                review.updated_at,
            );

            archived_count += 1;
        }

        // Persist the updated active reviewer list.
        env.storage()
            .persistent()
            .set(&StorageKey::ProjectReviews(project_id), &remaining);

        // Persist the updated archived reviewer index.
        if archived_count > 0 {
            let archived_key = ExtensionKey2::ProjectArchivedReviews(project_id);
            env.storage()
                .persistent()
                .set(&archived_key, &archived_reviewers);
            env.storage().persistent().extend_ttl(
                &archived_key,
                LEDGER_THRESHOLD_ARCHIVED_REVIEW,
                LEDGER_BUMP_ARCHIVED_REVIEW,
            );
        }

        Ok(archived_count)
    }

    /// Retrieve the compact archived record for a review that has been processed
    /// by `archive_old_reviews`. Returns `None` if the review was never archived
    /// (or the archived record has expired from on-chain storage).
    ///
    /// Use this instead of `get_review` for reviews known to be archived.
    pub fn get_archived_review(
        env: &Env,
        project_id: u64,
        reviewer: Address,
    ) -> Option<ArchivedReview> {
        env.storage()
            .persistent()
            .get(&ExtensionKey2::ArchivedReview(project_id, reviewer))
    }

    /// Return a paginated list of archived reviews for a project.
    ///
    /// Results are in the order reviews were archived (oldest-archived first).
    /// Use `start_index` / `limit` for pagination.
    pub fn list_archived_reviews(
        env: &Env,
        project_id: u64,
        start_index: u32,
        limit: u32,
    ) -> Vec<ArchivedReview> {
        let effective_limit = if limit == 0 || limit > MAX_PAGE_LIMIT {
            MAX_PAGE_LIMIT
        } else {
            limit
        };

        let reviewers: Vec<Address> = env
            .storage()
            .persistent()
            .get(&ExtensionKey2::ProjectArchivedReviews(project_id))
            .unwrap_or_else(|| Vec::new(env));

        let total = reviewers.len();
        let mut results: Vec<ArchivedReview> = Vec::new(env);
        if start_index >= total {
            return results;
        }

        let end = core::cmp::min(start_index.saturating_add(effective_limit), total);
        for i in start_index..end {
            if let Some(reviewer) = reviewers.get(i) {
                if let Some(archived) = env
                    .storage()
                    .persistent()
                    .get(&ExtensionKey2::ArchivedReview(project_id, reviewer))
                {
                    results.push_back(archived);
                }
            }
        }
        results
    }

    /// Admin-only: record the Arweave transaction ID for an archived review.
    ///
    /// Off-chain archival jobs call this after successfully writing the full
    /// review payload to Arweave, so on-chain queries can return the permanent
    /// storage reference.
    ///
    /// # Errors
    /// - `ContractError::AdminOnly` — caller is not a registered admin.
    /// - `ContractError::ReviewNotArchived` — no archived record exists for
    ///   `(project_id, reviewer)`.
    pub fn set_archived_review_arweave_tx(
        env: &Env,
        admin: Address,
        project_id: u64,
        reviewer: Address,
        arweave_tx_id: String,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        if !crate::admin_manager::AdminManager::is_admin(env, &admin) {
            return Err(ContractError::AdminOnly);
        }

        let archive_key = ExtensionKey2::ArchivedReview(project_id, reviewer.clone());
        let mut archived: ArchivedReview = env
            .storage()
            .persistent()
            .get(&archive_key)
            .ok_or(ContractError::ReviewNotArchived)?;

        archived.arweave_tx_id = Some(arweave_tx_id);
        env.storage().persistent().set(&archive_key, &archived);
        env.storage().persistent().extend_ttl(
            &archive_key,
            LEDGER_THRESHOLD_ARCHIVED_REVIEW,
            LEDGER_BUMP_ARCHIVED_REVIEW,
        );
        Ok(())
    }
}
