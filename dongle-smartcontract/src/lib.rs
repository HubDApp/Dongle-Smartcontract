#![no_std]
#![allow(warnings)]

mod admin_action_log;
mod admin_manager;
pub mod pagination;
pub mod auth;
mod bookmark_registry;
mod collection_registry;
pub mod constants;
mod dependency_registry;
mod dispute_registry;
mod emergency_pause;
mod endorsement_registry;
pub mod errors;
pub mod events;
mod featured_registry;
mod fee_manager;
mod project_registry;
pub mod rating_calculator;
mod report_registry;
pub mod review_registry;
pub mod storage_keys;
pub mod storage_manager;
mod subscription_registry;
mod timelock_manager;
pub mod types;
pub mod utils;
mod verification_registry;

#[cfg(test)]
mod tests;

use crate::admin_action_log::AdminActionLog;
use crate::admin_manager::AdminManager;
use crate::collection_registry::CollectionRegistry;
use crate::emergency_pause::EmergencyPause;
use crate::errors::ContractError;
use crate::featured_registry::FeaturedRegistry;
use crate::fee_manager::FeeManager;
use crate::project_registry::ProjectRegistry;
use crate::report_registry::ReportRegistry;
use crate::review_registry::ReviewRegistry;
use crate::storage_keys::ExtensionKey;
use crate::storage_manager::StorageManager;
use crate::timelock_manager::TimelockManager;
use crate::types::{
    AdminActionEntry, AdminProposal, ClaimRequest, ClaimStatus, Collection, ContractClaimRequest,
    DependencyRef, DisputeResolutionAction, DisputeStatus, DuplicateDispute, FeeConfig,
    FeePaymentRecord, Project, ProjectDependency, ProjectRegistrationParams, ProjectReport,
    ProjectSortMode, ProjectStats, ProjectUpdateParams, ProposalPayload, Review,
    ReviewEligibilityConfig, ReviewRevision, ReviewSortMode, ReviewTombstone,
    SecurityContactStatus, TimelockAction, VerificationRecord, VerificationStatus,
};
use crate::verification_registry::VerificationRegistry;
use soroban_sdk::{contract, contractimpl, Address, Env, String, Vec};

#[contract]
pub struct DongleContract;

#[contractimpl]
impl DongleContract {
    // --- Initialization & Admin Management ---

    pub fn initialize(env: Env, admin: Address) {
        AdminManager::initialize(&env, admin);
    }

    pub fn add_admin(env: Env, caller: Address, new_admin: Address) -> Result<(), ContractError> {
        AdminManager::add_admin(&env, caller, new_admin)
    }

    pub fn remove_admin(
        env: Env,
        caller: Address,
        admin_to_remove: Address,
    ) -> Result<(), ContractError> {
        AdminManager::remove_admin(&env, caller, admin_to_remove)
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

    pub fn get_admin_approval_threshold(env: Env) -> u32 {
        AdminManager::get_admin_approval_threshold(&env)
    }

    pub fn set_admin_approval_threshold(
        env: Env,
        caller: Address,
        threshold: u32,
    ) -> Result<(), ContractError> {
        AdminManager::set_admin_approval_threshold(&env, caller, threshold)
    }

    pub fn create_proposal(
        env: Env,
        proposer: Address,
        payload: ProposalPayload,
    ) -> Result<u64, ContractError> {
        AdminManager::create_proposal(&env, proposer, payload)
    }

    pub fn approve_proposal(
        env: Env,
        admin: Address,
        proposal_id: u64,
    ) -> Result<(), ContractError> {
        AdminManager::approve_proposal(&env, admin, proposal_id)
    }

    pub fn execute_proposal(
        env: Env,
        caller: Address,
        proposal_id: u64,
    ) -> Result<(), ContractError> {
        AdminManager::execute_proposal(&env, caller, proposal_id)
    }

    pub fn get_proposal(env: Env, proposal_id: u64) -> Option<AdminProposal> {
        AdminManager::get_proposal(&env, proposal_id)
    }

    // --- Contract Pause / Emergency Stop ---

    /// Pause the contract (admin-only). All non-admin mutating operations will fail.
    pub fn pause(env: Env, admin: Address) -> Result<(), ContractError> {
        EmergencyPause::pause(&env, &admin)
    }

    /// Unpause the contract (admin-only). Restores normal operation.
    pub fn unpause(env: Env, admin: Address) -> Result<(), ContractError> {
        EmergencyPause::unpause(&env, &admin)
    }

    /// Returns true if the contract is currently paused.
    pub fn is_paused(env: Env) -> bool {
        EmergencyPause::is_paused(&env)
    }

    // --- Project Registry ---

    pub fn register_project(
        env: Env,
        params: ProjectRegistrationParams,
    ) -> Result<u64, ContractError> {
        EmergencyPause::require_not_paused(&env)?;
        ProjectRegistry::register_project(&env, params)
    }

    pub fn update_project(env: Env, params: ProjectUpdateParams) -> Result<Project, ContractError> {
        EmergencyPause::require_not_paused(&env)?;
        ProjectRegistry::update_project(&env, params)
    }

    pub fn update_security_contact(
        env: Env,
        project_id: u64,
        caller: Address,
        contact: Option<String>,
    ) -> Result<Project, ContractError> {
        EmergencyPause::require_not_paused(&env)?;
        ProjectRegistry::update_security_contact(&env, project_id, caller, contact)
    }

    pub fn submit_security_contact_proof(
        env: Env,
        project_id: u64,
        caller: Address,
        proof_cid: String,
    ) -> Result<Project, ContractError> {
        EmergencyPause::require_not_paused(&env)?;
        ProjectRegistry::submit_security_contact_proof(&env, project_id, caller, proof_cid)
    }

    pub fn get_security_contact_status(
        env: Env,
        project_id: u64,
    ) -> Result<SecurityContactStatus, ContractError> {
        ProjectRegistry::get_security_contact_status(&env, project_id)
    }

    pub fn link_project(
        env: Env,
        project_id: u64,
        caller: Address,
        linked_project_id: u64,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env)?;
        ProjectRegistry::link_project(&env, project_id, caller, linked_project_id)
    }

    pub fn unlink_project(
        env: Env,
        project_id: u64,
        caller: Address,
        linked_project_id: u64,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env)?;
        ProjectRegistry::unlink_project(&env, project_id, caller, linked_project_id)
    }

    pub fn get_linked_projects(env: Env, project_id: u64) -> Vec<u64> {
        ProjectRegistry::get_linked_projects(&env, project_id)
    }

    pub fn get_project(env: Env, project_id: u64) -> Option<Project> {
        ProjectRegistry::get_project(&env, project_id)
    }

    pub fn get_project_by_slug(env: Env, slug: String) -> Option<Project> {
        ProjectRegistry::get_project_by_slug(&env, slug)
    }

    pub fn initiate_transfer(
        env: Env,
        project_id: u64,
        caller: Address,
        new_owner: Address,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env)?;
        ProjectRegistry::initiate_transfer(&env, project_id, caller, new_owner)
    }

    pub fn cancel_transfer(
        env: Env,
        project_id: u64,
        caller: Address,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env)?;
        ProjectRegistry::cancel_transfer(&env, project_id, caller)
    }

    pub fn accept_transfer(
        env: Env,
        project_id: u64,
        caller: Address,
    ) -> Result
