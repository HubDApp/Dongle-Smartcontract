//! Review registry: create/update/delete reviews and maintain aggregates and indexes.

use crate::auth::require_self_auth;
use crate::constants::{RATING_MAX, RATING_MIN};
use crate::errors::ContractError;
use crate::events::publish_review_event;
use crate::rating_calculator::RatingCalculator;
use crate::storage_keys::StorageKey;
use crate::types::{ProjectStats, Review, ReviewAction};
use soroban_sdk::{Address, Env, String, Vec};

pub struct ReviewRegistry;

impl ReviewRegistry {
    pub fn add_review(
        env: &Env,
        project_id: u64,
        reviewer: Address,
        rating: u32,
        comment_cid: Option<String>,
    ) -> Result<(), ContractError> {
        require_self_auth(&reviewer);

        if !(RATING_MIN..=RATING_MAX).contains(&rating) {
            return Err(ContractError::InvalidRating);
        }

        let review_key = StorageKey::Review(project_id, reviewer.clone());
        if env.storage().persistent().has(&review_key) {
            return Err(ContractError::DuplicateReview);
        }

        let now = env.ledger().timestamp();
        let review = Review {
            project_id,
            reviewer: reviewer.clone(),
            rating,
            ipfs_cid: None,
            comment_cid: comment_cid.clone(),
            created_at: now,
            updated_at: now,
        };
        env.storage().persistent().set(&review_key, &review);

        // Update user review index
        let mut user_reviews: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::UserReviews(reviewer.clone()))
            .unwrap_or_else(|| Vec::new(env));
        user_reviews.push_back(project_id);
        env.storage()
            .persistent()
            .set(&StorageKey::UserReviews(reviewer.clone()), &user_reviews);

        // Update project reviewer index
        let mut project_reviews: Vec<Address> = env
            .storage()
            .persistent()
            .get(&StorageKey::ProjectReviews(project_id))
            .unwrap_or_else(|| Vec::new(env));
        project_reviews.push_back(reviewer.clone());
        env.storage()
            .persistent()
            .set(&StorageKey::ProjectReviews(project_id), &project_reviews);

        // Update aggregate stats
        let stats_key = StorageKey::ProjectStats(project_id);
        let stats: ProjectStats =
            env.storage()
                .persistent()
                .get(&stats_key)
                .unwrap_or(ProjectStats {
                    rating_sum: 0,
                    review_count: 0,
                    average_rating: 0,
                });
        let (new_sum, new_count, new_avg) =
            RatingCalculator::add_rating(stats.rating_sum, stats.review_count, rating);
        env.storage().persistent().set(
            &stats_key,
            &ProjectStats {
                rating_sum: new_sum,
                review_count: new_count,
                average_rating: new_avg,
            },
        );

        publish_review_event(
            env,
            project_id,
            reviewer,
            ReviewAction::Submitted,
            None,
            comment_cid,
            now,
            now,
        );
        Ok(())
    }

    pub fn update_review(
        env: &Env,
        project_id: u64,
        reviewer: Address,
        rating: u32,
        comment_cid: Option<String>,
    ) -> Result<(), ContractError> {
        require_self_auth(&reviewer);

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

        let old_rating = review.rating;
        let now = env.ledger().timestamp();
        review.rating = rating;
        review.comment_cid = comment_cid.clone();
        review.updated_at = now;
        env.storage().persistent().set(&review_key, &review);

        // Update aggregate stats
        let stats_key = StorageKey::ProjectStats(project_id);
        let mut stats: ProjectStats =
            env.storage()
                .persistent()
                .get(&stats_key)
                .unwrap_or(ProjectStats {
                    rating_sum: 0,
                    review_count: 0,
                    average_rating: 0,
                });
        let (new_sum, _new_count, new_avg) = RatingCalculator::update_rating(
            stats.rating_sum,
            stats.review_count,
            old_rating,
            rating,
        );
        stats.rating_sum = new_sum;
        stats.average_rating = new_avg;
        env.storage().persistent().set(&stats_key, &stats);

        publish_review_event(
            env,
            project_id,
            reviewer,
            ReviewAction::Updated,
            None,
            comment_cid,
            review.created_at,
            now,
        );
        Ok(())
    }

    pub fn delete_review(
        env: &Env,
        project_id: u64,
        reviewer: Address,
    ) -> Result<(), ContractError> {
        require_self_auth(&reviewer);

        let review_key = StorageKey::Review(project_id, reviewer.clone());
        let existing: Review = env
            .storage()
            .persistent()
            .get(&review_key)
            .ok_or(ContractError::ReviewNotFound)?;

        if existing.reviewer != reviewer {
            return Err(ContractError::NotReviewOwner);
        }

        // Hard delete
        env.storage().persistent().remove(&review_key);

        // Update aggregate stats
        let stats_key = StorageKey::ProjectStats(project_id);
        let mut stats: ProjectStats =
            env.storage()
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
                existing.rating,
            );
            stats.rating_sum = new_sum;
            stats.review_count = new_count;
            stats.average_rating = new_avg;
            env.storage().persistent().set(&stats_key, &stats);
        }

        // Remove from user review index
        let user_reviews: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::UserReviews(reviewer.clone()))
            .unwrap_or_else(|| Vec::new(env));
        let mut new_user_reviews = Vec::new(env);
        for i in 0..user_reviews.len() {
            if let Some(id) = user_reviews.get(i) {
                if id != project_id {
                    new_user_reviews.push_back(id);
                }
            }
        }
        env.storage().persistent().set(
            &StorageKey::UserReviews(reviewer.clone()),
            &new_user_reviews,
        );

        // Remove from project reviewer index
        let project_reviews: Vec<Address> = env
            .storage()
            .persistent()
            .get(&StorageKey::ProjectReviews(project_id))
            .unwrap_or_else(|| Vec::new(env));
        let mut new_project_reviews = Vec::new(env);
        for i in 0..project_reviews.len() {
            if let Some(addr) = project_reviews.get(i) {
                if addr != reviewer {
                    new_project_reviews.push_back(addr);
                }
            }
        }
        env.storage().persistent().set(
            &StorageKey::ProjectReviews(project_id),
            &new_project_reviews,
        );

        let now = env.ledger().timestamp();
        publish_review_event(
            env,
            project_id,
            reviewer,
            ReviewAction::Deleted,
            None,
            None,
            existing.created_at,
            now,
        );
        Ok(())
    }

    pub fn get_review(env: &Env, project_id: u64, reviewer: Address) -> Option<Review> {
        env.storage()
            .persistent()
            .get(&StorageKey::Review(project_id, reviewer))
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

    pub fn list_reviews(env: &Env, project_id: u64, start_id: u32, limit: u32) -> Vec<Review> {
        // Enforce pagination limits: limit must be 1..=MAX_PAGE_LIMIT
        const MAX_PAGE_LIMIT: u32 = 100;
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
                    reviews.push_back(review);
                }
            }
        }
        reviews
    }
}
