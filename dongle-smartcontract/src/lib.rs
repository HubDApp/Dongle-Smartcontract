#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Env, String, Vec};

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

#[cfg(test)]
mod tests {
    mod fixtures;
    mod indexer;
    mod maintainers;
    mod index_limits;
    mod bounty_metadata;
}

#[contract]
pub struct DongleContract;

#[contractimpl]
impl DongleContract {
    pub fn initialize(env: Env, admin: Address) {
        admin_manager::AdminManager::initialize(&env, &admin);
    }

    // Placeholder for actual contract methods used elsewhere.
    // Full implementation resides in the registry modules.
}
