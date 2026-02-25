 //! Review submission with validation, duplicate handling, soft-delete,
//! user review index, events, and proper aggregate updates.

use crate::errors::ContractError;
use crate::events::publish_review_event;
use crate::rating_calculator::RatingCalculator;
use crate::types::{DataKey, ProjectStats, Review, ReviewAction};
use soroban_sdk::{Address, Env, String};

pub struct ReviewRegistry;

impl ReviewRegistry {
    fn validate_rating(rating: u32) {
        assert!(rating >= 1 && rating <= 5, "Rating must be between 1 and 5");
    }

    pub fn add_review(
        env: &Env,
        project_id: u64,
        reviewer: Address,
        rating: u32,
        comment_cid: Option<String>,
    ) {
        reviewer.require_auth();
        Self::validate_rating(rating);

        let review_key = DataKey::Review(project_id, reviewer.clone());

        // ── Prevent duplicate active review (allow re-submission after soft-delete) ──
        if let Some(existing) = env.storage().persistent().get(&review_key) {
            assert!(
                existing.is_deleted,
                "Reviewer has already submitted a review for this project"
            );
        }

        // Add project to user's review history (only on first-ever submission)
        if !env.storage().persistent().has(&review_key) {
            let mut user_reviews: soroban_sdk::Vec<u64> = env
                .storage()
                .persistent()
                .get(&DataKey::UserReviews(reviewer.clone()))
                .unwrap_or(soroban_sdk::Vec::new(env));

            user_reviews.push_back(project_id);
            env.storage()
                .persistent()
                .set(&DataKey::UserReviews(reviewer.clone()), &user_reviews);
        }

        let now = env.ledger().timestamp();
        let review = Review {
            project_id,
            reviewer: reviewer.clone(),
            rating,
            timestamp: now,
            comment_cid: comment_cid.clone(),
            is_deleted: false,
        };
        env.storage().persistent().set(&review_key, &review);

        // ── Update project stats (add rating) ──
        let stats_key = DataKey::ProjectStats(project_id);
        let mut stats: ProjectStats = env
            .storage()
            .persistent()
            .get(&stats_key)
            .unwrap_or(ProjectStats {
                rating_sum: 0,
                review_count: 0,
                average_rating: 0,
            });

        stats.rating_sum = stats.rating_sum.saturating_add(rating as u64);
        stats.review_count = stats.review_count.saturating_add(1);
        stats.average_rating = if stats.review_count > 0 {
            stats.rating_sum / (stats.review_count as u64)
        } else {
            0
        };
        env.storage().persistent().set(&stats_key, &stats);

        publish_review_event(
            env,
            project_id,
            reviewer,
            ReviewAction::Submitted,
            comment_cid,
        );
    }

    pub fn update_review(
        env: &Env,
        project_id: u64,
        reviewer: Address,
        rating: u32,
        comment_cid: Option<String>,
    ) {
        reviewer.require_auth();
        Self::validate_rating(rating);

        let review_key = DataKey::Review(project_id, reviewer.clone());
        let mut review: Review = env
            .storage()
            .persistent()
            .get(&review_key)
            .expect("Review not found");

        assert!(!review.is_deleted, "Cannot update a deleted review");

        let old_rating = review.rating;
        let now = env.ledger().timestamp();

        review.rating = rating;
        review.comment_cid = comment_cid.clone();
        review.timestamp = now;
        env.storage().persistent().set(&review_key, &review);

        // ── Update project stats (replace old rating) ──
        let stats_key = DataKey::ProjectStats(project_id);
        let mut stats: ProjectStats = env
            .storage()
            .persistent()
            .get(&stats_key)
            .unwrap_or(ProjectStats {
                rating_sum: 0,
                review_count: 0,
                average_rating: 0,
            });

        stats.rating_sum = stats
            .rating_sum
            .saturating_sub(old_rating as u64)
            .saturating_add(rating as u64);

        if stats.review_count > 0 {
            stats.average_rating = stats.rating_sum / (stats.review_count as u64);
        } else {
            stats.average_rating = 0;
        }
        env.storage().persistent().set(&stats_key, &stats);

        publish_review_event(
            env,
            project_id,
            reviewer,
            ReviewAction::Updated,
            comment_cid,
        );
    }

    pub fn delete_review(
        env: &Env,
        project_id: u64,
        reviewer: Address,
    ) -> Result<(), ContractError> {
        reviewer.require_auth();

        let review_key = DataKey::Review(project_id, reviewer.clone());
        let mut review: Review = env
            .storage()
            .persistent()
            .get(&review_key)
            .ok_or(ContractError::ReviewNotFound)?;

        if review.is_deleted {
            return Err(ContractError::ReviewAlreadyDeleted);
        }

        // Update aggregate (remove rating)
        let stats_key = DataKey::ProjectStats(project_id);
        let mut stats: ProjectStats = env
            .storage()
            .persistent()
            .get(&stats_key)
            .unwrap_or(ProjectStats {
                rating_sum: 0,
                review_count: 0,
                average_rating: 0,
            });

        if stats.review_count > 0 {
            let (new_sum, new_count, new_avg) = RatingCalculator::remove_rating(
                stats.rating_sum,
                stats.review_count,
                review.rating,
            );
            stats.rating_sum = new_sum;
            stats.review_count = new_count;
            stats.average_rating = new_avg;
            env.storage().persistent().set(&stats_key, &stats);
        }

        review.is_deleted = true;
        env.storage().persistent().set(&review_key, &review);

        publish_review_event(env, project_id, reviewer, ReviewAction::Deleted, None);

        Ok(())
    }

    pub fn get_review(env: &Env, project_id: u64, reviewer: Address) -> Option<Review> {
        env.storage()
            .persistent()
            .get(&DataKey::Review(project_id, reviewer))
    }

    pub fn get_reviews_by_user(
        env: &Env,
        user: Address,
        offset: u32,
        limit: u32,
    ) -> soroban_sdk::Vec<Review> {
        let project_ids: soroban_sdk::Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::UserReviews(user.clone()))
            .unwrap_or(soroban_sdk::Vec::new(env));

        let mut reviews = soroban_sdk::Vec::new(env);
        let start = offset;
        let len = project_ids.len();
        let end = core::cmp::min(offset.saturating_add(limit), len);

        for i in start..end {
            if let Some(project_id) = project_ids.get(i) {
                if let Some(review) = Self::get_review(env, project_id, user.clone()) {
                    if !review.is_deleted {
                        reviews.push_back(review);   // only active reviews
                    }
                }
            }
        }
        reviews
    }
}