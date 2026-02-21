//! Dongle smart contract: project registry, reviews, verification, and fees.
//! Security: descriptive errors, input validation, per-user limits, consistent events.

mod constants;
mod errors;
mod events;
mod fee_manager;
mod project_registry;
mod review_registry;
mod storage_keys;
mod types;
mod verification_registry;

pub use errors::Error;
pub use events::*;
pub use types::*;

use soroban_sdk::{contract, contractimpl, Address, Env};

#[contract]
pub struct DongleContract;

#[contractimpl]
impl DongleContract {
    // ---- Admin (modular: can be extended or moved to governance) ----
    pub fn set_admin(env: Env, admin: Address) {
        fee_manager::FeeManager::set_admin(&env, admin);
    }

    pub fn set_fee(
        env: Env,
        admin: Address,
        token: Option<Address>,
        amount: u128,
        treasury: Address,
    ) -> Result<(), Error> {
        fee_manager::FeeManager::set_fee(&env, admin, token, amount, treasury)
    }

    // ---- Project registry ----
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

    pub fn get_project(env: Env, project_id: u64) -> Result<Option<Project>, Error> {
        project_registry::ProjectRegistry::get_project(&env, project_id)
    }

    pub fn get_owner_project_count(env: Env, owner: Address) -> u32 {
        project_registry::ProjectRegistry::get_owner_project_count(&env, &owner)
    }

    // ---- Review registry ----
    pub fn add_review(
        env: Env,
        project_id: u64,
        reviewer: Address,
        rating: u32,
        comment_cid: Option<String>,
    ) -> Result<(), Error> {
        review_registry::ReviewRegistry::add_review(&env, project_id, reviewer, rating, comment_cid)
    }

    pub fn update_review(
        env: Env,
        project_id: u64,
        reviewer: Address,
        rating: u32,
        comment_cid: Option<String>,
    ) -> Result<(), Error> {
        review_registry::ReviewRegistry::update_review(
            &env, project_id, reviewer, rating, comment_cid,
        )
    }

    pub fn get_review(
        env: Env,
        project_id: u64,
        reviewer: Address,
    ) -> Option<Review> {
        review_registry::ReviewRegistry::get_review(&env, project_id, reviewer)
    }

    // ---- Verification registry ----
    pub fn request_verification(
        env: Env,
        project_id: u64,
        requester: Address,
        evidence_cid: String,
    ) -> Result<(), Error> {
        verification_registry::VerificationRegistry::request_verification(
            &env, project_id, requester, evidence_cid,
        )
    }

    pub fn approve_verification(
        env: Env,
        project_id: u64,
        verifier: Address,
    ) -> Result<(), Error> {
        verification_registry::VerificationRegistry::approve_verification(
            &env, project_id, verifier,
        )
    }

    pub fn reject_verification(
        env: Env,
        project_id: u64,
        verifier: Address,
    ) -> Result<(), Error> {
        verification_registry::VerificationRegistry::reject_verification(
            &env, project_id, verifier,
        )
    }

    pub fn get_verification(env: Env, project_id: u64) -> Option<VerificationRecord> {
        verification_registry::VerificationRegistry::get_verification(&env, project_id)
    }

    // ---- Fee manager ----
    pub fn pay_fee(
        env: Env,
        payer: Address,
        project_id: u64,
        token: Option<Address>,
    ) -> Result<(), Error> {
        fee_manager::FeeManager::pay_fee(&env, payer, project_id, token)
    }

    pub fn get_fee_config(env: Env) -> Result<FeeConfig, Error> {
        fee_manager::FeeManager::get_fee_config(&env)
    }
}

#[cfg(test)]
mod test;
