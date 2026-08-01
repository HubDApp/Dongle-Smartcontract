//! Validation utilities for project registration and updates.

use crate::errors::ContractError;
use crate::types::ProjectRegistrationParams;
use crate::utils::Utils;
use soroban_sdk::{String, Env};

/// Validates all fields of a project registration request.
pub fn validate_registration_params(env: &Env, params: &ProjectRegistrationParams) -> Result<(), ContractError> {
    // Validate mandatory fields
    if params.name.len() == 0 {
        return Err(ContractError::InvalidInput);
    }
    if params.slug.len() == 0 {
        return Err(ContractError::InvalidInput);
    }
    if params.description.len() == 0 {
        return Err(ContractError::InvalidInput);
    }
    
    // Validate optional bounty URL and CID
    validate_bounty_url(env, &params.bounty_url)?;
    validate_bounty_cid(env, &params.bounty_cid)?;
    
    Ok(())
}

/// Validate an optional bug bounty URL.
pub fn validate_bounty_url(env: &Env, url: &Option<String>) -> Result<(), ContractError> {
    if let Some(url) = url {
        let s: &str = &url;
        // Must start with http:// or https://
        if !s.starts_with("http://") && !s.starts_with("https://") {
            return Err(ContractError::InvalidInput);
        }
        // Simple length sanity check (e.g., at least 11 chars)
        if s.len() < 11 {
            return Err(ContractError::InvalidInput);
        }
    }
    Ok(())
}

/// Validate an optional IPFS CID (v0 or v1).
pub fn validate_bounty_cid(env: &Env, cid: &Option<String>) -> Result<(), ContractError> {
    if let Some(cid) = cid {
        if !Utils::is_valid_ipfs_cid(cid) {
            return Err(ContractError::InvalidInput);
        }
    }
    Ok(())
}
