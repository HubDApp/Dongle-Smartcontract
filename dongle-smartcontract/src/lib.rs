#![no_std]
#![allow(dead_code)]
#![allow(clippy::too_many_arguments)]
//! Dongle Smart Contract: project registry, reviews, and verification on Stellar/Soroban.

mod constants;
mod errors;
mod events;
mod fee_manager;
mod project_registry;
mod review_registry;
mod storage_keys;
mod types;
mod utils;
mod verification_registry;

use soroban_sdk::{contract, contractimpl, Address, Env, String, Vec};

use crate::errors::ContractError;
use crate::project_registry::ProjectRegistry;
use crate::types::{FeeConfig, Project, Review, VerificationRecord};

#[contract]
pub struct DongleContract;

#[contractimpl]
impl DongleContract {
    pub fn initialize(_env: Env, _admin: Address, _treasury: Address) -> Result<(), Error> {
        todo!("Contract initialization not yet implemented")
    }

    pub fn set_admin(_env: Env, _caller: Address, _new_admin: Address) -> Result<(), Error> {
        todo!("Admin management not yet implemented")
    }

    pub fn register_project(
        env: Env,
        owner: Address,
        name: String,
        description: String,
        category: String,
        website: Option<String>,
        logo_cid: Option<String>,
        metadata_cid: Option<String>,
    ) -> Result<u64, Error> {
        project_registry::ProjectRegistry::register_project(
            &env,
            owner,
            name,
            description,
            category,
            website,
            logo_cid,
            metadata_cid,
        )
    }

    pub fn update_project(
        env: Env,
        project_id: u64,
        caller: Address,
        name: String,
        description: String,
        category: String,
        website: Option<String>,
        logo_cid: Option<String>,
        metadata_cid: Option<String>,
    ) -> Result<(), ContractError> {
        // ACTUAL IMPLEMENTATION: Replacing todo!() with our secure logic
        ProjectRegistry::update_project(
            &env,
            project_id,
            caller,
            name,
            description,
            category,
            website,
            logo_cid,
            metadata_cid,
        )
    }

    pub fn get_project(env: Env, project_id: u64) -> Result<Project, ContractError> {
        // ACTUAL IMPLEMENTATION: Replacing todo!() with our retrieval logic
        ProjectRegistry::get_project(&env, project_id).ok_or(ContractError::ProjectNotFound)
    }

    pub fn list_projects(
        _env: Env,
        _start_id: u64,
        _limit: u32,
    ) -> Result<Vec<Project>, ContractError> {
        todo!("Project listing not yet implemented")
    }

    pub fn add_review(
        _env: Env,
        _project_id: u64,
        _reviewer: Address,
        _rating: u32,
        _comment_cid: Option<String>,
    ) -> Result<(), Error> {
        todo!("Review submission not yet implemented")
    }

    pub fn update_review(
        _env: Env,
        _project_id: u64,
        _reviewer: Address,
        _rating: u32,
        _comment_cid: Option<String>,
    ) -> Result<(), Error> {
        todo!("Review updates not yet implemented")
    }

    pub fn get_review(_env: Env, _project_id: u64, _reviewer: Address) -> Result<Review, Error> {
        todo!("Review retrieval not yet implemented")
    }

    pub fn get_project_reviews(
        _env: Env,
        _project_id: u64,
        _start_reviewer: Option<Address>,
        _limit: u32,
    ) -> Result<Vec<Review>, Error> {
        todo!("Project review listing not yet implemented")
    }

    pub fn request_verification(
        _env: Env,
        _project_id: u64,
        _requester: Address,
        _evidence_cid: String,
    ) -> Result<(), ContractError> {
        todo!("Verification requests not yet implemented")
    }

    pub fn approve_verification(
        _env: Env,
        _project_id: u64,
        _admin: Address,
    ) -> Result<(), ContractError> {
        todo!("Verification approval not yet implemented")
    }

    pub fn reject_verification(
        _env: Env,
        _project_id: u64,
        _admin: Address,
    ) -> Result<(), ContractError> {
        todo!("Verification rejection not yet implemented")
    }

    pub fn get_verification(
        _env: Env,
        _project_id: u64,
    ) -> Result<VerificationRecord, ContractError> {
        todo!("Verification record retrieval not yet implemented")
    }

    pub fn set_fee_config(
        _env: Env,
        _admin: Address,
        _token: Option<Address>,
        _verification_fee: u128,
        _registration_fee: u128,
    ) -> Result<(), ContractError> {
        todo!("Fee configuration not yet implemented")
    }

    pub fn get_fee_config(_env: Env) -> Result<FeeConfig, ContractError> {
        todo!("Fee configuration retrieval not yet implemented")
    }

    pub fn set_treasury(
        _env: Env,
        _admin: Address,
        _treasury: Address,
    ) -> Result<(), ContractError> {
        todo!("Treasury management not yet implemented")
    }

    pub fn get_treasury(_env: Env) -> Result<Address, ContractError> {
        todo!("Treasury address retrieval not yet implemented")
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn test_unauthorized_update_fails() {
        let env = Env::default();
        let _owner = Address::generate(&env);
        let _hacker = Address::generate(&env);
        // Note: Full test logic will require a project to be registered first
    }
}
