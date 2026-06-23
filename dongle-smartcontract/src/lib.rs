#![no_std]

mod admin_manager;
pub mod auth;
pub mod constants;
pub mod errors;
pub mod events;
mod fee_manager;
mod featured_registry;
mod project_registry;
pub mod rating_calculator;
mod report_registry;
pub mod review_registry;
pub mod storage_keys;
pub mod storage_manager;
pub mod types;
pub mod utils;
mod verification_registry;

#[cfg(test)]
mod tests;

use crate::admin_manager::AdminManager;
use crate::errors::ContractError;
use crate::fee_manager::FeeManager;
use crate::featured_registry::FeaturedRegistry;
use crate::project_registry::ProjectRegistry;
use crate::report_registry::ReportRegistry;
use crate::review_registry::ReviewRegistry;
use crate::storage_manager::StorageManager;
use crate::types::{
    FeeConfig, Project, ProjectRegistrationParams, ProjectReport, ProjectStats,
    ProjectUpdateParams, Review, VerificationRecord, VerificationStatus,
    ClaimRequest, ClaimStatus,
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

    /// Mark a project as claimable or not claimable
    pub fn set_project_claimable(
        env: Env,
        project_id: u64,
        caller: Address,
        claimable: bool,
    ) -> Result<(), ContractError> {
        ProjectRegistry::set_project_claimable(&env, project_id, caller, claimable)
    }

    /// Submit a claim request for a project
    pub fn submit_claim_request(
        env: Env,
        project_id: u64,
        claimant: Address,
        proof_cid: String,
    ) -> Result<u64, ContractError> {
        ProjectRegistry::submit_claim_request(&env, project_id, claimant, proof_cid)
    }

    /// Approve a claim request
    pub fn approve_claim_request(
        env: Env,
        claim_request_id: u64,
        admin: Address,
    ) -> Result<(), ContractError> {
        ProjectRegistry::approve_claim_request(&env, claim_request_id, admin)
    }

    /// Reject a claim request
    pub fn reject_claim_request(
        env: Env,
        claim_request_id: u64,
        admin: Address,
    ) -> Result<(), ContractError> {
        ProjectRegistry::reject_claim_request(&env, claim_request_id, admin)
    }

    /// Get a claim request by id
    pub fn get_claim_request(env: Env, claim_request_id: u64) -> Option<ClaimRequest> {
        ProjectRegistry::get_claim_request(&env, claim_request_id)
    }

    /// Get claim requests for a project
    pub fn get_claim_requests_for_project(env: Env, project_id: u64) -> Vec<ClaimRequest> {
        ProjectRegistry::get_claim_requests_for_project(&env, project_id)
    }
}
