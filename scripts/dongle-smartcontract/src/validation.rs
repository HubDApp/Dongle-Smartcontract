//! Validation utilities for project registration and updates.

use crate::errors::ContractError;
use crate::types::ProjectRegistrationParams;
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
        let s: &str = &cid;
        // CIDv0: starts with "Qm" and length 46, base58btc
        if s.starts_with("Qm") && s.len() == 46 {
            // basic check: all characters must be valid base58btc
            for c in s.chars() {
                if !c.is_ascii_alphanumeric() && c != '1' && c != '2' && c != '3' && c != '4' && c != '5' && c != '6' && c != '7' && c != '8' && c != '9' {
                    return Err(ContractError::InvalidInput);
                }
            }
        } else if s.starts_with("bafy") || s.starts_with("bafk") || s.starts_with("ba") {
            // CIDv1: typically starts with "b" + multibase prefix (e.g., "bafy...")
            // We accept any CIDv1 starting with "b" and length >= 40
            if s.len() < 40 || !s.starts_with('b') || !s[1..].chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
                return Err(ContractError::InvalidInput);
            }
        } else {
            return Err(ContractError::InvalidInput);
        }
    }
    Ok(())
}
