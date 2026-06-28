#![no_std]
use soroban_sdk::{contract, contractimpl, Address, BytesN, Env, String, Vec};

use crate::types::{
    validate_bounty_cid, validate_bounty_url, BountyInfo, Project, ProjectRegistrationParams,
    ProjectUpdateParams, SocialLink, VerificationStatus,
};
use crate::errors::ContractError;
use crate::storage_keys::StorageKey;
use crate::constants;

mod admin_manager;
mod collection_registry;
mod constants;
mod dependency_registry;
mod dispute_registry;
mod errors;
mod featured_registry;
mod report_registry;
mod storage_keys;
mod subscription_registry;
mod timelock_manager;
mod verification_registry;
mod bookmark_registry;
mod admin_action_log;

#[contract]
pub struct DongleContract;

#[contractimpl]
impl DongleContract {
    pub fn initialize(env: Env, admin: Address) {
        // initialization logic (assume exists)
    }

    pub fn register_project(env: Env, params: ProjectRegistrationParams) -> u64 {
        // Validate bounty fields
        if let Some(ref url) = params.bounty_url {
            if !validate_bounty_url(url) {
                panic_with_error!(&env, ContractError::InvalidBountyUrl);
            }
        }
        if let Some(ref cid) = params.bounty_cid {
            if !validate_bounty_cid(cid) {
                panic_with_error!(&env, ContractError::InvalidBountyCid);
            }
        }
        // Assume existing logic continues (generate id, store, etc.)
        0 // placeholder
    }

    pub fn update_project(env: Env, project_id: u64, updater: Address, params: ProjectUpdateParams) {
        // Validate bounty fields if present
        if let Some(ref url) = params.bounty_url {
            if !validate_bounty_url(url) {
                panic_with_error!(&env, ContractError::InvalidBountyUrl);
            }
        }
        if let Some(ref cid) = params.bounty_cid {
            if !validate_bounty_cid(cid) {
                panic_with_error!(&env, ContractError::InvalidBountyCid);
            }
        }
        // Assume existing logic
    }

    // Other functions remain unchanged
}
