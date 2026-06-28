use crate::errors::ContractError;
use crate::storage_keys::StorageKey;
use crate::types::{Project, ProjectRegistrationParams, ProjectUpdateParams};
use soroban_sdk::{Address, Env, String, Vec};

fn validate_bounty_url(url: &Option<String>) -> Result<(), ContractError> {
    if let Some(u) = url {
        if !u.starts_with("http://") && !u.starts_with("https://") {
            return Err(ContractError::InvalidBountyUrl);
        }
    }
    Ok(())
}

fn validate_bounty_cid(cid: &Option<String>) -> Result<(), ContractError> {
    if let Some(c) = cid {
        let len = c.len();
        // CIDv0: starts with Qm, length 46
        // CIDv1: starts with bafy, length 59 (example)
        if !(c.starts_with(&String::from_str(&Env::default(), "Qm")) && len == 46)
            && !(c.starts_with(&String::from_str(&Env::default(), "bafy")) && len == 59)
        {
            return Err(ContractError::InvalidBountyCid);
        }
    }
    Ok(())
}

pub fn register_project(
    env: &Env,
    params: ProjectRegistrationParams,
) -> Result<u64, ContractError> {
    validate_bounty_url(&params.bounty_url)?;
    validate_bounty_cid(&params.bounty_cid)?;
    // existing logic for project creation
    // ... (placeholder)
    Ok(1)
}

pub fn update_project(
    env: &Env,
    project_id: u64,
    params: ProjectUpdateParams,
) -> Result<(), ContractError> {
    if let Some(ref url) = params.bounty_url {
        validate_bounty_url(url)?;
    }
    if let Some(ref cid) = params.bounty_cid {
        validate_bounty_cid(cid)?;
    }
    // existing update logic
    Ok(())
}