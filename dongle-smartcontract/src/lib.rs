#![no_std]
#![allow(warnings)]

mod admin_action_log;
mod admin_manager;
pub mod pagination;
pub mod auth;
mod bookmark_registry;
mod collection_registry;
mod config_registry;
pub mod constants;
mod dependency_registry;
mod dispute_registry;
mod emergency_pause;
mod endorsement_registry;
mod changelog_registry;
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
use crate::config_registry::ConfigRegistry;
use crate::emergency_pause::EmergencyPause;
use crate::errors::ContractError;
use crate::featured_registry::FeaturedRegistry;
use crate::fee_manager::FeeManager;
use crate::changelog_registry::ChangelogRegistry;
use crate::project_registry::ProjectRegistry;
use crate::report_registry::ReportRegistry;
use crate::review_registry::ReviewRegistry;
use crate::storage_keys::ExtensionKey;
use crate::storage_manager::StorageManager;
use crate::timelock_manager::TimelockManager;
use crate::types::{
    AdminActionEntry, AdminProposal, ClaimRequest, ClaimStatus, Collection, ContractClaimRequest,
    ContractConfig, DependencyRef, DisputeResolutionAction, DisputeStatus, DuplicateDispute,
    FeeConfig, FeePaymentRecord, Project, ProjectDependency, ProjectRegistrationParams,
    ProjectReport, ProjectSortMode, ProjectStats, ProjectUpdateParams, ProposalPayload, Review,
    ReviewRevision, ReviewSortMode, ReviewTombstone, SecurityContactStatus, TimelockAction,
    VerificationRecord, VerificationStatus,
};
use crate::verification_registry::VerificationRegistry;
use soroban_sdk::{contract, contractimpl, Address, Env, String, Vec};

#[contract]
pub struct DongleContract;

#[contractimpl]
impl DongleContract {
    // --- Initialization & Admin Management ---

    /// Initializes the contract with `admin` as its first administrator.
    ///
    /// Fails when initialization has already completed; `admin` must authorize the call.
    pub fn initialize(env: Env, admin: Address) -> Result<(), ContractError> {
        AdminManager::initialize(&env, admin)
    }

    /// Adds `new_admin` to the administrator set.
    ///
    /// `caller` must be an existing administrator and both addresses must authorize the change.
    pub fn add_admin(env: Env, caller: Address, new_admin: Address) -> Result<(), ContractError> {
        AdminManager::add_admin(&env, caller, new_admin)
    }

    /// Removes `admin_to_remove` from the administrator set.
    ///
    /// `caller` must be an administrator; the operation fails when it would violate the
    /// configured administration constraints.
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

    /// Sets the duration, in seconds, for approved verifications.
    ///
    /// `caller` must be an administrator and the supplied duration must be valid.
    pub fn set_verification_duration(
        env: Env,
        caller: Address,
        duration_secs: u64,
    ) -> Result<(), ContractError> {
        AdminManager::set_verification_duration(&env, caller, duration_secs)
    }

    pub fn get_verification_duration(env: Env) -> u64 {
        AdminManager::get_verification_duration(&env)
    pub fn get_admin_approval_threshold(env: Env) -> u32 {
        AdminManager::get_admin_approval_threshold(&env)
    }

    pub fn get_config(env: Env) -> ContractConfig {
        ContractConfig {
            fee_config: FeeManager::get_fee_config(&env).ok(),
            treasury: FeeManager::get_treasury(&env).ok(),
            admin_count: AdminManager::get_admin_count(&env),
            paused: false,
            version: String::from_str(&env, "1.0.0"),
            max_projects_per_user: crate::constants::MAX_PROJECTS_PER_USER,
            max_reviews_per_project: crate::constants::MAX_REVIEWS_PER_PROJECT,
            max_reviews_per_user: crate::constants::MAX_REVIEWS_PER_USER,
            max_page_limit: crate::constants::MAX_PAGE_LIMIT,
            max_tags_per_project: crate::constants::MAX_TAGS_PER_PROJECT,
            max_social_links: crate::constants::MAX_SOCIAL_LINKS,
            verification_validity_period: crate::constants::VERIFICATION_VALIDITY_PERIOD,
            fee_payment_expiry_seconds: crate::constants::FEE_PAYMENT_EXPIRY_SECONDS,
            review_update_cooldown_seconds: crate::constants::REVIEW_UPDATE_COOLDOWN_SECONDS,
        }
    }

    /// Sets the number of administrator approvals required for a proposal.
    ///
    /// `caller` must be an administrator; `threshold` must be compatible with the admin set.
    pub fn set_admin_approval_threshold(
        env: Env,
        caller: Address,
        threshold: u32,
    ) -> Result<(), ContractError> {
        AdminManager::set_admin_approval_threshold(&env, caller, threshold)
    }

    /// Creates an administrative proposal containing the requested `payload`.
    ///
    /// `proposer` must be an administrator and must authorize the call.
    pub fn create_proposal(
        env: Env,
        proposer: Address,
        payload: ProposalPayload,
    ) -> Result<u64, ContractError> {
        AdminManager::create_proposal(&env, proposer, payload)
    }

    /// Records an administrator's approval for a pending proposal.
    ///
    /// `admin` must authorize the call and may not approve the same proposal twice.
    pub fn approve_proposal(
        env: Env,
        admin: Address,
        proposal_id: u64,
    ) -> Result<(), ContractError> {
        AdminManager::approve_proposal(&env, admin, proposal_id)
    }

    /// Executes a proposal after it has met the approval threshold.
    ///
    /// `caller` must be an administrator; execution fails for missing, executed, or
    /// insufficiently approved proposals.
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

    /// Registers a project using the supplied metadata and ownership parameters.
    ///
    /// The contract must not be paused. Validation failures, duplicate slugs, and invalid
    /// registration parameters return an error.
    pub fn register_project(
        env: Env,
        params: ProjectRegistrationParams,
    ) -> Result<u64, ContractError> {
        EmergencyPause::require_not_paused(&env)?;
        ProjectRegistry::register_project(&env, params)
    }

    /// Updates a project's mutable metadata.
    ///
    /// The contract must not be paused and the owner or an authorized maintainer encoded in
    /// `params` must authorize the update.
    pub fn update_project(env: Env, params: ProjectUpdateParams) -> Result<Project, ContractError> {
        EmergencyPause::require_not_paused(&env)?;
        ProjectRegistry::update_project(&env, params)
    }

    /// Sets or clears a project's security contact.
    ///
    /// The contract must not be paused; `caller` must be authorized to manage the project.
    pub fn update_security_contact(
        env: Env,
        project_id: u64,
        caller: Address,
        contact: Option<String>,
    ) -> Result<Project, ContractError> {
        EmergencyPause::require_not_paused(&env)?;
        ProjectRegistry::update_security_contact(&env, project_id, caller, contact)
    }

    /// Submits a CID proving control of a project's security contact.
    ///
    /// The contract must not be paused; `caller` must be authorized and `proof_cid` must be valid.
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

    /// Links two projects under the caller's control.
    ///
    /// The contract must not be paused; `caller` must be authorized for `project_id` and both
    /// project IDs must identify valid, linkable projects.
    pub fn link_project(
        env: Env,
        project_id: u64,
        caller: Address,
        linked_project_id: u64,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env)?;
        ProjectRegistry::link_project(&env, project_id, caller, linked_project_id)
    }

    /// Removes an existing link from `project_id` to `linked_project_id`.
    ///
    /// The contract must not be paused and `caller` must be authorized for the source project.
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

    /// Starts transfer of a project to `new_owner`.
    ///
    /// The contract must not be paused and `caller` must be the current owner.
    pub fn initiate_transfer(
        env: Env,
        project_id: u64,
        caller: Address,
        new_owner: Address,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env)?;
        ProjectRegistry::initiate_transfer(&env, project_id, caller, new_owner)
    }

    /// Cancels a pending project ownership transfer.
    ///
    /// The contract must not be paused and `caller` must be authorized to cancel the transfer.
    pub fn cancel_transfer(
        env: Env,
        project_id: u64,
        caller: Address,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env)?;
        ProjectRegistry::cancel_transfer(&env, project_id, caller)
    }

    /// Accepts a pending project transfer.
    ///
    /// `caller` must be the proposed owner and must authorize the acceptance.
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

    /// Sets an optional region tag for a project (owner only).
    /// Delegates to `ProjectRegistry::set_project_region` for the actual logic.
    pub fn set_project_region(
        env: Env,
        project_id: u64,
        caller: Address,
        region: Option<String>,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env)?;
        ProjectRegistry::set_project_region(&env, project_id, caller, region)
    }

    /// Returns the region tag for a project, if set.
    pub fn get_project_region(env: Env, project_id: u64) -> Option<String> {
        env.storage()
            .persistent()
            .get(&ExtensionKey::ProjectRegion(project_id))
    }

    /// Returns the stored integrity hash for a project, if any.
    pub fn get_project_integrity_hash(env: Env, project_id: u64) -> Option<soroban_sdk::Bytes> {
        env.storage()
            .persistent()
            .get(&ExtensionKey::ProjectIntegrityHash(project_id))
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
        start_index: u32,
        limit: u32,
    ) -> Vec<Project> {
        ProjectRegistry::list_projects_by_category(&env, category, start_index, limit)
    }

    pub fn list_projects_sorted(
        env: Env,
        sort_mode: ProjectSortMode,
        start_index: u64,
        limit: u32,
    ) -> Vec<Project> {
        ProjectRegistry::list_projects_sorted(&env, sort_mode, start_index, limit)
    }

    /// Creates a request to associate a contract address with a project.
    ///
    /// `caller` must be authorized for the project and `proof_cid` must demonstrate the claim.
    pub fn claim_contract_address(
        env: Env,
        project_id: u64,
        caller: Address,
        contract_address: String,
        proof_cid: String,
    ) -> Result<ContractClaimRequest, ContractError> {
        ProjectRegistry::claim_contract_address(
            &env,
            project_id,
            caller,
            contract_address,
            proof_cid,
        )
    }

    /// Approves a pending contract-address claim for a project.
    ///
    /// `admin` must be an administrator and must authorize the decision.
    pub fn approve_contract_claim(
        env: Env,
        project_id: u64,
        contract_address: String,
        admin: Address,
    ) -> Result<ContractClaimRequest, ContractError> {
        ProjectRegistry::approve_contract_claim(&env, project_id, contract_address, admin)
    }

    /// Rejects a pending contract-address claim for a project.
    ///
    /// `admin` must be an administrator and must authorize the decision.
    pub fn reject_contract_claim(
        env: Env,
        project_id: u64,
        contract_address: String,
        admin: Address,
    ) -> Result<ContractClaimRequest, ContractError> {
        ProjectRegistry::reject_contract_claim(&env, project_id, contract_address, admin)
    }

    pub fn get_verified_contracts(env: Env, project_id: u64) -> Vec<String> {
        ProjectRegistry::get_verified_contracts(&env, project_id)
    }

    /// Archives a project, making it inactive without deleting its data.
    ///
    /// `caller` must be authorized to manage the project.
    pub fn archive_project(
        env: Env,
        project_id: u64,
        caller: Address,
    ) -> Result<(), ContractError> {
        ProjectRegistry::archive_project(&env, project_id, caller)
    }

    /// Restores an archived project to active status.
    ///
    /// `caller` must be authorized to manage the project.
    pub fn reactivate_project(
        env: Env,
        project_id: u64,
        caller: Address,
    ) -> Result<(), ContractError> {
        ProjectRegistry::reactivate_project(&env, project_id, caller)
    }

    /// Grants `maintainer` permission to manage a project.
    ///
    /// `caller` must be the project owner or otherwise authorized to manage maintainers.
    pub fn add_maintainer(
        env: Env,
        project_id: u64,
        caller: Address,
        maintainer: Address,
    ) -> Result<(), ContractError> {
        ProjectRegistry::add_maintainer(&env, project_id, caller, maintainer)
    }

    /// Revokes a maintainer's project-management permission.
    ///
    /// `caller` must be the project owner or otherwise authorized to manage maintainers.
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

    /// Marks a project as featured or removes that designation.
    ///
    /// `admin` must be an administrator and must authorize the call.
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

    /// Adds a review with an optional CID for its comment content.
    ///
    /// `reviewer` must authorize the call; invalid ratings, duplicate reviews, and disabled
    /// project reviews return an error.
    pub fn add_review(
        env: Env,
        project_id: u64,
        reviewer: Address,
        rating: u32,
        comment_cid: Option<String>,
    ) -> Result<(), ContractError> {
        ReviewRegistry::add_review(&env, project_id, reviewer, rating, comment_cid)
    }

    /// Updates the calling reviewer's existing review.
    ///
    /// `reviewer` must authorize the call and the update must satisfy the review cooldown and
    /// rating validation rules.
    pub fn update_review(
        env: Env,
        project_id: u64,
        reviewer: Address,
        rating: u32,
        comment_cid: Option<String>,
    ) -> Result<(), ContractError> {
        ReviewRegistry::update_review(&env, project_id, reviewer, rating, comment_cid)
    }

    /// Deletes the review authored by `reviewer`.
    ///
    /// `reviewer` must authorize the call; a missing review returns an error.
    pub fn delete_review(
        env: Env,
        project_id: u64,
        reviewer: Address,
    ) -> Result<(), ContractError> {
        ReviewRegistry::delete_review(&env, project_id, reviewer)
    }

    /// Submits a review whose content is referenced by `review_cid`.
    ///
    /// `reviewer` must authorize the call; the CID, rating, and project review settings are
    /// validated before the review is stored.
    pub fn submit_review(
        env: Env,
        project_id: u64,
        reviewer: Address,
        rating: u32,
        review_cid: String,
    ) -> Result<(), ContractError> {
        ReviewRegistry::submit_review(&env, project_id, reviewer, rating, review_cid)
    }

    /// Adds or replaces a project representative's response to a review.
    ///
    /// `caller` must be authorized to manage `project_id`; the target review must exist.
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

    pub fn list_reviews(env: Env, project_id: u64, start_index: u32, limit: u32) -> Vec<Review> {
        ReviewRegistry::list_reviews(&env, project_id, start_index, limit)
    }

    pub fn get_project_stats(env: Env, project_id: u64) -> ProjectStats {
        ReviewRegistry::get_project_stats(&env, project_id)
    }

    /// Bayesian weighted rating (scaled by 100). See `RatingCalculator::calculate_weighted`.
    pub fn get_weighted_rating(env: Env, project_id: u64) -> u32 {
        ReviewRegistry::get_weighted_rating(&env, project_id)
    }

    pub fn get_review_revision_count(env: Env, project_id: u64, reviewer: Address) -> u32 {
        ReviewRegistry::get_review_revision_count(&env, project_id, reviewer)
    }

    pub fn get_review_history(
        env: Env,
        project_id: u64,
        reviewer: Address,
        start_index: u32,
        limit: u32,
    ) -> Vec<ReviewRevision> {
        ReviewRegistry::get_review_history(&env, project_id, reviewer, start_index, limit)
    }

    pub fn get_stats_batch(env: Env, ids: Vec<u64>) -> Vec<(u64, ProjectStats)> {
        ReviewRegistry::get_stats_batch(&env, ids)
    }

    /// Enables or disables new reviews for a project.
    ///
    /// `caller` must be authorized to manage the project.
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

    /// Reports a review for administrator moderation.
    ///
    /// `reporter` must authorize the call and cannot report a missing review.
    pub fn report_review(
        env: Env,
        project_id: u64,
        reviewer: Address,
        reporter: Address,
    ) -> Result<(), ContractError> {
        ReviewRegistry::report_review(&env, project_id, reviewer, reporter)
    }

    /// Hides a review from normal results.
    ///
    /// `admin` must be an administrator and must authorize the moderation action.
    pub fn hide_review(
        env: Env,
        project_id: u64,
        reviewer: Address,
        admin: Address,
    ) -> Result<(), ContractError> {
        ReviewRegistry::hide_review(&env, project_id, reviewer, admin)
    }

    /// Restores a review that was previously hidden.
    ///
    /// `admin` must be an administrator and must authorize the moderation action.
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

    /// Get the deletion tombstone for a review, distinguishing deleted vs never-existed.
    pub fn get_review_tombstone(
        env: Env,
        project_id: u64,
        reviewer: Address,
    ) -> Option<ReviewTombstone> {
        ReviewRegistry::get_review_tombstone(&env, project_id, reviewer)
    }

    /// List reviews sorted by the given sort mode with pagination.
    /// Sorting is performed on-chain in-memory; compute cost scales with review count.
    pub fn list_reviews_sorted(
        env: Env,
        project_id: u64,
        start_index: u32,
        limit: u32,
        sort_mode: ReviewSortMode,
    ) -> Vec<Review> {
        ReviewRegistry::list_reviews_sorted(&env, project_id, start_index, limit, sort_mode)
    }

    // --- Verification Registry ---

    /// Requests verification of a project using evidence stored at `evidence_cid`.
    ///
    /// `requester` must be authorized for the project; the evidence CID and project state are
    /// validated before a pending request is created.
    pub fn request_verification(
        env: Env,
        project_id: u64,
        requester: Address,
        evidence_cid: String,
    ) -> Result<(), ContractError> {
        VerificationRegistry::request_verification(&env, project_id, requester, evidence_cid)
    }

    /// Update the verification evidence CID for a pending verification request.
    ///
    /// # Restrictions
    /// - Only the project owner can update the evidence.
    /// - Updates are allowed only when the request status is `Pending`.
    /// - Once a request is finalized (either Approved/Verified or Rejected), it is immutable
    ///   and further updates will be rejected with an error.
    ///
    /// # Validation
    /// - The new evidence CID is validated using the project's standard IPFS CID rules.
    ///   Malformed or empty CIDs are rejected.
    ///
    /// # Events
    /// - On a successful update, emits a `VerificationEvidenceUpdatedEvent` event.
    pub fn update_verification_evidence(
        env: Env,
        project_id: u64,
        caller: Address,
        new_evidence_cid: String,
    ) -> Result<(), ContractError> {
        VerificationRegistry::update_verification_evidence(
            &env,
            project_id,
            caller,
            new_evidence_cid,
        )
    }

    /// Approves a pending verification request.
    ///
    /// `admin` must be an administrator and must authorize the approval.
    pub fn approve_verification(
        env: Env,
        project_id: u64,
        admin: Address,
    ) -> Result<(), ContractError> {
        VerificationRegistry::approve_verification(&env, project_id, admin)
    }

    /// Rejects a pending verification request.
    ///
    /// `admin` must be an administrator and must authorize the rejection.
    pub fn reject_verification(
        env: Env,
        project_id: u64,
        admin: Address,
    ) -> Result<(), ContractError> {
        VerificationRegistry::reject_verification(&env, project_id, admin)
    }

    /// Revokes a project's active verification and records `reason`.
    ///
    /// `admin` must be an administrator and the reason must satisfy validation rules.
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
    ) -> Option<VerificationRecord> {
        VerificationRegistry::get_verification(&env, project_id)
    }

    pub fn get_verification_record(
        env: Env,
        request_id: u64,
    ) -> Option<VerificationRecord> {
        VerificationRegistry::get_verification_record(&env, request_id)
    }

    pub fn get_verifications_batch(env: Env, ids: Vec<u64>) -> Vec<(u64, VerificationRecord)> {
        VerificationRegistry::get_verifications_batch(&env, ids)
    }

    pub fn is_verification_active(env: Env, project_id: u64) -> bool {
        VerificationRegistry::is_verification_active(&env, project_id)
    }

    /// Renews an eligible verification.
    ///
    /// `admin` must be an administrator; the project must have a renewable verification.
    pub fn renew_verification(
        env: Env,
        project_id: u64,
        admin: Address,
    ) -> Result<(), ContractError> {
        VerificationRegistry::renew_verification(&env, project_id, admin)
    pub fn get_verification_history(env: Env, project_id: u64) -> Vec<VerificationRecord> {
        VerificationRegistry::get_verification_history(&env, project_id)
    }

    /// Requests renewal of a project's verification using updated evidence.
    ///
    /// `requester` must be authorized for the project and `evidence_cid` must be valid.
    pub fn request_renewal(
        env: Env,
        project_id: u64,
        requester: Address,
        evidence_cid: String,
    ) -> Result<(), ContractError> {
        VerificationRegistry::request_renewal(&env, project_id, requester, evidence_cid)
    }

    /// Approves a pending verification-renewal request.
    ///
    /// `admin` must be an administrator and must authorize the decision.
    pub fn approve_renewal(env: Env, project_id: u64, admin: Address) -> Result<(), ContractError> {
        VerificationRegistry::approve_renewal(&env, project_id, admin)
    }

    /// Rejects a pending verification-renewal request.
    ///
    /// `admin` must be an administrator and must authorize the decision.
    pub fn reject_renewal(env: Env, project_id: u64, admin: Address) -> Result<(), ContractError> {
        VerificationRegistry::reject_renewal(&env, project_id, admin)
    }

    pub fn get_renewal_request(
        env: Env,
        project_id: u64,
    ) -> Option<crate::types::VerificationRenewalRecord> {
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

    // --- Verification Assignment ---

    /// Admin: assign a pending verification to a specific admin for review.
    pub fn assign_verification(
        env: Env,
        project_id: u64,
        admin: Address,
        assignee: Address,
    ) -> Result<(), ContractError> {
        VerificationRegistry::assign_verification(&env, project_id, admin, assignee)
    }

    /// Get the admin assigned to review a verification request.
    pub fn get_assigned_admin(env: Env, project_id: u64) -> Option<Address> {
        VerificationRegistry::get_assigned_admin(&env, project_id)
    }

    // --- Reserved Project Names ---

    /// Admin: add a name to the reserved list.
    pub fn add_reserved_name(env: Env, admin: Address, name: String) -> Result<(), ContractError> {
        ProjectRegistry::add_reserved_name(&env, admin, name)
    }

    /// Admin: remove a name from the reserved list.
    pub fn remove_reserved_name(
        env: Env,
        admin: Address,
        name: String,
    ) -> Result<(), ContractError> {
        ProjectRegistry::remove_reserved_name(&env, admin, name)
    }

    /// Get the list of reserved project names.
    pub fn get_reserved_names(env: Env) -> Vec<String> {
        ProjectRegistry::get_reserved_names(&env)
    }

    /// Check if a specific name is reserved.
    pub fn is_name_reserved(env: Env, name: String) -> bool {
        ProjectRegistry::is_name_reserved(&env, &name)
    }

    // --- Fee Manager ---

    /// Configures the token, fee amounts, and treasury used for contract payments.
    ///
    /// `admin` must be an administrator and must authorize the change; fee configuration is
    /// validated before it is stored.
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

    /// Pays the verification fee for a project.
    ///
    /// `payer` must authorize the token transfer; the token must match the configured fee and
    /// the project must be eligible to pay.
    pub fn pay_fee(
        env: Env,
        payer: Address,
        project_id: u64,
        token: Option<Address>,
    ) -> Result<(), ContractError> {
        FeeManager::pay_fee(&env, payer, project_id, token)
    }

    /// Cancels a pending project fee payment.
    ///
    /// An administrator may cancel while paused; otherwise `caller` must satisfy the fee
    /// manager's authorization rules and the contract must not be paused.
    pub fn cancel_fee_payment(
        env: Env,
        caller: Address,
        project_id: u64,
    ) -> Result<(), ContractError> {
        if !AdminManager::is_admin(&env, &caller) {
            EmergencyPause::require_not_paused(&env)?;
        }
        FeeManager::cancel_fee_payment(&env, caller, project_id)
    }

    pub fn is_fee_paid(env: Env, project_id: u64) -> bool {
        FeeManager::is_fee_paid(&env, project_id)
    }

    /// Pays the registration fee for `payer` using the configured token.
    ///
    /// `payer` must authorize the token transfer and the selected token must match configuration.
    pub fn pay_registration_fee(
        env: Env,
        payer: Address,
        token: Option<Address>,
    ) -> Result<(), ContractError> {
        FeeManager::pay_registration_fee(&env, payer, token)
    }

    pub fn get_fee_config(env: Env) -> Result<FeeConfig, ContractError> {
        FeeManager::get_fee_config(&env)
    }

    /// Get fee payment details for a project (payer, amount, token, timestamp).
    pub fn get_fee_payment_details(env: Env, project_id: u64) -> Option<FeePaymentRecord> {
        FeeManager::get_fee_payment_details(&env, project_id)
    }

    /// Get registration fee payment details for an address.
    pub fn get_reg_fee_payment_details(env: Env, address: Address) -> Option<FeePaymentRecord> {
        FeeManager::get_registration_fee_payment_details(&env, &address)
    }

    // --- TTL Management ---

    /// Extend TTL for a specific project and its related data
    pub fn extend_project_ttl(env: Env, project_id: u64) {
        if let Some(project) = ProjectRegistry::get_project(&env, project_id) {
            StorageManager::extend_project_full_ttl(&env, project_id, &project.name);
        }
    }

    /// Extend TTL for many project IDs. Missing projects are skipped.
    pub fn extend_projects_ttl(env: Env, project_ids: Vec<u64>) -> Result<u32, ContractError> {
        if project_ids.len() > crate::constants::MAX_TTL_BATCH_SIZE {
            return Err(ContractError::InvalidInput);
        }

        let mut refreshed = 0u32;
        for i in 0..project_ids.len() {
            if let Some(project_id) = project_ids.get(i) {
                if let Some(project) = ProjectRegistry::get_project(&env, project_id) {
                    StorageManager::extend_project_full_ttl(&env, project_id, &project.name);
                    refreshed = refreshed.saturating_add(1);
                }
            }
        }
        Ok(refreshed)
    }

    /// Extend TTL for a specific review
    pub fn extend_review_ttl(env: Env, project_id: u64, reviewer: Address) {
        StorageManager::extend_review_ttl(&env, project_id, &reviewer);
    }

    /// Extend TTL for many review records. Missing reviews are skipped.
    pub fn extend_reviews_ttl(
        env: Env,
        review_ids: Vec<(u64, Address)>,
    ) -> Result<u32, ContractError> {
        if review_ids.len() > crate::constants::MAX_TTL_BATCH_SIZE {
            return Err(ContractError::InvalidInput);
        }

        let mut refreshed = 0u32;
        for i in 0..review_ids.len() {
            if let Some((project_id, reviewer)) = review_ids.get(i) {
                if ReviewRegistry::get_review(&env, project_id, reviewer.clone()).is_some() {
                    StorageManager::extend_review_ttl(&env, project_id, &reviewer);
                    StorageManager::extend_project_reviews_ttl(&env, project_id);
                    StorageManager::extend_project_stats_ttl(&env, project_id);
                    StorageManager::extend_user_reviews_ttl(&env, &reviewer);
                    refreshed = refreshed.saturating_add(1);
                }
            }
        }
        Ok(refreshed)
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
    pub fn list_projects_by_tag(env: Env, tag: String, start_index: u32, limit: u32) -> Vec<Project> {
        ProjectRegistry::list_projects_by_tag(&env, tag, start_index, limit)
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
    pub fn get_collection(env: Env, collection_id: u64) -> Option<Collection> {
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

    /// Allows or disallows ownership claims for a project.
    ///
    /// `caller` must be authorized to manage the project.
    pub fn set_project_claimable(
        env: Env,
        project_id: u64,
        caller: Address,
        claimable: bool,
    ) -> Result<(), ContractError> {
        ProjectRegistry::set_project_claimable(&env, project_id, caller, claimable)
    }

    /// Submits an ownership claim for a claimable project.
    ///
    /// `claimant` must authorize the call and provide a valid `proof_cid`.
    pub fn submit_claim_request(
        env: Env,
        project_id: u64,
        claimant: Address,
        proof_cid: String,
    ) -> Result<u64, ContractError> {
        ProjectRegistry::submit_claim_request(&env, project_id, claimant, proof_cid)
    }

    /// Approves a pending project ownership claim.
    ///
    /// `admin` must be an administrator and must authorize the decision.
    pub fn approve_claim_request(
        env: Env,
        claim_request_id: u64,
        admin: Address,
    ) -> Result<(), ContractError> {
        ProjectRegistry::approve_claim_request(&env, claim_request_id, admin)
    }

    /// Rejects a pending project ownership claim.
    ///
    /// `admin` must be an administrator and must authorize the decision.
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

    /// Adds a dependency record to a project.
    ///
    /// `caller` must be authorized to manage the project and the dependency must be valid.
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

    /// Replaces the dependency identified by `dependency_key`.
    ///
    /// `caller` must be authorized to manage the project; a missing dependency returns an error.
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

    /// Removes a dependency from a project.
    ///
    /// `caller` must be authorized to manage the project; a missing dependency returns an error.
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

    /// Opens a duplicate-project dispute with supporting evidence.
    ///
    /// `creator` must authorize the request and `evidence_cid` must be valid.
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

    /// Resolves a duplicate-project dispute according to `action`.
    ///
    /// `admin` must be an administrator and must authorize the resolution.
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

    // --- Project Changelog ---

    /// Add a new changelog entry for a project (owner only).
    ///
    /// # Arguments
    /// - `project_id`: The project ID to add changelog for
    /// - `owner`: The project owner (must be authenticated)
    /// - `cid`: IPFS CID containing the changelog content
    /// - `description`: Optional description/title for the changelog entry
    ///
    /// # Returns
    /// - `Ok(u64)` with the new changelog entry ID on success
    /// - `Err(ContractError)` on failure
    pub fn add_changelog_entry(
        env: Env,
        project_id: u64,
        owner: Address,
        cid: String,
        description: Option<String>,
    ) -> Result<u64, ContractError> {
        EmergencyPause::require_not_paused(&env)?;
        ChangelogRegistry::add_changelog_entry(&env, project_id, owner, cid, description)
    }

    /// Remove a changelog entry (project owner only).
    ///
    /// # Arguments
    /// - `changelog_id`: The changelog entry ID to remove
    /// - `owner`: The project owner (must be authenticated)
    ///
    /// # Returns
    /// - `Ok(())` on success
    /// - `Err(ContractError)` on failure
    pub fn remove_changelog_entry(
        env: Env,
        changelog_id: u64,
        owner: Address,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env)?;
        ChangelogRegistry::remove_changelog_entry(&env, changelog_id, owner)
    }

    /// Get a single changelog entry by ID.
    ///
    /// # Arguments
    /// - `changelog_id`: The changelog entry ID
    ///
    /// # Returns
    /// - `Option<ChangelogEntry>` the changelog entry if found
    pub fn get_changelog_entry(env: Env, changelog_id: u64) -> Option<ChangelogEntry> {
        ChangelogRegistry::get_changelog_entry(&env, changelog_id)
    }

    /// Get paginated changelog entries for a project.
    ///
    /// # Arguments
    /// - `project_id`: The project ID to get changelog for
    /// - `start`: Starting index for pagination
    /// - `limit`: Maximum number of entries to return (capped at MAX_PAGE_LIMIT)
    /// - `sort_mode`: Sort order (Newest or Oldest)
    ///
    /// # Returns
    /// - `Vec<ChangelogEntry>` paginated and sorted changelog entries
    pub fn get_project_changelog(
        env: Env,
        project_id: u64,
        start: u32,
        limit: u32,
        sort_mode: ChangelogSortMode,
    ) -> Vec<ChangelogEntry> {
        ChangelogRegistry::get_project_changelog(&env, project_id, start, limit, sort_mode)
    }

    /// Get changelog entry count for a project.
    ///
    /// # Arguments
    /// - `project_id`: The project ID
    ///
    /// # Returns
    /// - `u32` number of changelog entries
    pub fn get_changelog_count(env: Env, project_id: u64) -> u32 {
        ChangelogRegistry::get_changelog_count(&env, project_id)
    }

    // --- Subscription / Follow ---

    /// Follows a project on behalf of `follower`.
    ///
    /// `follower` must authorize the call; duplicate follows return an error.
    pub fn follow_project(
        env: Env,
        project_id: u64,
        follower: Address,
    ) -> Result<(), ContractError> {
        crate::subscription_registry::SubscriptionRegistry::follow_project(
            &env, project_id, follower,
        )
    }

    /// Removes `follower`'s subscription to a project.
    ///
    /// `follower` must authorize the call; following must already exist.
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

    /// Saves a project in `user`'s bookmarks.
    ///
    /// `user` must authorize the call; duplicate bookmarks return an error.
    pub fn bookmark_project(
        env: Env,
        project_id: u64,
        user: Address,
    ) -> Result<(), ContractError> {
        crate::bookmark_registry::BookmarkRegistry::bookmark_project(&env, project_id, user)
    }

    /// Removes a project from `user`'s bookmarks.
    ///
    /// `user` must authorize the call; the bookmark must already exist.
    pub fn unbookmark_project(
        env: Env,
        project_id: u64,
        user: Address,
    ) -> Result<(), ContractError> {
        crate::bookmark_registry::BookmarkRegistry::unbookmark_project(&env, project_id, user)
    }

    pub fn is_bookmarked(env: Env, project_id: u64, user: Address) -> bool {
        crate::bookmark_registry::BookmarkRegistry::is_bookmarked(&env, project_id, &user)
    }

    pub fn get_user_bookmarks(env: Env, user: Address, start: u32, limit: u32) -> Vec<u64> {
        crate::bookmark_registry::BookmarkRegistry::get_user_bookmarks(&env, user, start, limit)
    }

    // --- Endorsement Registry ---

    /// Records `user`'s endorsement of a project.
    ///
    /// `user` must authorize the call; duplicate endorsements return an error.
    pub fn endorse_project(
        env: Env,
        project_id: u64,
        user: Address,
    ) -> Result<(), ContractError> {
        crate::endorsement_registry::EndorsementRegistry::endorse_project(&env, project_id, user)
    }

    /// Removes `user`'s endorsement of a project.
    ///
    /// `user` must authorize the call; the endorsement must already exist.
    pub fn unendorse_project(
        env: Env,
        project_id: u64,
        user: Address,
    ) -> Result<(), ContractError> {
        crate::endorsement_registry::EndorsementRegistry::unendorse_project(&env, project_id, user)
    }

    pub fn get_endorsement_count(env: Env, project_id: u64) -> u32 {
        crate::endorsement_registry::EndorsementRegistry::get_endorsement_count(&env, project_id)
    }

    pub fn has_endorsed(env: Env, project_id: u64, user: Address) -> bool {
        crate::endorsement_registry::EndorsementRegistry::has_endorsed(&env, project_id, &user)
    }

    // --- Admin Timelock ---

    /// Schedules a fee configuration change for execution at `execution_timestamp`.
    ///
    /// `admin` must be an administrator and the timestamp must satisfy timelock rules.
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

    /// Schedules addition of an administrator after the configured timelock.
    ///
    /// `admin` must be an administrator and the timestamp must satisfy timelock rules.
    pub fn schedule_add_admin(
        env: Env,
        admin: Address,
        new_admin: Address,
        execution_timestamp: u64,
    ) -> Result<u64, ContractError> {
        TimelockManager::schedule_add_admin(&env, admin, new_admin, execution_timestamp)
    }

    /// Schedules removal of an administrator after the configured timelock.
    ///
    /// `admin` must be an administrator and the timestamp must satisfy timelock rules.
    pub fn schedule_remove_admin(
        env: Env,
        admin: Address,
        admin_to_remove: Address,
        execution_timestamp: u64,
    ) -> Result<u64, ContractError> {
        TimelockManager::schedule_remove_admin(&env, admin, admin_to_remove, execution_timestamp)
    }

    /// Cancels a pending timelock action.
    ///
    /// `caller` must be authorized by the timelock manager; executed actions cannot be cancelled.
    pub fn cancel_scheduled_action(
        env: Env,
        caller: Address,
        action_id: u64,
    ) -> Result<(), ContractError> {
        TimelockManager::cancel_action(&env, caller, action_id)
    }

    /// Executes a due, scheduled fee configuration action.
    ///
    /// `caller` must be authorized and the action must be ready for execution.
    pub fn execute_scheduled_set_fee(
        env: Env,
        caller: Address,
        action_id: u64,
    ) -> Result<(), ContractError> {
        TimelockManager::execute_set_fee(&env, caller, action_id)
    }

    /// Executes a due, scheduled administrator-addition action.
    ///
    /// `caller` must be authorized and the action must be ready for execution.
    pub fn execute_scheduled_add_admin(
        env: Env,
        caller: Address,
        action_id: u64,
    ) -> Result<(), ContractError> {
        TimelockManager::execute_add_admin(&env, caller, action_id)
    }

    /// Executes a due, scheduled administrator-removal action.
    ///
    /// `caller` must be authorized and the action must be ready for execution.
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

    // --- Contract Configuration View ---

    /// Returns the aggregated `ContractConfigView` snapshot (fees, treasury,
    /// admin count, pause state, limits, and version) in a single read.
    ///
    /// Returns `ContractError::FeeConfigNotSet` until `set_fee` has been
    /// called at least once. Frontends can use the presence of a fee
    /// config as a readiness signal for production traffic.
    pub fn get_config(env: Env) -> Result<ContractConfigView, ContractError> {
        ConfigRegistry::get_config(&env)
    }

    /// Admin: toggle the global pause flag surfaced by `get_config`.
    ///
    /// **Returns** the pause state *before* the call (so callers can
    /// detect transitions without an extra `get_config` round-trip).
    /// Records an `AdminActionLog` entry (`ContractPaused` or
    /// `ContractResumed`) for audit parity with every other admin
    /// mutation in this contract.
    ///
    /// **Scope:** this method only writes the flag. Enforcement across
    /// mutating entry points (`register_project`, `pay_fee`, …) is
    /// intentionally out of scope for the config-view feature — see the
    /// future pause-enforcement ticket.
    pub fn set_pause(env: Env, admin: Address, paused: bool) -> Result<bool, ContractError> {
        ConfigRegistry::set_pause(&env, admin, paused)
    }
}
