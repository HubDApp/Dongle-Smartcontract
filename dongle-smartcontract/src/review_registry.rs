//! Review submission with validation, duplicate handling, and events.
use crate::constants::{MAX_CID_LEN, RATING_MAX, RATING_MIN};
use crate::errors::ContractError;
use crate::events::ReviewAdded;
use crate::events::ReviewUpdated;
use crate::storage_keys::StorageKey;
use crate::types::Review;
use soroban_sdk::{Address, Env, String};

/// Validates an optional IPFS CID string.
///
/// Rules enforced on-chain:
/// - If `None`, passes — CID is optional (rating-only reviews are allowed).
/// - Must not exceed `MAX_CID_LEN` characters (storage efficiency).
/// - Must be at least 46 characters (shortest valid CIDv0).
/// - Must not be empty (len == 0).
///
/// Note: Full prefix validation (Qm vs bafy) is not possible with soroban_sdk::String
/// in no_std without heap allocation. Length bounds provide the contract-level guard;
/// the frontend is responsible for supplying well-formed CIDs before calling this function.
fn validate_optional_cid(s: &Option<String>) -> Result<(), ContractError> {
    if let Some(ref x) = s {
        let len = x.len() as usize;

        // Must not exceed maximum storage size
        if len > MAX_CID_LEN {
            return Err(ContractError::StringLengthExceeded);
        }

        // Empty string is not a valid CID
        if len == 0 {
            return Err(ContractError::InvalidCid);
        }

        // CIDv0 is exactly 46 chars; CIDv1 is longer.
        // Anything shorter than 46 cannot be a valid IPFS CID.
        if len < 46 {
            return Err(ContractError::InvalidCid);
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
        _env: &Env,
        _project_id: u64,
        _reviewer: Address,
        _rating: u32,
        _comment_cid: Option<String>,
    ) -> Result<(), ContractError> {
        todo!("Review update logic not implemented")
    }

    pub fn get_review(
        _env: &Env,
        _project_id: u64,
        _reviewer: Address,
    ) -> Result<Review, ContractError> {
        todo!("Review retrieval logic not implemented")
    }

    pub fn get_project_reviews(
        _env: &Env,
        _project_id: u64,
        _start_reviewer: Option<Address>,
        _limit: u32,
    ) -> Result<Vec<Review>, ContractError> {
        todo!("Project review listing logic not implemented")
    }

    pub fn get_review_stats(_env: &Env, _project_id: u64) -> Result<(u32, u32), ContractError> {
        todo!("Review statistics calculation not implemented")
    }

    pub fn review_exists(_env: &Env, _project_id: u64, _reviewer: Address) -> bool {
        false
    }

    pub fn validate_review_data(
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

    pub fn delete_review(
        _env: &Env,
        _project_id: u64,
        _reviewer: Address,
        _admin: Address,
    ) -> Result<(), ContractError> {
        todo!("Review deletion logic not implemented")
    }
}
