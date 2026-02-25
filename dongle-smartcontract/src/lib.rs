#![no_std]

pub mod errors;
pub mod events;
pub mod fee_manager;
pub mod project_registry;
pub mod review_registry;
pub mod types;
pub mod utils;
pub mod verification_registry;
pub mod storage_keys;
pub mod constants;

// # Dongle Smart Contract
// 
// A decentralized project registry and discovery platform built on Stellar/Soroban.
// This contract enables transparent project registration, community reviews, and 
// verification processes for the Stellar ecosystem.

#[cfg(test)]
mod test;
#[cfg(test)]
mod test_treasury;

use soroban_sdk::{contract, contractimpl, Address, Env, String, Vec};

use crate::types::{Project, Review, VerificationRecord, FeeConfig};
use crate::errors::ContractError;
use crate::review_registry::ReviewRegistry;
use crate::project_registry::ProjectRegistry;
use crate::verification_registry::VerificationRegistry;
use crate::fee_manager::FeeManager;
use crate::storage_keys::StorageKey;

#[contract]
pub struct DongleContract;

#[contractimpl]
impl DongleContract {
    // ==========================================
    // INITIALIZATION & ADMIN FUNCTIONS
    // ==========================================

    pub fn initialize(env: Env, admin: Address) -> Result<(), ContractError> {
        if env.storage().persistent().has(&StorageKey::Admin) {
            return Err(ContractError::Unauthorized); // Already initialized
        }
        FeeManager::set_admin(&env, admin.clone(), admin)?;
        Ok(())
    }

    pub fn set_admin(env: Env, caller: Address, new_admin: Address) -> Result<(), ContractError> {
        FeeManager::set_admin(&env, caller, new_admin)
    }

    // ==========================================
    // PROJECT REGISTRY FUNCTIONS
    // ==========================================

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
        ProjectRegistry::register_project(&env, owner, name, description, category, website, logo_cid, metadata_cid)
    }

    pub fn update_project(
        env: Env,
        project_id: u64,
        caller: Address,
        name: Option<String>,
        description: Option<String>,
        category: Option<String>,
        website: Option<Option<String>>,
        logo_cid: Option<Option<String>>,
        metadata_cid: Option<Option<String>>,
    ) -> Result<Project, ContractError> {
        ProjectRegistry::update_project(&env, project_id, caller, name, description, category, website, logo_cid, metadata_cid)
    }

    pub fn get_project(env: Env, project_id: u64) -> Result<Project, ContractError> {
        ProjectRegistry::get_project(&env, project_id).ok_or(ContractError::ProjectNotFound)
    }

    pub fn list_projects(env: Env, start_id: u64, limit: u32) -> Result<Vec<Project>, ContractError> {
        ProjectRegistry::list_projects(&env, start_id, limit)
    }

    pub fn get_projects_by_owner(env: Env, owner: Address) -> Vec<Project> {
        ProjectRegistry::get_projects_by_owner(&env, owner)
    }

    // ==========================================
    // REVIEW SYSTEM FUNCTIONS
    // ==========================================

    pub fn add_review(
        env: Env,
        project_id: u64,
        reviewer: Address,
        rating: u32,
        comment_cid: Option<String>,
    ) -> Result<(), ContractError> {
        ReviewRegistry::add_review(env, project_id, reviewer, rating, comment_cid);
        Ok(())
    }

    pub fn update_review(
        env: Env,
        project_id: u64,
        reviewer: Address,
        rating: u32,
        comment_cid: Option<String>,
    ) -> Result<(), ContractError> {
        ReviewRegistry::update_review(env, project_id, reviewer, rating, comment_cid);
        Ok(())
    }

    pub fn get_review(env: Env, project_id: u64, reviewer: Address) -> Result<Review, ContractError> {
        ReviewRegistry::get_review(env, project_id, reviewer)
            .ok_or(ContractError::ReviewNotFound)
    }

    pub fn get_project_reviews(env: Env, project_id: u64) -> Vec<Review> {
        ReviewRegistry::get_reviews_by_project(env, project_id)
    }

    // ==========================================
    // VERIFICATION SYSTEM FUNCTIONS
    // ==========================================

    pub fn request_verification(
        env: Env,
        project_id: u64,
        requester: Address,
        evidence_cid: String,
    ) -> Result<(), ContractError> {
        // First pay the fee
        FeeManager::pay_fee(&env, requester.clone(), project_id, "verification")?;
        
        // Then register the request
        VerificationRegistry::request_verification(&env, project_id, requester, evidence_cid);
        Ok(())
    }

    pub fn approve_verification(
        env: Env,
        project_id: u64,
        admin: Address,
    ) -> Result<(), ContractError> {
        VerificationRegistry::approve_verification(&env, project_id, admin)
    }

    pub fn reject_verification(
        env: Env,
        project_id: u64,
        admin: Address,
    ) -> Result<(), ContractError> {
        VerificationRegistry::reject_verification(&env, project_id, admin)
    }

    pub fn get_verification(
        env: Env,
        project_id: u64,
    ) -> Result<VerificationRecord, ContractError> {
        VerificationRegistry::get_verification(&env, project_id)
    }

    // ==========================================
    // FEE & TREASURY MANAGEMENT FUNCTIONS
    // ==========================================

    pub fn set_fee_config(
        env: Env,
        admin: Address,
        token: Option<Address>,
        verification_fee: u128,
        registration_fee: u128,
    ) -> Result<(), ContractError> {
        FeeManager::set_fee(&env, admin, token, verification_fee, registration_fee)
    }

    pub fn get_fee_config(env: Env) -> Result<FeeConfig, ContractError> {
        FeeManager::get_fee_config(&env)
    }

    pub fn withdraw_treasury(
        env: Env,
        admin: Address,
        token: Address,
        amount: u128,
        to: Address,
    ) -> Result<(), ContractError> {
        FeeManager::withdraw_treasury(&env, admin, token, amount, to)
    }

    pub fn get_treasury_balance(env: Env, token: Address) -> u128 {
        FeeManager::get_treasury_balance(&env, &token)
    }
}
