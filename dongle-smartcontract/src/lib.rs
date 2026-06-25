#![no_std]
#![allow(warnings)]

mod admin_action_log;
mod admin_manager;
pub mod auth;
mod bookmark_registry;
mod collection_registry;
pub mod constants;
mod dependency_registry;
mod dispute_registry;
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
use crate::errors::ContractError;
use crate::featured_registry::FeaturedRegistry;
use crate::fee_manager::FeeManager;
use crate::project_registry::ProjectRegistry;
use crate::report_registry::ReportRegistry;
use crate::review_registry::ReviewRegistry;
use crate::storage_manager::StorageManager;
use crate::timelock_manager::TimelockManager;
use crate::types::{
    AdminActionEntry, AdminProposal, ClaimRequest, ClaimStatus, Collection, DependencyRef,
    DisputeResolutionAction, DisputeStatus, DuplicateDispute, FeeConfig, Project,
    ProjectDependency, ProjectRegistrationParams, ProjectReport, ProjectStats, ProjectUpdateParams,
    ProposalPayload, Review, TimelockAction, VerificationRecord, VerificationStatus,
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

    // --- Project Registry ---

    pub fn register_project(
        env: Env,
        params: ProjectRegistrationParams,
    ) -> Result<u64, ContractError> {
        ProjectRegistry::register_project(&env, params)
    }

    pub fn update_project(env: Env, params: ProjectUpdateParams) -> Result<Project, ContractError> {
        ProjectRegistry::update_project(&env, params)
    }

    pub fn link_project(
        env: Env,
        project_id: u64,
        caller: Address,
        linked_project_id: u64,
    ) -> Result<(), ContractError> {
        ProjectRegistry::link_project(&env, project_id, caller, linked_project_id)
    }

    pub fn unlink_project(
        env: Env,
        project_id: u64,
        caller: Address,
        linked_project_id: u64,
    ) -> Result<(), ContractError> {
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
        ProjectRegistry::initiate_transfer(&env, project_id, caller, new_owner)
    }

    pub fn cancel_transfer(
        env: Env,
        project_id: u64,
        caller: Address,
    ) -> Result<(), ContractError> {
        ProjectRegistry::cancel_transfer(&env, project_id, caller)
    }

    pub fn accept_transfer(
        env: Env,
        project_id: u64,
        caller: Address,
    ) -> Result<(), ContractError> {
        ProjectRegistry::accept_transfer(&env, project_id, caller)
    }

    pub fn list_projects(env: Env, start_id: u64, limit: u32) -> Vec<Project> {
        ProjectRegistry::list_projects(&env, start_id, limit)
    }

    pub fn get_projects_by_owner(env: Env, owner: Address) -> Vec<Project> {
        ProjectRegistry::get_projects_by_owner(&env, owner)
    }

    pub fn get_owner_project_count(env: Env, owner: Address) -> u32 {
        ProjectRegistry::get_owner_project_count(&env, &owner)
    }

    pub fn get_project_count(env: Env) -> u64 {
        ProjectRegistry::get_project_count(&env)
    }

    pub fn get_projects_by_ids(env: Env, ids: Vec<u64>) -> Vec<Project> {
        ProjectRegistry::get_projects_by_ids(&env, ids)
    }

    pub fn list_projects_by_status(
        env: Env,
        status: VerificationStatus,
        start_id: u64,
        limit: u32,
    ) -> Vec<Project> {
        ProjectRegistry::list_projects_by_status(&env, status, start_id, limit)
    }

    pub fn list_projects_by_category(
        env: Env,
        category: String,
        start_id: u32,
        limit: u32,
    ) -> Vec<Project> {
        ProjectRegistry::list_projects_by_category(&env, category, start_id, limit)
    }

    pub fn archive_project(
        env: Env,
        project_id: u64,
        caller: Address,
    ) -> Result<(), ContractError> {
        ProjectRegistry::archive_project(&env, project_id, caller)
    }

    pub fn reactivate_project(
        env: Env,
        project_id: u64,
        caller: Address,
    ) -> Result<(), ContractError> {
        ProjectRegistry::reactivate_project(&env, project_id, caller)
    }

    pub fn add_maintainer(
        env: Env,
        project_id: u64,
        caller: Address,
        maintainer: Address,
    ) -> Result<(), ContractError> {
        ProjectRegistry::add_maintainer(&env, project_id, caller, maintainer)
    }

    pub fn remove_maintainer(
        env: Env,
        project_id: u64,
        caller: Address,
        maintainer: Address,
    ) -> Result<(), ContractError> {
        ProjectRegistry::remove_maintainer(&env, project_id, caller, maintainer)
    }

    pub fn get_maintainers(env: Env, project_id: u64) -> Vec<Address> {
        ProjectRegistry::get_maintainers(&env, project_id)
    }

    // --- Featured Registry ---

    pub fn set_featured(
        env: Env,
        admin: Address,
        project_id: u64,
        featured: bool,
    ) -> Result<(), ContractError> {
        FeaturedRegistry::set_featured(&env, admin, project_id, featured)
    }

    pub fn list_featured_projects(env: Env, start: u32, limit: u32) -> Vec<Project> {
        FeaturedRegistry::list_featured_projects(&env, start, limit)
    }

    // --- Review Registry ---

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

    pub fn submit_review(
        env: Env,
        project_id: u64,
        reviewer: Address,
        rating: u32,
        review_cid: String,
    ) -> Result<(), ContractError> {
        ReviewRegistry::submit_review(&env, project_id, reviewer, rating, review_cid)
    }

    pub fn respond_to_review(
        env: Env,
        project_id: u64,
        caller: Address,
        reviewer: Address,
        response: String,
    ) -> Result<(), ContractError> {
        ReviewRegistry::respond_to_review(&env, project_id, caller, reviewer, response)
    }

    pub fn get_review_response(env: Env, project_id: u64, reviewer: Address) -> Option<String> {
        ReviewRegistry::get_review_response(&env, project_id, reviewer)
    }

    pub fn get_review(env: Env, project_id: u64, reviewer: Address) -> Option<Review> {
        ReviewRegistry::get_review(&env, project_id, reviewer)
    }

    pub fn get_review_cid(env: Env, project_id: u64, reviewer: Address) -> Option<String> {
        ReviewRegistry::get_review_cid(&env, project_id, reviewer)
    }

    pub fn get_project_review_cids(env: Env, project_id: u64) -> Vec<(Address, String)> {
        ReviewRegistry::get_project_review_cids(&env, project_id)
    }

    pub fn get_reviews_by_ids(env: Env, ids: Vec<(u64, Address)>) -> Vec<Review> {
        ReviewRegistry::get_reviews_by_ids(&env, ids)
    }

    pub fn list_reviews(env: Env, project_id: u64, start_id: u32, limit: u32) -> Vec<Review> {
        ReviewRegistry::list_reviews(&env, project_id, start_id, limit)
    }

    pub fn get_project_stats(env: Env, project_id: u64) -> ProjectStats {
        ReviewRegistry::get_project_stats(&env, project_id)
    }

    pub fn get_stats_batch(env: Env, ids: Vec<u64>) -> Vec<(u64, ProjectStats)> {
        ReviewRegistry::get_stats_batch(&env, ids)
    }

    pub fn set_reviews_enabled(
        env: Env,
        project_id: u64,
        caller: Address,
        enabled: bool,
    ) -> Result<(), ContractError> {
        ReviewRegistry::set_reviews_enabled(&env, project_id, caller, enabled)
    }

    pub fn get_reviews_enabled(env: Env, project_id: u64) -> bool {
        ReviewRegistry::get_reviews_enabled(&env, project_id)
    }

    pub fn report_review(
        env: Env,
        project_id: u64,
        reviewer: Address,
        reporter: Address,
    ) -> Result<(), ContractError> {
        ReviewRegistry::report_review(&env, project_id, reviewer, reporter)
    }

    pub fn hide_review(
        env: Env,
        project_id: u64,
        reviewer: Address,
        admin: Address,
    ) -> Result<(), ContractError> {
        ReviewRegistry::hide_review(&env, project_id, reviewer, admin)
    }

    pub fn restore_review(
        env: Env,
        project_id: u64,
        reviewer: Address,
        admin: Address,
    ) -> Result<(), ContractError> {
        ReviewRegistry::restore_review(&env, project_id, reviewer, admin)
    }

    /// Admin hard-delete a review permanently (admin-only).
    pub fn admin_delete_review(
        env: Env,
        project_id: u64,
        reviewer: Address,
        admin: Address,
    ) -> Result<(), ContractError> {
        ReviewRegistry::admin_delete_review(&env, project_id, reviewer, admin)
    }

    // --- Verification Registry ---

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

    pub fn revoke_verification(
        env: Env,
        project_id: u64,
        admin: Address,
        reason: String,
    ) -> Result<(), ContractError> {
        VerificationRegistry::revoke_verification(&env, project_id, admin, reason)
    }

    pub fn get_verification(
        env: Env,
        project_id: u64,
    ) -> Result<VerificationRecord, ContractError> {
        VerificationRegistry::get_verification(&env, project_id)
    }

    pub fn get_verification_record(
        env: Env,
        request_id: u64,
    ) -> Result<VerificationRecord, ContractError> {
        VerificationRegistry::get_verification_record(&env, request_id)
    }

    pub fn get_verifications_batch(env: Env, ids: Vec<u64>) -> Vec<(u64, VerificationRecord)> {
        VerificationRegistry::get_verifications_batch(&env, ids)
    }

    pub fn get_verification_history(env: Env, project_id: u64) -> Vec<VerificationRecord> {
        VerificationRegistry::get_verification_history(&env, project_id)
    }

    pub fn request_renewal(
        env: Env,
        project_id: u64,
        requester: Address,
        evidence_cid: String,
    ) -> Result<(), ContractError> {
        VerificationRegistry::request_renewal(&env, project_id, requester, evidence_cid)
    }

    pub fn approve_renewal(env: Env, project_id: u64, admin: Address) -> Result<(), ContractError> {
        VerificationRegistry::approve_renewal(&env, project_id, admin)
    }

    pub fn reject_renewal(env: Env, project_id: u64, admin: Address) -> Result<(), ContractError> {
        VerificationRegistry::reject_renewal(&env, project_id, admin)
    }

    pub fn get_renewal_request(
        env: Env,
        project_id: u64,
    ) -> Result<crate::types::VerificationRenewalRecord, ContractError> {
        VerificationRegistry::get_renewal_request(&env, project_id)
    }

    pub fn get_renewal_history(
        env: Env,
        project_id: u64,
        start_index: u32,
        limit: u32,
    ) -> Vec<crate::types::VerificationRenewalRecord> {
        VerificationRegistry::get_renewal_history(&env, project_id, start_index, limit)
    }

    pub fn is_verification_expired(env: Env, project_id: u64) -> Result<bool, ContractError> {
        VerificationRegistry::is_verification_expired(&env, project_id)
    }

    /// Admin: prune verification history, keeping the most recent `keep_count` records.
    /// Returns the number of records removed.
    pub fn clear_verification_history(
        env: Env,
        project_id: u64,
        admin: Address,
        keep_count: u32,
    ) -> Result<u32, ContractError> {
        VerificationRegistry::clear_verification_history(&env, project_id, &admin, keep_count)
    }

    /// Admin: clear all renewal history records for a project.
    /// Returns the number of records removed.
    pub fn clear_renewal_history(
        env: Env,
        project_id: u64,
        admin: Address,
    ) -> Result<u32, ContractError> {
        VerificationRegistry::clear_renewal_history(&env, project_id, &admin)
    }

    // --- Fee Manager ---

    pub fn set_fee(
        env: Env,
        admin: Address,
        token: Option<Address>,
        verification_fee: u128,
        registration_fee: u128,
        treasury: Address,
    ) -> Result<(), ContractError> {
        FeeManager::set_fee(
            &env,
            admin,
            token,
            verification_fee,
            registration_fee,
            treasury,
        )
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

    // --- TTL Management ---

    /// Extend TTL for a specific project and its related data
    pub fn extend_project_ttl(env: Env, project_id: u64) {
        if let Some(project) = ProjectRegistry::get_project(&env, project_id) {
            StorageManager::extend_project_full_ttl(&env, project_id, &project.name);
        }
    }

    /// Extend TTL for a specific review
    pub fn extend_review_ttl(env: Env, project_id: u64, reviewer: Address) {
        StorageManager::extend_review_ttl(&env, project_id, &reviewer);
    }

    /// Extend TTL for all admin-related data
    pub fn extend_admin_ttl(env: Env, admin: Address) {
        StorageManager::extend_all_admin_ttl(&env, &admin);
    }

    /// Extend TTL for critical contract configuration (admin list, fee config, treasury)
    pub fn extend_critical_config_ttl(env: Env) {
        StorageManager::extend_critical_config_ttl(&env);
    }

    /// Extend TTL for user-related data (owner projects, user reviews)
    pub fn extend_user_ttl(env: Env, user: Address) {
        StorageManager::extend_owner_projects_ttl(&env, &user);
        StorageManager::extend_user_reviews_ttl(&env, &user);
    }

    /// Extend TTL for verification data
    pub fn extend_verification_ttl(env: Env, project_id: u64) {
        StorageManager::extend_verification_ttl(&env, project_id);
        StorageManager::extend_fee_paid_ttl(&env, project_id);
    }

    // --- New Features ---

    /// Set minimum project age before verification (admin only) - Issue #130
    pub fn set_min_project_age(
        env: Env,
        admin: Address,
        min_age_seconds: u64,
    ) -> Result<(), ContractError> {
        VerificationRegistry::set_min_project_age(&env, admin, min_age_seconds)
    }

    /// Get minimum project age configuration - Issue #130
    pub fn get_min_project_age(env: Env) -> u64 {
        VerificationRegistry::get_min_project_age(&env)
    }

    /// Set verification duration (admin only)
    pub fn set_verification_duration(
        env: Env,
        admin: Address,
        duration_seconds: u64,
    ) -> Result<(), ContractError> {
        VerificationRegistry::set_verification_duration(&env, admin, duration_seconds)
    }

    /// Get verification duration configuration
    pub fn get_verification_duration(env: Env) -> u64 {
        VerificationRegistry::get_verification_duration(&env)
    }

    /// Report a project for spam, scams, broken links, or abusive metadata - Issue #127
    pub fn report_project(
        env: Env,
        project_id: u64,
        reporter: Address,
        reason_cid: String,
    ) -> Result<(), ContractError> {
        ReportRegistry::report_project(&env, project_id, reporter, reason_cid)
    }

    /// Get all reports for a project - Issue #127
    pub fn get_project_reports(env: Env, project_id: u64) -> Vec<ProjectReport> {
        ReportRegistry::get_project_reports(&env, project_id)
    }

    /// Get report count for a project - Issue #127
    pub fn get_project_report_count(env: Env, project_id: u64) -> u32 {
        ReportRegistry::get_project_report_count(&env, project_id)
    }

    /// Check if a user has already reported a project - Issue #127
    pub fn has_user_reported(env: Env, project_id: u64, reporter: Address) -> bool {
        ReportRegistry::has_user_reported(&env, project_id, &reporter)
    }

    /// Admin: clear all reports for a project (admin-only).
    pub fn clear_project_reports(
        env: Env,
        project_id: u64,
        admin: Address,
    ) -> Result<(), ContractError> {
        ReportRegistry::clear_project_reports(&env, project_id, &admin)
    }

    /// List projects by tag - Issue #125
    pub fn list_projects_by_tag(env: Env, tag: String, start_id: u32, limit: u32) -> Vec<Project> {
        ProjectRegistry::list_projects_by_tag(&env, tag, start_id, limit)
    }

    // --- Collection Registry ---

    /// Admin: create a new curated collection of projects.
    pub fn create_collection(
        env: Env,
        admin: Address,
        name: String,
        description: String,
    ) -> Result<u64, ContractError> {
        CollectionRegistry::create_collection(&env, admin, name, description)
    }

    /// Admin: update a collection's name and description.
    pub fn update_collection(
        env: Env,
        admin: Address,
        collection_id: u64,
        name: String,
        description: String,
    ) -> Result<(), ContractError> {
        CollectionRegistry::update_collection(&env, admin, collection_id, name, description)
    }

    /// Admin: delete a collection and its project associations.
    pub fn delete_collection(
        env: Env,
        admin: Address,
        collection_id: u64,
    ) -> Result<(), ContractError> {
        CollectionRegistry::delete_collection(&env, admin, collection_id)
    }

    /// Admin: add a project to a collection.
    pub fn add_project_to_collection(
        env: Env,
        admin: Address,
        collection_id: u64,
        project_id: u64,
    ) -> Result<(), ContractError> {
        CollectionRegistry::add_project_to_collection(&env, admin, collection_id, project_id)
    }

    /// Admin: remove a project from a collection.
    pub fn remove_project_from_collection(
        env: Env,
        admin: Address,
        collection_id: u64,
        project_id: u64,
    ) -> Result<(), ContractError> {
        CollectionRegistry::remove_project_from_collection(&env, admin, collection_id, project_id)
    }

    /// Get a collection by ID.
    pub fn get_collection(env: Env, collection_id: u64) -> Result<Collection, ContractError> {
        CollectionRegistry::get_collection(&env, collection_id)
    }

    /// List all collections with pagination.
    pub fn list_collections(env: Env, start: u32, limit: u32) -> Vec<Collection> {
        CollectionRegistry::list_collections(&env, start, limit)
    }

    /// List project IDs in a collection with pagination.
    pub fn list_collection_projects(
        env: Env,
        collection_id: u64,
        start: u32,
        limit: u32,
    ) -> Vec<u64> {
        CollectionRegistry::list_collection_projects(&env, collection_id, start, limit)
    }

    /// Get the number of projects in a collection.
    pub fn get_collection_project_count(env: Env, collection_id: u64) -> u32 {
        CollectionRegistry::get_collection_project_count(&env, collection_id)
    }

    /// Get the total number of collections.
    pub fn get_collection_count(env: Env) -> u64 {
        CollectionRegistry::get_collection_count(&env)
    }

    // --- Admin Action Log ---

    /// Get a single admin action log entry by ID.
    pub fn get_admin_action_log_entry(env: Env, log_id: u64) -> Option<AdminActionEntry> {
        AdminActionLog::get_log_entry(&env, log_id)
    }

    /// List admin action log entries with pagination (most recent first).
    pub fn list_admin_actions(env: Env, start: u32, limit: u32) -> Vec<AdminActionEntry> {
        AdminActionLog::list_admin_actions(&env, start, limit)
    }

    /// Get the total number of admin action log entries.
    pub fn get_admin_action_log_count(env: Env) -> u64 {
        AdminActionLog::get_action_log_count(&env)
    }

    // --- Project Claiming ---

    pub fn set_project_claimable(
        env: Env,
        project_id: u64,
        caller: Address,
        claimable: bool,
    ) -> Result<(), ContractError> {
        ProjectRegistry::set_project_claimable(&env, project_id, caller, claimable)
    }

    pub fn submit_claim_request(
        env: Env,
        project_id: u64,
        claimant: Address,
        proof_cid: String,
    ) -> Result<u64, ContractError> {
        ProjectRegistry::submit_claim_request(&env, project_id, claimant, proof_cid)
    }

    pub fn approve_claim_request(
        env: Env,
        claim_request_id: u64,
        admin: Address,
    ) -> Result<(), ContractError> {
        ProjectRegistry::approve_claim_request(&env, claim_request_id, admin)
    }

    pub fn reject_claim_request(
        env: Env,
        claim_request_id: u64,
        admin: Address,
    ) -> Result<(), ContractError> {
        ProjectRegistry::reject_claim_request(&env, claim_request_id, admin)
    }

    pub fn get_claim_request(env: Env, claim_request_id: u64) -> Option<ClaimRequest> {
        ProjectRegistry::get_claim_request(&env, claim_request_id)
    }

    pub fn get_claim_requests_for_project(env: Env, project_id: u64) -> Vec<ClaimRequest> {
        ProjectRegistry::get_claim_requests_for_project(&env, project_id)
    }

    // --- Project Dependencies ---

    pub fn add_project_dependency(
        env: Env,
        project_id: u64,
        caller: Address,
        dependency: ProjectDependency,
    ) -> Result<(), ContractError> {
        crate::dependency_registry::DependencyRegistry::add_dependency(
            &env, project_id, caller, dependency,
        )
    }

    pub fn update_project_dependency(
        env: Env,
        project_id: u64,
        caller: Address,
        dependency_key: DependencyRef,
        new_dependency: ProjectDependency,
    ) -> Result<(), ContractError> {
        crate::dependency_registry::DependencyRegistry::update_dependency(
            &env,
            project_id,
            caller,
            dependency_key,
            new_dependency,
        )
    }

    pub fn remove_project_dependency(
        env: Env,
        project_id: u64,
        caller: Address,
        dependency_key: DependencyRef,
    ) -> Result<(), ContractError> {
        crate::dependency_registry::DependencyRegistry::remove_dependency(
            &env,
            project_id,
            caller,
            dependency_key,
        )
    }

    pub fn get_project_dependencies(env: Env, project_id: u64) -> Vec<ProjectDependency> {
        crate::dependency_registry::DependencyRegistry::get_dependencies(&env, project_id)
    }

    // --- Duplicate Disputes ---

    pub fn open_duplicate_dispute(
        env: Env,
        project_id: u64,
        original_project_id: u64,
        creator: Address,
        evidence_cid: String,
    ) -> Result<u64, ContractError> {
        crate::dispute_registry::DisputeRegistry::open_duplicate_dispute(
            &env,
            project_id,
            original_project_id,
            creator,
            evidence_cid,
        )
    }

    pub fn resolve_duplicate_dispute(
        env: Env,
        dispute_id: u64,
        admin: Address,
        action: DisputeResolutionAction,
    ) -> Result<(), ContractError> {
        crate::dispute_registry::DisputeRegistry::resolve_duplicate_dispute(
            &env, dispute_id, admin, action,
        )
    }

    pub fn get_duplicate_dispute(env: Env, dispute_id: u64) -> Option<DuplicateDispute> {
        crate::dispute_registry::DisputeRegistry::get_duplicate_dispute(&env, dispute_id)
    }

    pub fn get_disputes_for_project(env: Env, project_id: u64) -> Vec<DuplicateDispute> {
        crate::dispute_registry::DisputeRegistry::get_disputes_for_project(&env, project_id)
    }

    // --- Subscription / Follow ---

    pub fn follow_project(
        env: Env,
        project_id: u64,
        follower: Address,
    ) -> Result<(), ContractError> {
        crate::subscription_registry::SubscriptionRegistry::follow_project(
            &env, project_id, follower,
        )
    }

    pub fn unfollow_project(
        env: Env,
        project_id: u64,
        follower: Address,
    ) -> Result<(), ContractError> {
        crate::subscription_registry::SubscriptionRegistry::unfollow_project(
            &env, project_id, follower,
        )
    }

    pub fn get_follower_count(env: Env, project_id: u64) -> u32 {
        crate::subscription_registry::SubscriptionRegistry::get_follower_count(&env, project_id)
    }

    pub fn is_following(env: Env, project_id: u64, user: Address) -> bool {
        crate::subscription_registry::SubscriptionRegistry::is_following(&env, project_id, &user)
    }

    pub fn get_project_followers(
        env: Env,
        project_id: u64,
        start: u32,
        limit: u32,
    ) -> Vec<Address> {
        crate::subscription_registry::SubscriptionRegistry::get_project_followers(
            &env, project_id, start, limit,
        )
    }

    pub fn get_user_subscriptions(env: Env, user: Address, start: u32, limit: u32) -> Vec<u64> {
        crate::subscription_registry::SubscriptionRegistry::get_user_subscriptions(
            &env, user, start, limit,
        )
    }

    // --- Bookmark Registry ---

    pub fn bookmark_project(
        env: Env,
        project_id: u64,
        user: Address,
    ) -> Result<(), crate::bookmark_registry::BookmarkError> {
        crate::bookmark_registry::BookmarkRegistry::bookmark_project(&env, project_id, user)
    }

    pub fn unbookmark_project(
        env: Env,
        project_id: u64,
        user: Address,
    ) -> Result<(), crate::bookmark_registry::BookmarkError> {
        crate::bookmark_registry::BookmarkRegistry::unbookmark_project(&env, project_id, user)
    }

    pub fn is_bookmarked(env: Env, project_id: u64, user: Address) -> bool {
        crate::bookmark_registry::BookmarkRegistry::is_bookmarked(&env, project_id, &user)
    }

    pub fn get_user_bookmarks(env: Env, user: Address, start: u32, limit: u32) -> Vec<u64> {
        crate::bookmark_registry::BookmarkRegistry::get_user_bookmarks(&env, user, start, limit)
    }

    // --- Admin Timelock ---

    pub fn schedule_set_fee(
        env: Env,
        admin: Address,
        token: Option<Address>,
        verification_fee: u128,
        registration_fee: u128,
        treasury: Address,
        execution_timestamp: u64,
    ) -> Result<u64, ContractError> {
        TimelockManager::schedule_set_fee(
            &env,
            admin,
            token,
            verification_fee,
            registration_fee,
            treasury,
            execution_timestamp,
        )
    }

    pub fn schedule_add_admin(
        env: Env,
        admin: Address,
        new_admin: Address,
        execution_timestamp: u64,
    ) -> Result<u64, ContractError> {
        TimelockManager::schedule_add_admin(&env, admin, new_admin, execution_timestamp)
    }

    pub fn schedule_remove_admin(
        env: Env,
        admin: Address,
        admin_to_remove: Address,
        execution_timestamp: u64,
    ) -> Result<u64, ContractError> {
        TimelockManager::schedule_remove_admin(&env, admin, admin_to_remove, execution_timestamp)
    }

    pub fn cancel_scheduled_action(
        env: Env,
        caller: Address,
        action_id: u64,
    ) -> Result<(), ContractError> {
        TimelockManager::cancel_action(&env, caller, action_id)
    }

    pub fn execute_scheduled_set_fee(
        env: Env,
        caller: Address,
        action_id: u64,
    ) -> Result<(), ContractError> {
        TimelockManager::execute_set_fee(&env, caller, action_id)
    }

    pub fn execute_scheduled_add_admin(
        env: Env,
        caller: Address,
        action_id: u64,
    ) -> Result<(), ContractError> {
        TimelockManager::execute_add_admin(&env, caller, action_id)
    }

    pub fn execute_scheduled_remove_admin(
        env: Env,
        caller: Address,
        action_id: u64,
    ) -> Result<(), ContractError> {
        TimelockManager::execute_remove_admin(&env, caller, action_id)
    }

    pub fn get_scheduled_action(env: Env, action_id: u64) -> Option<TimelockAction> {
        TimelockManager::get_action(&env, action_id)
    }

    pub fn list_scheduled_actions(env: Env, start: u32, limit: u32) -> Vec<TimelockAction> {
        TimelockManager::list_scheduled_actions(&env, start, limit)
    }

    pub fn get_scheduled_action_count(env: Env) -> u64 {
        TimelockManager::get_scheduled_action_count(&env)
    }
}
