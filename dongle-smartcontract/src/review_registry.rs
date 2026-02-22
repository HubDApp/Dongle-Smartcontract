use crate::types::{DataKey, Review};
use crate::errors::ContractError;
use soroban_sdk::{symbol_short, Address, Env, String};

pub struct ReviewRegistry;

impl ReviewRegistry {
    pub fn add_review(
        env: &Env,
        project_id: u64,
        reviewer: Address,
        rating: u8,
        comment_cid: Option<String>,
    ) -> Result<(), ContractError> {
        // Ensure the caller authorized this transaction
        reviewer.require_auth();

        // Validate that the rating is between 1 and 5
        if rating < 1 || rating > 5 {
            return Err(ContractError::InvalidRating);
        }

        // Check for duplicates
        let key = DataKey::ProjectReview(project_id, reviewer.clone());
        if env.storage().persistent().has(&key) {
            return Err(ContractError::AlreadyReviewed);
        }

        // Save the review
        let review = Review {
            reviewer: reviewer.clone(),
            rating,
            comment_cid: comment_cid.clone(),
        };
        env.storage().persistent().set(&key, &review);

        // Emit an event
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
        rating: u8,
        comment_cid: Option<String>,
    ) -> Result<(), ContractError> {
        reviewer.require_auth();

        if rating < 1 || rating > 5 {
            return Err(ContractError::InvalidRating);
        }

        let key = DataKey::ProjectReview(project_id, reviewer.clone());
        
        if !env.storage().persistent().has(&key) {
            panic!("Review does not exist");
        }

        let updated_review = Review {
            reviewer: reviewer.clone(),
            rating,
            comment_cid,
        };
        env.storage().persistent().set(&key, &updated_review);

        Ok(())
    }

    pub fn get_review(env: &Env, project_id: u64, reviewer: Address) -> Option<Review> {
        let key = DataKey::ProjectReview(project_id, reviewer);
        env.storage().persistent().get(&key)
    }
}