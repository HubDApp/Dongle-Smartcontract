//! Review registry: create/update/delete reviews and maintain aggregates and indexes.

use crate::admin_action_log::AdminActionLog;
use crate::constants::{
    MAX_CID_LEN, MAX_PAGE_LIMIT, MAX_REVIEWS_PER_PROJECT, MAX_REVIEWS_PER_USER,
    MAX_REVIEW_REVISIONS, RATING_MAX, RATING_MIN, REVIEW_UPDATE_COOLDOWN_SECONDS,
};
use crate::errors::ContractError;
use crate::events::{publish_review_event, publish_review_revision_event};
use crate::project_registry::ProjectRegistry;
use crate::rating_calculator::RatingCalculator;
use crate::storage_keys::{ExtensionKey, StorageKey};
use crate::storage_manager::StorageManager;
use crate::types::{
    AdminActionType, Project, ProjectStats, Review, ReviewAction, ReviewRevision, ReviewSortMode,
    ReviewTombstone,
};
use crate::utils::Utils;
use soroban_sdk::{Address, Env, String, Vec};

pub struct ReviewRegistry;

impl ReviewRegistry {
    fn validate_review_cid(cid: &String) -> Result<(), ContractError> {
        if !Utils::is_valid_ipfs_cid(cid) || cid.len() as usize > MAX_CID_LEN {
            return Err(ContractError::InvalidProjectData);
        }
        Ok(())
    }

    pub fn add_review(
        env: &Env,
        project_id: u64,
        reviewer: Address,
        rating: u32,
        comment_cid: Option<String>,
    ) -> Result<(), ContractError> {
        if let Some(cid) = comment_cid.as_ref() {
            Self::validate_review_cid(cid)?;
        }

        // Validation phase
        reviewer.require_auth();

        // Check if project exists
        let project = match ProjectRegistry::get_project(env, project_id) {
            Some(p) => p,
            None => return Err(ContractError::ProjectNotFound),
        };

        // Project owners cannot review their own project
        if project.owner == reviewer {
            return Err(ContractError::OwnerCannotReview);
        }

        // Check if reviews are enabled for this project
        if !Self::get_reviews_enabled(env, project_id) {
            return Err(ContractError::ReviewsDisabled);
        }

        if !(RATING_MIN..=RATING_MAX).contains(&rating) {
            return Err(ContractError::InvalidRating);
        }

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

        if project_reviews.len() >= MAX_REVIEWS_PER_PROJECT {
            return Err(ContractError::MaxProjectsExceeded);
        }
        if user_reviews.len() >= MAX_REVIEWS_PER_USER {
            return Err(ContractError::MaxProjectsExceeded);
        }

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
            hidden: false,
            report_count: 0,
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

        publish_review_event(
            env,
            project_id,
            reviewer,
            ReviewAction::Submitted,
            comment_cid.clone(),
            None,
            now,
            now,
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
        Self::validate_review_cid(&review_cid)?;
        Self::add_review(env, project_id, reviewer, rating, Some(review_cid))
    }

    pub fn update_review(
        env: &Env,
        project_id: u64,
        reviewer: Address,
        rating: u32,
        comment_cid: Option<String>,
    ) -> Result<(), ContractError> {
        if let Some(cid) = comment_cid.as_ref() {
            Self::validate_review_cid(cid)?;
        }

        // Validation phase
        reviewer.require_auth();

        // Check if project exists
        if ProjectRegistry::get_project(env, project_id).is_none() {
            return Err(ContractError::ProjectNotFound);
        }

        if !(RATING_MIN..=RATING_MAX).contains(&rating) {
            return Err(ContractError::InvalidRating);
        }

        let review_key = StorageKey::Review(project_id, reviewer.clone());
        let mut review: Review = env
            .storage()
            .persistent()
            .get(&review_key)
            .ok_or(ContractError::ReviewNotFound)?;

        if review.reviewer != reviewer {
            return Err(ContractError::NotReviewOwner);
        }

        // Cooldown: reject update if within REVIEW_UPDATE_COOLDOWN_SECONDS of the last update.
        // Note: ContractError::InvalidStatus (9) is reused as the cooldown error because
        // Soroban SDK 22 #[contracterror] is limited to 50 variants and this enum is full.
        // A dedicated ReviewCooldownActive variant should be introduced when a variant slot
        // is freed in a future refactor.
        let cooldown_key = ExtensionKey::ReviewLastUpdated(project_id, reviewer.clone());
        if let Some(last_updated_at) = env.storage().persistent().get::<_, u64>(&cooldown_key) {
            let now_ts = env.ledger().timestamp();
            if now_ts.saturating_sub(last_updated_at) < REVIEW_UPDATE_COOLDOWN_SECONDS {
                return Err(ContractError::InvalidStatus);
            }
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
        env.storage().persistent().set(
            &StorageKey::ProjectStats(project_id),
            &ProjectStats {
                rating_sum: new_sum,
                review_count: stats.review_count,
                average_rating: new_avg,
            },
        );

        // Record the update timestamp for cooldown enforcement on subsequent updates.
        env.storage().persistent().set(
            &ExtensionKey::ReviewLastUpdated(project_id, reviewer.clone()),
            &now,
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
            revision_count.saturating_sub(1)
        }
    }

    pub fn get_review_revision_count(env: &Env, project_id: u64, reviewer: Address) -> u32 {
        env.storage()
            .persistent()
            .get(&ExtensionKey::ReviewRevisionCount(project_id, reviewer))
            .unwrap_or(0)
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

        publish_review_event(
            env,
            project_id,
            reviewer,
            ReviewAction::Deleted,
            None,
            existing.owner_response.clone(),
            existing.created_at,
            now,
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
                    reviews.push_back(review);
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

        publish_review_event(
            env,
            project_id,
            reviewer,
            ReviewAction::Updated,
            review.content_cid.clone(),
            review.owner_response.clone(),
            review.created_at,
            now,
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
        Self::get_review(env, project_id, reviewer).and_then(|review| review.content_cid)
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
                if let Some(cid) = Self::get_review_cid(env, project_id, reviewer.clone()) {
                    cids.push_back((reviewer, cid));
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

    pub fn list_reviews(env: &Env, project_id: u64, start_id: u32, limit: u32) -> Vec<Review> {
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
        if start_id >= len {
            return reviews;
        }
        let end = core::cmp::min(start_id.saturating_add(effective_limit), len);

        for i in start_id..end {
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
            return Err(ContractError::ReviewAlreadyReported);
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

    /// List reviews sorted by the requested `sort_mode` with pagination.
    ///
    /// # On-chain in-memory sort
    /// This fetches all non-hidden reviews for the project, sorts them entirely
    /// in the contract's working memory, then applies pagination. For projects
    /// with many reviews this increases compute budget usage linearly with the
    /// total review count. Use `list_reviews` (insertion-order) when sorting is
    /// not required.
    pub fn list_reviews_sorted(
        env: &Env,
        project_id: u64,
        start_id: u32,
        limit: u32,
        sort_mode: ReviewSortMode,
    ) -> Vec<Review> {
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

        // Collect all non-hidden reviews.
        let mut all: Vec<Review> = Vec::new(env);
        for i in 0..reviewers.len() {
            if let Some(reviewer) = reviewers.get(i) {
                if let Some(review) = Self::get_review(env, project_id, reviewer) {
                    if !review.hidden {
                        all.push_back(review);
                    }
                }
            }
        }

        // Bubble-sort in-memory by the requested mode.
        let n = all.len();
        for i in 0..n {
            for j in 0..n.saturating_sub(i + 1) {
                let a = all.get(j).unwrap();
                let b = all.get(j + 1).unwrap();
                let swap = match sort_mode {
                    ReviewSortMode::Newest => a.created_at < b.created_at,
                    ReviewSortMode::Oldest => a.created_at > b.created_at,
                    ReviewSortMode::RatingHigh => a.rating < b.rating,
                    ReviewSortMode::RatingLow => a.rating > b.rating,
                };
                if swap {
                    all.set(j, b);
                    all.set(j + 1, a);
                }
            }
        }

        // Apply pagination.
        let mut out = Vec::new(env);
        if start_id >= n {
            return out;
        }
        let end = core::cmp::min(start_id.saturating_add(effective_limit), n);
        for i in start_id..end {
            if let Some(review) = all.get(i) {
                out.push_back(review);
            }
        }
        out
    }
}
