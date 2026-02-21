#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Env, String};

mod types;
mod errors;
mod utils;
mod project_registry;
mod review_registry;
mod verification_registry;
mod fee_manager;

#[cfg(test)]
mod test;

#[contract]
pub struct DongleContract;

#[contractimpl]
impl DongleContract {
    /// Initialize the contract with an admin.
    pub fn init(env: Env, admin: Address) {
        utils::set_admin(&env, &admin);
    }

    /// Register a new project.
    pub fn register_project(
        env: Env,
        owner: Address,
        name: String,
        description: String,
        category: String,
        website: Option<String>,
        logo_cid: Option<String>,
        metadata_cid: Option<String>,
    ) -> u64 {
        owner.require_auth();
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

    /// Get project details.
    pub fn get_project(env: Env, project_id: u64) -> Option<types::Project> {
        project_registry::ProjectRegistry::get_project(&env, project_id)
    }

    /// Add a review to a project.
    pub fn add_review(
        env: Env,
        reviewer: Address,
        project_id: u64,
        rating: u32,
        comment_cid: Option<String>,
    ) {
        reviewer.require_auth();
        review_registry::ReviewRegistry::add_review(&env, project_id, reviewer, rating, comment_cid)
    }

    /// Approve a project verification.
    pub fn approve_verification(env: Env, admin: Address, project_id: u64) {
        admin.require_auth();
        utils::check_admin(&env, &admin);
        verification_registry::VerificationRegistry::approve_verification(&env, project_id, admin)
    }
}
