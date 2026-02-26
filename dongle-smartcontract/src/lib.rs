#![no_std]

mod admin_manager;
mod constants;
mod errors;
mod events;
mod fee_manager;
mod project_registry;
mod rating_calculator;
mod review_registry;
mod storage_keys;
mod types;
mod utils;
mod verification_registry;

#[cfg(test)]
mod test;

#[cfg(test)]
mod admin_tests;

use crate::admin_manager::AdminManager;
use crate::errors::ContractError;
use crate::fee_manager::FeeManager;
use crate::project_registry::ProjectRegistry;
use crate::review_registry::ReviewRegistry;
use crate::types::{
    FeeConfig, Project, ProjectRegistrationParams, ProjectUpdateParams, Review, VerificationRecord,
};
use crate::verification_registry::VerificationRegistry;
use soroban_sdk::{contract, contractimpl, Address, Env, String, Vec};

#[contract]
pub struct DongleContract;

#[contractimpl]
impl DongleContract {
    pub fn initialize(env: Env, admin: Address) {
        admin.require_auth();
        env.storage()
            .persistent()
            .set(&crate::storage_keys::StorageKey::Admin, &admin);
    }

    #[allow(clippy::too_many_arguments)]
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

    #[allow(clippy::too_many_arguments)]
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
    ) -> Option<Project> {
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
        params: ProjectRegistrationParams,
    ) -> Result<u64, ContractError> {
        ProjectRegistry::register_project(&env, params)
    }

    pub fn update_project(env: Env, params: ProjectUpdateParams) -> Option<Project> {
        ProjectRegistry::update_project(&env, params)
    }

    pub fn get_project(env: Env, project_id: u64) -> Option<Project> {
        ProjectRegistry::get_project(&env, project_id)
    }

    pub fn list_projects(env: Env, start_id: u64, limit: u32) -> Vec<Project> {
        ProjectRegistry::list_projects(&env, start_id, limit)
    }

    pub fn get_projects_by_owner(env: Env, owner: Address) -> Vec<Project> {
        ProjectRegistry::get_projects_by_owner(&env, owner)
    }

    pub fn get_owner_project_count(env: Env, owner: Address) -> u32 {
        ProjectRegistry::get_projects_by_owner(&env, owner).len()
    }

    pub fn add_review(
        env: Env,
        project_id: u64,
        reviewer: Address,
        rating: u32,
        comment_cid: Option<String>,
    ) -> Result<(), ContractError> {
        ReviewRegistry::add_review(&env, project_id, reviewer, rating, comment_cid)
    }

    pub fn update_review(
        env: Env,
        project_id: u64,
        reviewer: Address,
        rating: u32,
        comment_cid: Option<String>,
    ) -> Result<(), ContractError> {
        ReviewRegistry::update_review(&env, project_id, reviewer, rating, comment_cid)
    }

    pub fn delete_review(
        env: Env,
        project_id: u64,
        reviewer: Address,
    ) -> Result<(), ContractError> {
        ReviewRegistry::delete_review(&env, project_id, reviewer)
    }

    pub fn get_review(env: Env, project_id: u64, reviewer: Address) -> Option<Review> {
        ReviewRegistry::get_review(&env, project_id, reviewer)
    }

    pub fn list_reviews(env: Env, project_id: u64, start_id: u32, limit: u32) -> Vec<Review> {
        ReviewRegistry::list_reviews(&env, project_id, start_id, limit)
    }

    pub fn request_verification(
        env: Env,
        project_id: u64,
        requester: Address,
        evidence_cid: String,
    ) -> Result<(), ContractError> {
        VerificationRegistry::request_verification(&env, project_id, requester, evidence_cid)
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

    pub fn set_fee(
        env: Env,
        admin: Address,
        token: Option<Address>,
        amount: u128,
        treasury: Address,
    ) -> Result<(), ContractError> {
        FeeManager::set_fee(&env, admin, token, amount, treasury)
    }

    pub fn pay_fee(
        env: Env,
        payer: Address,
        project_id: u64,
        token: Option<Address>,
    ) -> Result<(), ContractError> {
        FeeManager::pay_fee(&env, payer, project_id, token)
    }

    pub fn get_fee_config(env: Env) -> Result<FeeConfig, ContractError> {
        FeeManager::get_fee_config(&env)
    }
}

#[cfg(test)]
mod test;

    // --- Admin Management ---

    pub fn initialize(env: Env, admin: Address) {
        AdminManager::initialize(&env, admin);
    }

    pub fn add_admin(env: Env, caller: Address, new_admin: Address) {
        let _ = AdminManager::add_admin(&env, caller, new_admin);
    }

    pub fn remove_admin(env: Env, caller: Address, admin_to_remove: Address) {
        let _ = AdminManager::remove_admin(&env, caller, admin_to_remove);
    }

    pub fn is_admin(env: Env, address: Address) -> bool {
        AdminManager::is_admin(&env, &address)
    }

    pub fn get_admin_list(env: Env) -> Vec<Address> {
        AdminManager::get_admin_list(&env)
    }

    pub fn get_admin_count(env: Env) -> u32 {
        AdminManager::get_admin_count(&env)
    }

    pub fn set_admin(env: Env, admin: Address) {
        env.storage()
            .persistent()
            .set(&crate::types::DataKey::Admin(admin), &());
    }
}
#[cfg(test)]
mod registration_tests;

#[cfg(test)]
mod verification_tests;
