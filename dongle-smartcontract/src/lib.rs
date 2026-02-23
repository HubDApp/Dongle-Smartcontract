#![no_std]

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

use errors::Error;
use types::{FeeConfig, Project, Review, VerificationRecord};

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
    ) -> Result<(), Error> {
        project_registry::ProjectRegistry::update_project(
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

    pub fn get_project(env: Env, project_id: u64) -> Result<Project, Error> {
        project_registry::ProjectRegistry::get_project(&env, project_id)?
            .ok_or(Error::ProjectNotFound)
    }

    pub fn list_projects(_env: Env, _start_id: u64, _limit: u32) -> Result<Vec<Project>, Error> {
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
        env: Env,
        project_id: u64,
        requester: Address,
        evidence_cid: String,
    ) -> Result<(), Error> {
        todo!("Verification requests not yet implemented")
    }

    pub fn approve_verification(env: Env, project_id: u64, admin: Address) -> Result<(), Error> {
        todo!("Verification approval not yet implemented")
    }

    pub fn reject_verification(env: Env, project_id: u64, admin: Address) -> Result<(), Error> {
        todo!("Verification rejection not yet implemented")
    }

    pub fn get_verification(env: Env, project_id: u64) -> Result<VerificationRecord, Error> {
        todo!("Verification record retrieval not yet implemented")
    }

    pub fn set_fee_config(
        env: Env,
        admin: Address,
        token: Option<Address>,
        verification_fee: u128,
        registration_fee: u128,
    ) -> Result<(), Error> {
        todo!("Fee configuration not yet implemented")
    }

    pub fn get_fee_config(env: Env) -> Result<FeeConfig, Error> {
        todo!("Fee configuration retrieval not yet implemented")
    }

    pub fn set_treasury(env: Env, admin: Address, treasury: Address) -> Result<(), Error> {
        todo!("Treasury management not yet implemented")
    }

    pub fn get_treasury(env: Env) -> Result<Address, Error> {
        todo!("Treasury address retrieval not yet implemented")
    }
}
