#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Env, String, Vec};

mod admin_action_log;
mod admin_manager;
mod bookmark_registry;
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
mod types;
mod validation;
mod verification_registry;

#[contract]
pub struct DongleContract;

#[contractimpl]
impl DongleContract {
    pub fn initialize(env: Env, admin: Address) {
        admin_manager::initialize(&env, &admin);
    }

    pub fn register_project(env: Env, params: types::ProjectRegistrationParams) -> u64 {
        // Validate bounty fields if present
        if let Some(ref url) = params.bounty_url {
            validation::validate_bounty_url(url).unwrap_or_else(|e| { panic_with_error!(&env, e) });
        }
        if let Some(ref cid) = params.bounty_cid {
            validation::validate_bounty_cid(cid).unwrap_or_else(|e| { panic_with_error!(&env, e) });
        }
        // Delegate to verification_registry (or appropriate module)
        verification_registry::register_project(&env, params)
    }

    pub fn update_project(env: Env, project_id: u64, params: types::ProjectUpdateParams) {
        // Validate bounty fields if present
        if let Some(ref url) = params.bounty_url {
            validation::validate_bounty_url(url).unwrap_or_else(|e| { panic_with_error!(&env, e) });
        }
        if let Some(ref cid) = params.bounty_cid {
            validation::validate_bounty_cid(cid).unwrap_or_else(|e| { panic_with_error!(&env, e) });
        }
        verification_registry::update_project(&env, project_id, params)
    }

    // ... other public functions remain unchanged
}
