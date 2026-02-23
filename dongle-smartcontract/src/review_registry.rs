use soroban_sdk::{symbol_short, Address, Env, String, Vec};
use crate::types::{DataKey, Review};
use crate::errors::ContractError;

pub struct ReviewRegistry;

impl ReviewRegistry {
    pub fn add_review(
        env: &Env,
        project_id: u64,
        reviewer: Address,
        rating: u32, // Matches types.rs u32
        comment_cid: Option<String>,
    ) -> Result<(), ContractError> {
        reviewer.require_auth();

        if rating < 1 || rating > 5 {
            return Err(ContractError::InvalidRating);
        }

        let key = DataKey::ProjectReview(project_id, reviewer.clone());
        if env.storage().persistent().has(&key) {
            return Err(ContractError::AlreadyReviewed);
        }

        // Get current ledger timestamp for the new fields
        let now = env.ledger().timestamp();

        let review = Review {
            project_id,
            reviewer: reviewer.clone(),
            rating,
            comment_cid: comment_cid.clone(),
            created_at: now,
            updated_at: now,
        };
        env.storage().persistent().set(&key, &review);

        env.events().publish(
            (symbol_short!("Review"), project_id, reviewer),
            (rating, comment_cid),
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
        reviewer.require_auth();

        if rating < 1 || rating > 5 {
            return Err(ContractError::InvalidRating);
        }

        let key = DataKey::ProjectReview(project_id, reviewer.clone());
        
        // Retrieve existing review to keep the original created_at timestamp
        let mut review: Review = env.storage().persistent().get(&key)
            .ok_or(ContractError::ReviewNotFound)?;

        review.rating = rating;
        review.comment_cid = comment_cid;
        review.updated_at = env.ledger().timestamp(); // Update only the edit time

        env.storage().persistent().set(&key, &review);

        Ok(())
    }

    pub fn get_review(
        env: &Env,
        project_id: u64,
        reviewer: Address,
    ) -> Result<Review, ContractError> {
        let key = DataKey::ProjectReview(project_id, reviewer);
        env.storage().persistent().get(&key).ok_or(ContractError::ReviewNotFound)
    }
}