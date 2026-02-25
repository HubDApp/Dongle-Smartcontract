#![no_std]

//! # Dongle Smart Contract
//! 
//! A decentralized project registry and discovery platform built on Stellar/Soroban.
//! This contract enables transparent project registration, community reviews, and 
//! verification processes for the Stellar ecosystem.

pub mod types;
pub mod errors;
pub mod project_registry;
pub mod review_registry;
pub mod verification_registry;
pub mod fee_manager;
pub mod events;
pub mod utils;
pub mod constants;
pub mod rating_calculator;

use soroban_sdk::{contract, contractimpl, Address, Env, String, Vec};

use types::{Project, Review, VerificationRecord, FeeConfig};
use crate::errors::ContractError;
use crate::review_registry::ReviewRegistry;

/// The main Dongle smart contract
#[contract]
pub struct DongleContract;

/// Contract implementation with all core functionality
#[contractimpl]
impl DongleContract {
    // ==========================================
    // INITIALIZATION & ADMIN FUNCTIONS
    // ==========================================

    pub fn initialize(_env: Env, _admin: Address, _treasury: Address) -> Result<(), ContractError> {
        todo!("Contract initialization not yet implemented")
    }

    pub fn set_admin(_env: Env, _caller: Address, _new_admin: Address) -> Result<(), ContractError> {
        todo!("Admin management not yet implemented")
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
        crate::project_registry::ProjectRegistry::register_project(
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
        name: Option<String>,
        description: Option<String>,
        category: Option<String>,
        website: Option<Option<String>>,
        logo_cid: Option<Option<String>>,
        metadata_cid: Option<Option<String>>,
    ) -> Result<(), ContractError> {
        crate::project_registry::ProjectRegistry::update_project(
            &env,
            project_id,
            caller,
            name,
            description,
            category,
            website,
            logo_cid,
            metadata_cid,
        ).ok_or(ContractError::ProjectNotFound)?;
        Ok(())
    }

    pub fn get_project(env: Env, project_id: u64) -> Result<Project, ContractError> {
        crate::project_registry::ProjectRegistry::get_project(&env, project_id)
            .ok_or(ContractError::ProjectNotFound)
    }

    pub fn list_projects(_env: Env, _start_id: u64, _limit: u32) -> Result<Vec<Project>, ContractError> {
        todo!("Project listing not yet implemented")
    }

    // ==========================================
    // REVIEW SYSTEM FUNCTIONS (Your Logic Integrated)
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

    pub fn get_user_reviews(
        env: Env,
        user: Address,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<Review>, ContractError> {
        Ok(ReviewRegistry::get_reviews_by_user(env, user, offset, limit))
    }

    // ==========================================
    // VERIFICATION SYSTEM FUNCTIONS
    // ==========================================

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

    pub fn get_verification(_env: Env, _project_id: u64) -> Result<VerificationRecord, ContractError> {
        todo!("Verification record retrieval not yet implemented")
    }

    // ==========================================
    // FEE MANAGEMENT FUNCTIONS
    // ==========================================

    pub fn set_fee_config(
        _env: Env,
        _admin: Address,
        _token: Option<Address>,
        _verification_fee: u128,
        _registration_fee: u128,
    ) -> Result<(), ContractError> {
        todo!("Fee configuration not yet implemented")
    }

    pub fn set_treasury(_env: Env, _admin: Address, _treasury: Address) -> Result<(), ContractError> {
        todo!("Treasury management not yet implemented")
    }
}

#[cfg(test)]
mod registration_tests;
