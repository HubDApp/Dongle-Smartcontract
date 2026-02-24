#![no_std]
//! Dongle Smart Contract: project registry, reviews, and verification on Stellar/Soroban.

pub mod constants;
pub mod errors;
pub mod events;
pub mod fee_manager;
pub mod project_registry;
pub mod rating_calculator;
pub mod review_registry;
pub mod storage_keys;
pub mod types;
pub mod utils;
pub mod verification_registry;

use soroban_sdk::{contract, contractimpl, Address, Env, String, Vec};

use errors::ContractError;
use fee_manager::FeeManager;
use project_registry::ProjectRegistry;
use review_registry::ReviewRegistry;
use storage_keys::StorageKey;
use types::{FeeConfig, Project, Review, VerificationRecord};
use verification_registry::VerificationRegistry;

#[contract]
pub struct DongleContract;

#[contractimpl]
impl DongleContract {
    /// Initializes the contract with an admin and treasury address.
    pub fn initialize(env: Env, admin: Address, treasury: Address) -> Result<(), ContractError> {
        admin.require_auth();
        env.storage().persistent().set(&StorageKey::Admin, &admin);
        env.storage()
            .persistent()
            .set(&StorageKey::Treasury, &treasury);
        Ok(())
    }

    /// Sets the admin. Caller must be the existing admin (or anyone if unset).
    pub fn set_admin(env: Env, caller: Address, new_admin: Address) -> Result<(), ContractError> {
        utils::Utils::add_admin(&env, &caller, &new_admin)
    }

    /// Registers a new project and returns its ID.
    pub fn register_project(
        env: Env,
        owner: Address,
        name: String,
        description: String,
        category: String,
        website: Option<String>,
        logo_cid: Option<String>,
        metadata_cid: Option<String>,
    ) -> Result<u64, ContractError> {
        ProjectRegistry::register_project(
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

    /// Updates an existing project. Caller must be the project owner.
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

    /// Returns a project by ID, or error if not found.
    pub fn get_project(env: Env, project_id: u64) -> Result<Project, ContractError> {
        ProjectRegistry::get_project(&env, project_id)?.ok_or(ContractError::ProjectNotFound)
    }

    /// Returns the number of projects registered by an owner.
    pub fn get_owner_project_count(env: Env, owner: Address) -> u32 {
        ProjectRegistry::get_owner_project_count(&env, &owner)
    }

    /// Returns a paginated list of projects starting from start_id.
    pub fn list_projects(
        env: Env,
        start_id: u64,
        limit: u32,
    ) -> Result<Vec<Project>, ContractError> {
        let mut results = Vec::new(&env);
        let mut id = if start_id == 0 { 1 } else { start_id };
        let mut count = 0u32;
        let max = limit.min(100);
        while count < max {
            match ProjectRegistry::get_project(&env, id)? {
                Some(p) => {
                    results.push_back(p);
                    count += 1;
                }
                None => break,
            }
            id += 1;
        }
        Ok(results)
    }

    /// Submits a review for a project. Only the CID is stored on-chain; text lives on IPFS.
    pub fn add_review(
        env: Env,
        project_id: u64,
        reviewer: Address,
        rating: u32,
        comment_cid: Option<String>,
    ) -> Result<(), ContractError> {
        ReviewRegistry::add_review(&env, project_id, reviewer, rating, comment_cid)
    }

    /// Updates an existing review. Caller must be the original reviewer.
    pub fn update_review(
        env: Env,
        project_id: u64,
        reviewer: Address,
        rating: u32,
        comment_cid: Option<String>,
    ) -> Result<(), ContractError> {
        ReviewRegistry::update_review(&env, project_id, reviewer, rating, comment_cid)
    }

    /// Returns a specific review, or error if not found.
    /// The `comment_cid` field is used by the frontend to fetch review text from IPFS.
    pub fn get_review(
        env: Env,
        project_id: u64,
        reviewer: Address,
    ) -> Result<Review, ContractError> {
        ReviewRegistry::get_review(&env, project_id, reviewer)
            .ok_or(ContractError::ReviewNotFound)
    }

    /// Returns reviews for a project (stub — full pagination requires an index).
    pub fn get_project_reviews(
        env: Env,
        _project_id: u64,
        _start_reviewer: Option<Address>,
        _limit: u32,
    ) -> Result<Vec<Review>, ContractError> {
        Ok(Vec::new(&env))
    }

    /// Requests verification for a project.
    pub fn request_verification(
        env: Env,
        project_id: u64,
        requester: Address,
        evidence_cid: String,
    ) -> Result<(), ContractError> {
        VerificationRegistry::request_verification(&env, project_id, requester, evidence_cid)
    }

    /// Approves a pending verification. Caller must be admin.
    pub fn approve_verification(
        env: Env,
        project_id: u64,
        admin: Address,
    ) -> Result<(), ContractError> {
        VerificationRegistry::approve_verification(&env, project_id, admin)
    }

    /// Rejects a pending verification. Caller must be admin.
    pub fn reject_verification(
        env: Env,
        project_id: u64,
        admin: Address,
    ) -> Result<(), ContractError> {
        VerificationRegistry::reject_verification(&env, project_id, admin)
    }

    /// Returns the verification record for a project, or error if not found.
    pub fn get_verification(
        env: Env,
        project_id: u64,
    ) -> Result<VerificationRecord, ContractError> {
        VerificationRegistry::get_verification(&env, project_id)
            .ok_or(ContractError::VerificationNotFound)
    }

    /// Sets the fee configuration. Caller must be admin.
    pub fn set_fee_config(
        env: Env,
        admin: Address,
        token: Option<Address>,
        verification_fee: u128,
        registration_fee: u128,
    ) -> Result<(), ContractError> {
        FeeManager::set_fee_config(&env, &admin, token, verification_fee, registration_fee)
    }

    /// Convenience fee setter with an explicit treasury address (used in tests / simple flows).
    /// Stores the treasury, then sets the fee config.
    pub fn set_fee(
        env: Env,
        admin: Address,
        token: Option<Address>,
        amount: u128,
        treasury: Address,
    ) -> Result<(), ContractError> {
        // Persist the treasury so pay_fee can reference it.
        env.storage()
            .persistent()
            .set(&StorageKey::Treasury, &treasury);
        FeeManager::set_fee(&env, admin, token, amount, treasury)
    }

    /// Pays the verification fee for a project, marking it eligible for verification.
    pub fn pay_fee(
        env: Env,
        payer: Address,
        project_id: u64,
        token: Option<Address>,
    ) -> Result<(), ContractError> {
        FeeManager::pay_fee(&env, payer, project_id, token)
    }

    /// Returns the current fee configuration.
    pub fn get_fee_config(env: Env) -> Result<FeeConfig, ContractError> {
        FeeManager::get_fee_config(&env)
    }

    /// Sets the treasury address. Caller must be admin.
    pub fn set_treasury(env: Env, admin: Address, treasury: Address) -> Result<(), ContractError> {
        FeeManager::set_treasury(&env, &admin, treasury)
    }

    /// Returns the current treasury address.
    pub fn get_treasury(env: Env) -> Result<Address, ContractError> {
        FeeManager::get_treasury(&env)
    }
}

#[cfg(test)]
mod test;
