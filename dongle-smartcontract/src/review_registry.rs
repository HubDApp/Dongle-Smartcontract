//! Review submission with validation, duplicate handling, and events.
use crate::constants::{MAX_CID_LEN, RATING_MAX, RATING_MIN};
use crate::errors::ContractError;
use crate::events::ReviewAdded;
use crate::events::ReviewUpdated;
use crate::storage_keys::StorageKey;
use crate::types::Review;
use soroban_sdk::{Address, Env, String};

fn validate_optional_cid(s: &Option<String>) -> Result<(), ContractError> {
    if let Some(ref x) = s {
        if x.len() as usize > MAX_CID_LEN {
            return Err(ContractError::StringLengthExceeded);
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
        rating: u32,
        comment_cid: Option<String>,
    ) -> Result<(), ContractError> {
        reviewer.require_auth();

        if rating < RATING_MIN || rating > RATING_MAX {
            return Err(ContractError::InvalidRating);
        }
        validate_optional_cid(&comment_cid)?;

        let key = StorageKey::Review(project_id, reviewer.clone());
        if env.storage().persistent().has(&key) {
            return Err(ContractError::DuplicateReview);
        }

        let ledger_timestamp = env.ledger().timestamp();
        let review = Review {
            project_id,
            reviewer: reviewer.clone(),
            rating,
            comment_cid: comment_cid.clone(),
            created_at: ledger_timestamp,
            updated_at: ledger_timestamp,
        };

        env.storage().persistent().set(&key, &review);

        ReviewAdded {
            project_id,
            reviewer: reviewer.clone(),
            rating,
        }
        .publish(env);

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

        if rating < RATING_MIN || rating > RATING_MAX {
            return Err(ContractError::InvalidRating);
        }
        validate_optional_cid(&comment_cid)?;

        let key = StorageKey::Review(project_id, reviewer.clone());
        let mut review: Review = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(ContractError::ReviewNotFound)?;

        if review.reviewer != reviewer {
            return Err(ContractError::NotReviewAuthor);
        }

        let ledger_timestamp = env.ledger().timestamp();
        review.rating = rating;
        review.comment_cid = comment_cid;
        review.updated_at = ledger_timestamp;

        env.storage().persistent().set(&key, &review);

        ReviewUpdated {
            project_id,
            reviewer,
            rating,
            updated_at: ledger_timestamp,
        }
        .publish(env);

        Ok(())
    }

    pub fn get_review(env: &Env, project_id: u64, reviewer: Address) -> Option<Review> {
        env.storage()
            .persistent()
            .get(&StorageKey::Review(project_id, reviewer))
    }
}
