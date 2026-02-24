//! Review submission with validation, duplicate handling, and events.

use crate::constants::{MAX_CID_LEN, RATING_MAX, RATING_MIN};
use crate::errors::Error;
use crate::events::ReviewAdded;
use crate::events::ReviewUpdated;
use crate::storage_keys::StorageKey;
use crate::types::Review;
use soroban_sdk::{Address, Env, String as SorobanString};

fn validate_optional_cid(s: &Option<SorobanString>) -> Result<(), Error> {
    if let Some(ref x) = s {
        if x.len() as usize > MAX_CID_LEN {
            return Err(Error::StringLengthExceeded);
        }
    }
    Ok(())
}

pub struct ReviewRegistry;

impl ReviewRegistry {
    pub fn add_review(
        _env: &Env,
        _project_id: u64,
        _reviewer: Address,
        _rating: u32,
        _comment_cid: Option<String>,
    ) -> Result<(), ContractError> {
        todo!("Review submission logic not implemented")
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
        _comment_cid: &Option<String>,
    ) -> Result<(), ContractError> {
        if !(1..=5).contains(&rating) {
            return Err(ContractError::InvalidRating);
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
