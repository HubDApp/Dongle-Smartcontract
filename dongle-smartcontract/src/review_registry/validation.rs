//! Review input validation helpers (CID, rating bounds, ownership checks).

use crate::constants::{MAX_CID_LEN, RATING_MAX, RATING_MIN};
use crate::errors::ContractError;
use crate::types::Project;
use crate::utils::Utils;
use soroban_sdk::{Address, String};

/// Validation helpers for review create/update paths.
pub struct ReviewValidation;

impl ReviewValidation {
    pub fn validate_review_cid(cid: &String) -> Result<(), ContractError> {
        if !Utils::is_valid_ipfs_cid(cid) || cid.len() as usize > MAX_CID_LEN {
            return Err(ContractError::InvalidProjectData);
        }
        Ok(())
    }

    pub fn validate_rating(rating: u32) -> Result<(), ContractError> {
        if !(RATING_MIN..=RATING_MAX).contains(&rating) {
            return Err(ContractError::InvalidRating);
        }
        Ok(())
    }

    /// Project owners cannot review their own project.
    pub fn ensure_not_owner(project: &Project, reviewer: &Address) -> Result<(), ContractError> {
        if project.owner == *reviewer {
            return Err(ContractError::OwnerCannotReview);
        }
        Ok(())
    }
}
