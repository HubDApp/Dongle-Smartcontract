//! Review submission with validation, duplicate handling, and events.

use crate::constants::{MAX_CID_LEN, RATING_MAX, RATING_MIN};
use crate::errors::ContractError;
use crate::events::ReviewAdded;
use crate::events::ReviewUpdated;
use crate::storage_keys::StorageKey;
use crate::types::Review;
use soroban_sdk::{Address, Env, String as SorobanString, Vec};

fn validate_optional_cid(s: &Option<SorobanString>) -> Result<(), ContractError> {
    if let Some(ref x) = s {
        if x.len() as usize > MAX_CID_LEN {
            return Err(ContractError::InvalidProjectData);
        }
    }
    Ok(())
}

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