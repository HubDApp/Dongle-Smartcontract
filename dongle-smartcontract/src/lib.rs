#![no_std]

extern crate alloc;
#[cfg(test)]
extern crate std;

mod admin_action_log;
mod admin_manager;
mod analytics_export;
mod auto_archive_registry;
pub mod auto_archive_types;
pub mod auth;
mod blacklist_registry;
mod bookmark_registry;
mod campaign_registry;
pub mod category_types;
mod category_registry;
mod changelog_registry;
mod collection_registry;
mod computed_view_cache;
mod rate_limiter;
// mod community_collection_registry;
mod config_registry;
pub mod constants;
mod dependency_registry;
mod deprecation_registry;
mod dispute_registry;
mod emergency_pause;
mod endorsement_delegation;
mod endorsement_graph;
mod endorsement_registry;
pub mod errors;
pub mod events;
mod featured_registry;
mod featured_rotation_registry;
mod fee_manager;
mod fee_extensions;
mod governance_ranges;
mod notification_registry;
mod mutual_recommendation;
mod grant_registry;
pub mod pagination;
mod probation_registry;
mod project_registry;
pub mod project_controls;
mod project_operation_limiter;
pub mod rating_calculator;
mod recommendation_registry;
mod report_registry;
mod reward_registry;
pub mod review_registry;
mod social_analytics_registry;
pub mod storage_keys;
pub mod trust_and_safety;
pub mod storage_manager;
mod state_export;
mod subscription_bulk;
mod subscription_registry;
mod timelock_manager;
pub mod types;
pub mod utils;
mod validation;
mod verification_registry;
mod social_analytics_registry;
mod probation_registry;
mod performance_metrics;
mod invariant_checker;
mod pow_challenge;
mod compliance_mode;
mod benchmark_suite;
mod security_contact_verification;
mod health_score_registry;
mod activity_feed_registry;
mod bounty_registry;
mod metadata_enrichment_registry;
mod similarity_registry;
mod smart_bookmark_registry;
mod trending_registry;

#[cfg(test)]
mod tests;

use crate::admin_action_log::AdminActionLog;
use crate::admin_manager::AdminManager;
use crate::blacklist_registry::BlacklistRegistry;
use crate::changelog_registry::ChangelogRegistry;
use crate::collection_registry::CollectionRegistry;
use crate::config_registry::ConfigRegistry;
use crate::emergency_pause::EmergencyPause;
use crate::errors::ContractError;
use crate::featured_registry::FeaturedRegistry;
use crate::fee_manager::FeeManager;
use crate::fee_extensions::FeeExtensions;
use crate::probation_registry::ProbationRegistry;
use crate::project_registry::ProjectRegistry;
use crate::report_registry::ReportRegistry;
use crate::review_registry::ReviewRegistry;
use crate::similarity_registry::SimilarityRegistry;
use crate::smart_bookmark_registry::SmartBookmarkRegistry;
use crate::storage_manager::StorageManager;
use crate::timelock_manager::TimelockManager;
use crate::trending_registry::TrendingRegistry;
use crate::activity_feed_registry::ActivityFeedRegistry;
use crate::health_score_registry::HealthScoreRegistry;
use crate::metadata_enrichment_registry::MetadataEnrichmentRegistry;
use crate::notification_registry::NotificationRegistry;
use crate::security_contact_verification::SecurityContactVerificationRegistry;
use crate::types::{
    // #924 dependency graph
    DependencyGraph,
    // #925 dependency deprecation
    DeprecationRecord,
    // #926 feature campaigns
    CampaignInteraction, CampaignStats, FeatureCampaign, FeatureCampaignConfig,
    // #927 featured rotation
    FeaturedRotationSchedule,
    AdminActionEntry, AdminActivityRecord, AdminProposal, AdminVerificationPerformance,
    ArchivedReview, BatchTtlResult,
    SimilarProject, SmartBookmarkCollection, SmartBookmarkRules, TrendingProject,

    AdminActionEntry, AdminProposal, AdminWorkload, ArchivedReview, BatchTtlResult, BookmarkFolder,
    ChangelogEntry, ChangelogSortMode, ClaimRequest, Collection, CommunityColInclusionStatus,
    CommunityColRevenueSnapshot, CommunityCollection, CommunityCollectionTemplateId,
    CommunityCollectionVote, ContractClaimRequest, ContractConfigView, DependencyRef,
    DisputeResolutionAction, DuplicateDispute, EmergencyRecoveryRequest, EvidenceLink, FeeConfig, FeeConfigHistoryEntry, FeePaymentRecord,
    FeeRefundRecord, FeeWaiverRecord, FeeWaiverType, AutoRefundRecord, FeeReceipt, EscrowedFee, FeeDisputeResolution, RefundTrigger, WaiverStatus,
    NotificationDeliveryStatus, ProbationRecord, Project, ProjectDependency,
    ProjectEngagementMetric, ProjectLifecycleStatus, ProjectRegistrationParams, ProjectReport,
    ProjectSocialAnalyticsExport, ProjectSocialDailyCheckpoint, ProjectSocialPeerRow,
    ProjectSortMode, ProjectStats, ProjectSunsetPlan, ProjectUpdateParams, ProposalComment, ProposalPayload,
    Recommendation, RecommendationAlgorithm, RecommendationAnalytics, RecommendationEngagementKind,
    RecommendationFeedback, Review, ReviewRevision, ReviewSortMode, ReviewTombstone,
    SecurityContactStatus, SmartFolder, SmartFolderFilter, TimelockAction, VerificationAssignment,
    VerificationAssignmentStatus, VerificationBatchAction, VerificationBatchReport, VerificationExpiryNotification,
    VerificationRecord, VerificationRiskAssessment, VerificationRiskModel, VerificationStatus,
    VerificationSuspension,
    AdminActionEntry, AdminActivityRecord, AdminProposal, ArchivedReview, BatchTtlResult,
    BookmarkFolder, ChangelogEntry, ChangelogSortMode, ClaimRequest, Collection,
    ContractClaimRequest, ContractConfigView, DependencyRef, DisputeResolutionAction,
    DuplicateDispute, EmergencyRecoveryRequest, EvidenceLink, FeeConfig, FeeConfigHistoryEntry,
    FeePaymentRecord, FeeRefundRecord, Project, ProjectDependency, ProjectLifecycleStatus,
    ProjectRegistrationParams, ProjectReport, ProjectSortMode, ProjectStats, ProjectSunsetPlan,
    ProjectUpdateParams, ProposalComment, ProposalPayload, Review, ReviewRevision, ReviewSortMode,
    ReviewTombstone, SecurityContactStatus, SmartFolder, SmartFolderFilter, TimelockAction,
    VerificationBatchAction, VerificationBatchReport, VerificationPerformanceReport,
    VerificationPerformanceSnapshot, VerificationRecord, VerificationStatus,
    VerificationStatusFilter, NotificationDeliveryStatus,
    VerificationExpiryNotification, VerificationRiskAssessment,
    VerificationRiskModel, VerificationStatusFilter, VerificationSuspension,
    VerificationTrendPoint, AdminWorkload, VerificationAssignment, VerificationAssignmentStatus,
    VerificationBatchAction, VerificationBatchReport, VerificationRecord, VerificationStatus,
    VerificationStatusFilter, NotificationDeliveryStatus, TimelockAction,
    VerificationExpiryNotification, VerificationRecord, VerificationRiskAssessment,
    VerificationRiskModel, VerificationStatus, VerificationStatusFilter, VerificationSuspension,
    AdminWorkload, VerificationAssignment, VerificationAssignmentStatus,
    // #757 security contact verification
    SecurityContactVerificationRecord, SecurityContactVerificationStatus,
    // #756 health score
    HealthScoreBreakdown, HealthScoreConfig, HealthScoreSnapshot, ProjectHealthScore,
    // #759 activity feed
    ActivityEntry, ActivityKind,
    // #760 metadata enrichment
    EnrichmentSuggestion, EnrichmentSuggestionStatus, MetadataEnrichmentFields,
    // Custom verification requirements
    VerificationRequirementAudit, ProjectVerificationRequirements,
    // #915 bookmark auto-archive
    ArchivedBookmarkRecord, BookmarkConfig, BookmarkExpirationPolicy,
    // #911 feature recommendations
    RecommendationABAnalytics, RecommendationABConfig, RecommendationOptions,
    RecommendationStrategy, RecommendationVariant,
    // #910 complex query filters
    FilterLogic, ProjectFilterCriteria, ProjectFilterSortBy, ProjectQueryFilter, ProjectQueryResult,
    // #908 collection immutable snapshots
    CollectionSnapshot, CollectionSnapshotComparison,
};
use crate::verification_registry::{VerificationAssignmentRegistry, VerificationRegistry};
use soroban_sdk::{contract, contractimpl, Address, Env, String, Vec};

#[contract]
pub struct DongleContract;

#[contractimpl]
impl DongleContract {
    // --- Initialization & Admin Management ---

    pub fn initialize(env: Env, admin: Address) -> Result<(), ContractError> {
        AdminManager::initialize(&env, admin)
    }

    pub fn add_admin(env: Env, caller: Address, new_admin: Address) -> Result<(), ContractError> {
        AdminManager::add_admin(&env, caller, new_admin)
    }

    /// Add a new admin with an assigned role (closes role-based permission criteria).
    pub fn add_admin_with_role(
        env: Env,
        caller: Address,
        new_admin: Address,
        role: AdminRole,
    ) -> Result<(), ContractError> {
        AdminManager::add_admin_with_role(&env, caller, new_admin, role)
    }

    /// Get the role assigned to an admin address.
    pub fn get_admin_role(env: Env, admin: Address) -> Option<AdminRole> {
        AdminManager::get_admin_role(&env, &admin)
    }

    /// Check if an admin has a specific granular permission.
    pub fn has_admin_permission(env: Env, admin: Address, permission: AdminPermission) -> bool {
        AdminManager::has_admin_permission(&env, &admin, permission)
    }

    /// Migrate existing admins without stored roles to SuperAdmin for backward compatibility.
    pub fn migrate_existing_admins(env: Env, caller: Address) -> Result<u32, ContractError> {
        AdminManager::migrate_existing_admins(&env, caller)
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

    pub fn set_verification_duration(
        env: Env,
        caller: Address,
        duration_secs: u64,
    ) -> Result<(), ContractError> {
        AdminManager::set_verification_duration(&env, caller, duration_secs)
    }

    pub fn get_verification_duration(env: Env) -> u64 {
        AdminManager::get_verification_duration(&env)
    }

    pub fn get_admin_approval_threshold(env: Env) -> u32 {
        AdminManager::get_admin_approval_threshold(&env)
    }

    pub fn get_config(env: Env) -> Result<ContractConfigView, ContractError> {
        ConfigRegistry::get_config(&env)
    }

    /// Returns the current maximum number of reviews allowed per project.
    ///
    /// Falls back to the compile-time default of 500 if no value has been
    /// configured by an admin.
    pub fn get_max_reviews_per_project(env: Env) -> u32 {
        ConfigRegistry::get_max_reviews_per_project(&env)
    }

    /// Admin-only: set the maximum number of reviews allowed per project.
    ///
    /// `max` must be ≥ 1. Affects all future review submissions; existing
    /// reviews beyond a lowered limit are not removed.
    pub fn set_max_reviews_per_project(
        env: Env,
        admin: Address,
        max: u32,
    ) -> Result<(), ContractError> {
        ConfigRegistry::set_max_reviews_per_project(&env, admin, max)
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
        expires_at: u64,
    ) -> Result<u64, ContractError> {
        AdminManager::create_proposal(&env, proposer, payload, expires_at)
    }

    pub fn approve_proposal(
        env: Env,
        admin: Address,
        proposal_id: u64,
    ) -> Result<(), ContractError> {
        AdminManager::approve_proposal(&env, admin, proposal_id)
    }

    pub fn reject_proposal(
        env: Env,
        admin: Address,
        proposal_id: u64,
    ) -> Result<(), ContractError> {
        AdminManager::reject_proposal(&env, admin, proposal_id)
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

    /// List admin proposals with pagination.
    ///
    /// `start_index` is a zero-based offset and `limit` caps the page size.
    pub fn list_proposals(env: Env, start_index: u32, limit: u32) -> Vec<AdminProposal> {
        AdminManager::list_proposals(&env, start_index, limit)
    }

    /// Batch-remove expired admin proposals to prevent storage bloat (#728).
    ///
    /// Scans at most `batch_size` proposals (capped at 100). Expired proposals
    /// are those whose `expires_at` is non-zero and has passed. Returns the
    /// number of proposals removed.
    pub fn cleanup_expired_proposals(
        env: Env,
        caller: Address,
        batch_size: u32,
    ) -> Result<u32, ContractError> {
        AdminManager::cleanup_expired_proposals(&env, caller, batch_size)
    }

    // --- #736: Proposal comment/discussion system ---

    /// Add a comment to a proposal. Comments are immutable once voting starts.
    pub fn add_proposal_comment(
        env: Env,
        caller: Address,
        proposal_id: u64,
        content: String,
    ) -> Result<u64, ContractError> {
        AdminManager::add_proposal_comment(&env, caller, proposal_id, content)
    }

    /// Get comments for a proposal with pagination.
    pub fn get_proposal_comments(
        env: Env,
        proposal_id: u64,
        start_index: u32,
        limit: u32,
    ) -> Vec<crate::types::ProposalComment> {
        AdminManager::get_proposal_comments(&env, proposal_id, start_index, limit)
    }

    // --- #738: Emergency admin recovery ---

    /// Initiate an emergency admin recovery request. Requires 2/3 of remaining
    /// admins to approve, with a 7-day voting period.
    pub fn initiate_emergency_recovery(
        env: Env,
        caller: Address,
        lost_admin: Address,
        new_admin: Address,
    ) -> Result<u64, ContractError> {
        AdminManager::initiate_emergency_recovery(&env, caller, lost_admin, new_admin)
    }

    /// Approve an emergency recovery request.
    pub fn approve_emergency_recovery(
        env: Env,
        admin: Address,
        request_id: u64,
    ) -> Result<(), ContractError> {
        AdminManager::approve_emergency_recovery(&env, admin, request_id)
    }

    /// Get an emergency recovery request by ID.
    pub fn get_emergency_recovery(
        env: Env,
        request_id: u64,
    ) -> Option<crate::types::EmergencyRecoveryRequest> {
        AdminManager::get_emergency_recovery(&env, request_id)
    }

    // --- #739: Inactive admin tracking ---

    /// Get the admin activity record for an address.
    pub fn get_admin_activity(
        env: Env,
        admin: Address,
    ) -> Option<crate::types::AdminActivityRecord> {
        AdminManager::get_admin_activity(&env, &admin)
    }

    /// Check if an admin has been inactive for more than the specified days.
    pub fn is_admin_inactive(env: Env, admin: Address, days: u64) -> bool {
        AdminManager::is_admin_inactive(&env, &admin, days)
    }

    /// Admin-only: set the monthly veto limit per admin (#730).
    /// 0 = unlimited (default).
    pub fn set_veto_monthly_limit(
        env: Env,
        caller: Address,
        limit: u32,
    ) -> Result<(), ContractError> {
        AdminManager::set_veto_monthly_limit(&env, caller, limit)
    }

    /// Return the current monthly veto limit (0 = unlimited).
    pub fn get_veto_monthly_limit(env: Env) -> u32 {
        AdminManager::get_veto_monthly_limit(&env)
    }

    /// Return how many vetoes `admin` has cast in the current calendar month.
    pub fn get_veto_count(env: Env, admin: Address) -> u32 {
        AdminManager::get_veto_count(&env, &admin)
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

    // --- Fine-grained pause control (#902) ---

    /// Pause the contract with a reason and an estimated resume time
    /// (admin-only, issue #901).
    ///
    /// `reason` is a short operator note (1..=256 bytes) explaining why the
    /// contract was halted.  `estimated_resume` is the expected resume time
    /// as a Unix timestamp (`0` means "no estimate").
    pub fn set_pause_with_reason(
        env: Env,
        admin: Address,
        reason: String,
        estimated_resume: u64,
    ) -> Result<(), ContractError> {
        EmergencyPause::set_pause_with_reason(&env, &admin, reason, estimated_resume)
    }

    /// Pause only a specific family of operations (admin-only, issue #902).
    ///
    /// `state` selects the blocked operation family
    /// (`PausedVerification`, `PausedFees`, `PausedReviews`, `PausedAll`);
    /// passing `Running` resumes the contract.
    pub fn set_pause_state(
        env: Env,
        admin: Address,
        state: PauseState,
        reason: Option<String>,
        estimated_resume: Option<u64>,
    ) -> Result<(), ContractError> {
        EmergencyPause::set_pause_state(&env, &admin, state, reason, estimated_resume)
    }

    /// Current fine-grained pause state (issue #902).
    pub fn get_pause_state(env: Env) -> PauseState {
        EmergencyPause::get_pause_state(&env)
    }

    /// Operational status of the contract including the pause reason and the
    /// estimated resume time (issue #901).
    pub fn get_contract_status(env: Env) -> ContractStatus {
        EmergencyPause::get_contract_status(&env)
    }

    // --- Staged recovery after an emergency pause (#904) ---

    /// Resume the contract up to the given staged-recovery phase
    /// (admin-only, issue #904).
    ///
    /// Phases: `0` = nothing resumed, `1` = read operations, `2` = reviews,
    /// `3` = verification, `4` = fees (fully running again).
    pub fn resume_operations_phase(
        env: Env,
        admin: Address,
        phase: u32,
    ) -> Result<(), ContractError> {
        EmergencyPause::resume_operations_phase(&env, &admin, phase)
    }

    /// Roll the staged recovery back by one phase (admin-only, issue #904).
    pub fn rollback_operations_phase(env: Env, admin: Address) -> Result<(), ContractError> {
        EmergencyPause::rollback_operations_phase(&env, &admin)
    }

    /// Current staged-recovery phase (issue #904).
    pub fn get_resume_phase(env: Env) -> u32 {
        EmergencyPause::get_resume_phase(&env)
    }

    /// Ordered history of staged-recovery phase transitions (issue #904).
    pub fn get_phase_history(env: Env) -> Vec<u32> {
        EmergencyPause::get_phase_history(&env)
    }

    // --- Pause notifications (#901) ---

    /// Subscribe an address to emergency-pause notifications (issue #901).
    pub fn subscribe_pause_notifications(env: Env, subscriber: Address) -> Result<(), ContractError> {
        subscriber.require_auth();
        NotificationRegistry::subscribe_pause_notifications(&env, subscriber)
    }

    /// Unsubscribe an address from emergency-pause notifications (issue #901).
    pub fn unsubscribe_pause_notifications(
        env: Env,
        subscriber: Address,
    ) -> Result<(), ContractError> {
        subscriber.require_auth();
        NotificationRegistry::unsubscribe_pause_notifications(&env, subscriber)
    }

    /// Addresses currently subscribed to emergency-pause notifications (#901).
    pub fn list_pause_subscribers(env: Env) -> Vec<Address> {
        NotificationRegistry::list_pause_subscribers(&env)
    }

    // --- Project Registry ---

    pub fn register_project(
        env: Env,
        params: ProjectRegistrationParams,
    ) -> Result<u64, ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        ProjectRegistry::register_project(&env, params)
    }

    pub fn update_project(env: Env, params: ProjectUpdateParams) -> Result<Project, ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        ProjectRegistry::update_project(&env, params)
    }

    pub fn export_project(
        env: Env,
        project_id: u64,
        caller: Address,
    ) -> Result<crate::types::ProjectExport, ContractError> {
        caller.require_auth();
        let project = ProjectRegistry::get_project(&env, project_id).ok_or(ContractError::ProjectNotFound)?;
        
        if project.owner != caller {
            return Err(ContractError::Unauthorized);
        }

        let reviews = crate::review_registry::ReviewRegistry::list_reviews(&env, project_id, 0, 100);
        let dependencies = crate::dependency_registry::DependencyRegistry::get_dependencies(&env, project_id);
        let verification_records = crate::verification_registry::VerificationRegistry::get_verification_history(&env, project_id);

        Ok(crate::types::ProjectExport {
            version: 1,
            project,
            reviews,
            dependencies,
            verification_records,
            encrypted_payload: None,
            owner_signature: None,
        })
    }

    pub fn import_project(
        env: Env,
        export: crate::types::ProjectExport,
        caller: Address,
    ) -> Result<u64, ContractError> {
        caller.require_auth();
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        
        let params = crate::types::ProjectRegistrationParams {
            owner: caller.clone(),
            name: export.project.name.clone(),
            slug: export.project.slug.clone(),
            description: export.project.description.clone(),
            category: export.project.category.clone(),
            website: export.project.website.clone(),
            license: export.project.license.clone(),
            logo_cid: export.project.logo_cid.clone(),
            metadata_cid: export.project.metadata_cid.clone(),
            tags: export.project.tags.clone(),
            social_links: export.project.social_links.clone(),
            launch_timestamp: export.project.launch_timestamp,
            bounty_url: export.project.bounty_url.clone(),
            repository_url: export.project.repository_url.clone(),
        };
        
        let new_project_id = ProjectRegistry::register_project(&env, params)?;
        
        for i in 0..export.dependencies.len() {
            if let Some(dep) = export.dependencies.get(i) {
                let _ = crate::dependency_registry::DependencyRegistry::add_dependency(&env, new_project_id, caller.clone(), dep);
            }
        }
        
        let mut review_keys: Vec<Address> = env
            .storage()
            .persistent()
            .get(&crate::storage_keys::StorageKey::ProjectReviews(new_project_id))
            .unwrap_or_else(|| Vec::new(&env));
            
        for i in 0..export.reviews.len() {
            if let Some(mut rev) = export.reviews.get(i) {
                rev.project_id = new_project_id;
                env.storage().persistent().set(&crate::storage_keys::StorageKey::Review(new_project_id, rev.reviewer.clone()), &rev);
                if !review_keys.contains(&rev.reviewer) {
                    review_keys.push_back(rev.reviewer.clone());
                }
            }
        }
        env.storage().persistent().set(&crate::storage_keys::StorageKey::ProjectReviews(new_project_id), &review_keys);
        
        // Let's also restore verification records just in case, though they usually require admin
        for i in 0..export.verification_records.len() {
            if let Some(mut vr) = export.verification_records.get(i) {
                vr.project_id = new_project_id;
                // Generate a new request id to not overlap with global request ids
                let new_request_id = crate::utils::Utils::generate_request_id(&env);
                vr.request_id = new_request_id;
                env.storage().persistent().set(&crate::storage_keys::StorageKey::VerificationRecord(new_request_id), &vr);
            }
        }
        
        Ok(new_project_id)
    }


    pub fn set_project_lifecycle_status(
        env: Env,
        project_id: u64,
        caller: Address,
        status: ProjectLifecycleStatus,
    ) -> Result<Project, ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        ProjectRegistry::set_project_lifecycle_status(&env, project_id, caller, status)
    }

    pub fn merge_projects(
        env: Env,
        primary_project_id: u64,
        secondary_project_id: u64,
        primary_owner: Address,
        secondary_owner: Address,
        admin: Address,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        ProjectRegistry::merge_projects(
            &env,
            primary_project_id,
            secondary_project_id,
            primary_owner,
            secondary_owner,
            admin,
        )
    pub fn export_project_data(env: Env, project_id: u64, owner: Address) -> Result<crate::types::ProjectDataExport, ContractError> {
        ProjectRegistry::export_project_data(&env, project_id, owner)
    }

    pub fn import_project_data(env: Env, export: crate::types::ProjectDataExport, owner: Address) -> Result<u64, ContractError> {
        ProjectRegistry::import_project_data(&env, export, owner)
    }
    }

    pub fn schedule_project_sunset(
        env: Env,
        project_id: u64,
        caller: Address,
        sunset_at: u64,
        alternative_project_ids: Vec<u64>,
        redirect_project_id: Option<u64>,
    ) -> Result<ProjectSunsetPlan, ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        ProjectRegistry::schedule_project_sunset(
            &env,
            project_id,
            caller,
            sunset_at,
            alternative_project_ids,
            redirect_project_id,
        )
    }

    pub fn get_project_sunset_plan(env: Env, project_id: u64) -> Option<ProjectSunsetPlan> {
        ProjectRegistry::get_project_sunset_plan(&env, project_id)
    }

    pub fn get_project_redirect(env: Env, project_id: u64) -> Option<u64> {
        ProjectRegistry::get_project_redirect(&env, project_id)
    }

    pub fn process_project_sunset(
        env: Env,
        project_id: u64,
        caller: Address,
    ) -> Result<Project, ContractError> {
        ProjectRegistry::process_project_sunset(&env, project_id, caller)
    }

    pub fn update_security_contact(
        env: Env,
        project_id: u64,
        caller: Address,
        contact: Option<String>,
    ) -> Result<Project, ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        ProjectRegistry::update_security_contact(&env, project_id, caller, contact)
    }

    pub fn submit_security_contact_proof(
        env: Env,
        project_id: u64,
        caller: Address,
        proof_cid: String,
    ) -> Result<Project, ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
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
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        ProjectRegistry::link_project(&env, project_id, caller, linked_project_id)
    }

    pub fn unlink_project(
        env: Env,
        project_id: u64,
        caller: Address,
        linked_project_id: u64,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        ProjectRegistry::unlink_project(&env, project_id, caller, linked_project_id)
    }

    pub fn get_linked_projects(env: Env, project_id: u64) -> Vec<u64> {
        ProjectRegistry::get_linked_projects(&env, project_id)
    }

    /// Anonymous read: returns only `Public` projects (#874). Use
    /// `get_project_as_viewer` to read `Private` / `Restricted` projects.
    pub fn get_project(env: Env, project_id: u64) -> Option<Project> {
        ProjectRegistry::get_project(&env, project_id)
            .filter(|p| ProjectRegistry::can_view_project(&env, p, None))
    }

    /// Authenticated read honouring project visibility (#874).
    pub fn get_project_as_viewer(env: Env, project_id: u64, viewer: Address) -> Option<Project> {
        viewer.require_auth();
        ProjectRegistry::get_project(&env, project_id)
            .filter(|p| ProjectRegistry::can_view_project(&env, p, Some(&viewer)))
    }

    pub fn set_project_visibility(
        env: Env,
        project_id: u64,
        caller: Address,
        visibility: crate::types::ProjectVisibility,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env)?;
        ProjectRegistry::set_project_visibility(&env, project_id, caller, visibility)
    }

    pub fn get_project_versions(
        env: Env,
        project_id: u64,
        start: u32,
        limit: u32,
    ) -> Vec<ProjectVersion> {
        ProjectRegistry::get_project_versions(&env, project_id, start, limit)
    }

    pub fn restore_project_version(
        env: Env,
        project_id: u64,
        version: u32,
        admin: Address,
    ) -> Result<Project, ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        ProjectRegistry::restore_project_version(&env, project_id, version, admin)
    }

    pub fn get_project_by_slug(env: Env, slug: String) -> Option<Project> {
        ProjectRegistry::get_project_by_slug(&env, slug)
    }

    pub fn get_project_by_name(env: Env, name: String) -> Option<Project> {
        ProjectRegistry::get_project_by_name(&env, name)
    }

    pub fn initiate_transfer(
        env: Env,
        project_id: u64,
        caller: Address,
        new_owner: Address,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        ProjectRegistry::initiate_transfer(&env, project_id, caller, new_owner)
    }

    pub fn cancel_transfer(
        env: Env,
        project_id: u64,
        caller: Address,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
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

    /// Batch-fetch projects by ID in one call (max 100). See `get_projects_batch`.
    pub fn get_projects_batch(env: Env, project_ids: Vec<u64>) -> Vec<Project> {
        ProjectRegistry::get_projects_batch(&env, project_ids)
    }

    /// Sets an optional region tag for a project (owner only).
    /// Delegates to `ProjectRegistry::set_project_region` for the actual logic.
    pub fn set_project_region(
        env: Env,
        project_id: u64,
        caller: Address,
        region: Option<String>,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        ProjectRegistry::set_project_region(&env, project_id, caller, region)
    }

    pub fn set_project_region_hierarchy(
        env: Env,
        project_id: u64,
        caller: Address,
        region: Option<ProjectRegionHierarchy>,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        ProjectRegistry::set_project_region_hierarchy(&env, project_id, caller, region)
    }

    pub fn get_project_region_hierarchy(
        env: Env,
        project_id: u64,
    ) -> Option<ProjectRegionHierarchy> {
        ProjectRegistry::get_project_region_hierarchy(&env, project_id)
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Project Media Gallery (#158)
    // ─────────────────────────────────────────────────────────────────────────────

    /// Add a media entry to a project's gallery (owner only).
    pub fn add_media(
        env: Env,
        project_id: u64,
        caller: Address,
        cid: String,
        media_type: crate::types::MediaType,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        ProjectRegistry::add_media(&env, project_id, caller, cid, media_type)
    }

    /// Remove a media entry from a project's gallery by CID (owner only).
    pub fn remove_media(
        env: Env,
        project_id: u64,
        caller: Address,
        cid: String,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        ProjectRegistry::remove_media(&env, project_id, caller, cid)
    }

    /// Get media gallery for a project.
    pub fn get_media_gallery(env: Env, project_id: u64) -> Vec<crate::types::MediaEntry> {
        ProjectRegistry::get_media_gallery(&env, project_id)
    }

    pub fn list_projects_by_region(
        env: Env,
        continent: Option<String>,
        country_code: Option<String>,
        region: Option<String>,
        city: Option<String>,
        start_id: u64,
        limit: u32,
    ) -> Vec<Project> {
        ProjectRegistry::list_projects_by_region(
            &env,
            continent,
            country_code,
            region,
            city,
            start_id,
            limit,
        )
    }

    pub fn get_project_region_stats(
        env: Env,
        continent: Option<String>,
        country_code: Option<String>,
        region: Option<String>,
        city: Option<String>,
    ) -> ProjectRegionStats {
        ProjectRegistry::get_project_region_stats(
            &env,
            continent,
            country_code,
            region,
            city,
        )
    }

    /// Returns the region tag for a project, if set.
    pub fn get_project_region(env: Env, project_id: u64) -> Option<String> {
        ProjectRegistry::get_project_region(&env, project_id)
    }

    /// Returns the stored integrity hash for a project, if any.
    pub fn get_project_integrity_hash(env: Env, project_id: u64) -> Option<soroban_sdk::Bytes> {
        ProjectRegistry::get_project_integrity_hash(&env, project_id)
    }

    pub fn get_license_stats(env: Env, license: String) -> u32 {
        ProjectRegistry::get_license_stats(&env, license)
    }

    pub fn list_projects_by_status(
        env: Env,
        status: VerificationStatus,
        start_id: u64,
        limit: u32,
    ) -> Vec<Project> {
        ProjectRegistry::list_projects_by_status(&env, status, start_id, limit)
    }

    // ─────────────────────────────────────────────────────────────────────────────
    // Verification Pending List (#479) - Efficient admin dashboard retrieval
    // ─────────────────────────────────────────────────────────────────────────────

    /// Get the list of all verification requests currently in Pending status.
    /// Returns a Vec of request IDs in creation order without full scans.
    pub fn list_pending_verifications(env: Env, start_index: u32, limit: u32) -> Vec<u64> {
        let limit = if limit == 0 || limit > crate::constants::MAX_PAGE_LIMIT {
            crate::constants::MAX_PAGE_LIMIT
        } else {
            limit
        };

        let all_pending: Vec<u64> = env
            .storage()
            .persistent()
            .get(&ExtensionKey::PendingVerificationRequests)
            .unwrap_or_else(|| Vec::new(&env));

        let mut results = Vec::new(&env);
        let start = start_index as usize;
        let end = core::cmp::min(start + limit as usize, all_pending.len());

        if start < all_pending.len() {
            for i in start..end {
                results.push_back(all_pending.get(i as u32).unwrap());
            }
        }

        results
    }

    /// Get total count of pending verification requests (for pagination metadata).
    pub fn get_pending_verification_count(env: Env) -> u32 {
        let pending: Vec<u64> = env
            .storage()
            .persistent()
            .get(&ExtensionKey::PendingVerificationRequests)
            .unwrap_or_else(|| Vec::new(&env));
        pending.len() as u32
    }

    pub fn list_projects_by_category(
        env: Env,
        category: String,
        start_index: u32,
        limit: u32,
    ) -> Vec<Project> {
        ProjectRegistry::list_projects_by_category(&env, category, start_index, limit)
    }

    /// Search active projects by name, description, tags, or category.
    /// Results are relevance-ranked with rating and verification boosts.
    pub fn search_projects(
        env: Env,
        query: String,
        start_index: u32,
        limit: u32,
    ) -> Vec<Project> {
        ProjectRegistry::search_projects(&env, query, start_index, limit)
    }

    /// Search active projects with limit (starts at index 0) (#909).
    pub fn search_projects_with_limit(
        env: Env,
        query: String,
        limit: u32,
    ) -> Vec<Project> {
        ProjectRegistry::search_projects_with_limit(&env, query, limit)
    }


    /// List projects filtered by lifecycle status.
    ///
    /// Named `list_projects_by_lifecycle` rather than
    /// `..._by_lifecycle_status`: Soroban caps exported contract function
    /// names at 32 characters and the longer form is 33, which panics
    /// `#[contractimpl]` at compile time. The internal
    /// `ProjectRegistry::list_projects_by_lifecycle_status` keeps its full
    /// name, since the limit applies only to exported symbols.
    pub fn list_projects_by_lifecycle(
        env: Env,
        status: ProjectLifecycleStatus,
        start_id: u64,
        limit: u32,
    ) -> Vec<Project> {
        ProjectRegistry::list_projects_by_lifecycle_status(&env, status, start_id, limit)
    }

    pub fn list_projects_sorted(
        env: Env,
        sort_mode: ProjectSortMode,
        start_index: u64,
        limit: u32,
    ) -> Vec<Project> {
        ProjectRegistry::list_projects_sorted(&env, sort_mode, start_index, limit)
    }

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

    pub fn approve_contract_claim(
        env: Env,
        project_id: u64,
        contract_address: String,
        admin: Address,
    ) -> Result<ContractClaimRequest, ContractError> {
        ProjectRegistry::approve_contract_claim(&env, project_id, contract_address, admin)
    }

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

    pub fn archive_project(
        env: Env,
        project_id: u64,
        caller: Address,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        ProjectRegistry::archive_project(&env, project_id, caller)
    }

    pub fn reactivate_project(
        env: Env,
        project_id: u64,
        caller: Address,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        ProjectRegistry::reactivate_project(&env, project_id, caller)
    }

    pub fn archive_project_with_reason(
        env: Env,
        project_id: u64,
        caller: Address,
        reason: crate::types::ArchiveReason,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env)?;
        ProjectRegistry::archive_project_with_reason(&env, project_id, caller, reason)
    }

    pub fn bulk_archive_projects(
        env: Env,
        caller: Address,
        project_ids: Vec<u64>,
        reason: crate::types::ArchiveReason,
    ) -> Result<u32, ContractError> {
        EmergencyPause::require_not_paused(&env)?;
        ProjectRegistry::bulk_archive_projects(&env, caller, project_ids, reason)
    }

    pub fn get_archive_reason(env: Env, project_id: u64) -> Option<crate::types::ArchiveReason> {
        ProjectRegistry::get_archive_reason(&env, project_id)
    }

    pub fn get_archive_audit_log(
        env: Env,
        project_id: u64,
        start: u32,
        limit: u32,
    ) -> Vec<crate::types::ArchiveAuditEntry> {
        ProjectRegistry::get_archive_audit_log(&env, project_id, start, limit)
    }

    // ── Automatic Inactivity Archival (#753) ───────────────────────────────

    pub fn get_auto_archive_config(env: Env) -> crate::auto_archive_types::AutoArchiveConfig {
        crate::auto_archive_registry::AutoArchiveRegistry::get_config(&env)
    }

    /// Configure the global inactivity threshold (30 days through 10 years).
    pub fn set_auto_archive_config(
        env: Env,
        admin: Address,
        config: crate::auto_archive_types::AutoArchiveConfig,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        crate::auto_archive_registry::AutoArchiveRegistry::set_config(&env, admin, config)
    }

    /// Permissionless keeper action that emits the owner-facing notice once the
    /// project enters the 30-day pre-archival window.
    pub fn notify_project_auto_archive(
        env: Env,
        keeper: Address,
        project_id: u64,
    ) -> Result<bool, ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        crate::auto_archive_registry::AutoArchiveRegistry::notify_project(
            &env,
            keeper,
            project_id,
        )
    }

    pub fn get_auto_archive_notice(
        env: Env,
        project_id: u64,
    ) -> Option<crate::auto_archive_types::AutoArchiveNotice> {
        crate::auto_archive_registry::AutoArchiveRegistry::get_notice(&env, project_id)
    }

    /// Record off-chain delivery success/failure for the owner notice.
    pub fn record_auto_archive_delivery(
        env: Env,
        admin: Address,
        project_id: u64,
        delivered: bool,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        crate::auto_archive_registry::AutoArchiveRegistry::record_notice_delivery(
            &env,
            admin,
            project_id,
            delivered,
        )
    }

    /// Archive a bounded range of projects whose notices have fully elapsed.
    pub fn archive_inactive_projects(
        env: Env,
        keeper: Address,
        start_project_id: u64,
        limit: u32,
    ) -> Result<crate::auto_archive_types::AutoArchiveBatchResult, ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        crate::auto_archive_registry::AutoArchiveRegistry::archive_inactive_batch(
            &env,
            keeper,
            start_project_id,
            limit,
        )
    }

    pub fn get_auto_archive_record(
        env: Env,
        project_id: u64,
    ) -> Option<crate::auto_archive_types::AutoArchiveRecord> {
        crate::auto_archive_registry::AutoArchiveRegistry::get_record(&env, project_id)
    }

    /// Owner/admin restoration entry point for automatically archived projects.
    pub fn restore_auto_archived_project(
        env: Env,
        project_id: u64,
        caller: Address,
    ) -> Result<crate::types::Project, ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        crate::auto_archive_registry::AutoArchiveRegistry::restore_archived_project(
            &env,
            project_id,
            caller,
        )
    }

    /// Cursor-based search over archived project name, slug, category, and text.
    pub fn search_archived_projects(
        env: Env,
        query: String,
        start_cursor: u64,
        limit: u32,
    ) -> Result<crate::auto_archive_types::ArchivedProjectSearchResult, ContractError> {
        crate::auto_archive_registry::AutoArchiveRegistry::search_archived_projects(
            &env,
            query,
            start_cursor,
            limit,
        )
    }

    pub fn add_maintainer(
        env: Env,
        project_id: u64,
        caller: Address,
        maintainer: Address,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
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

    pub fn list_featured_projects(env: Env, start_index: u32, limit: u32) -> Vec<Project> {
        FeaturedRegistry::list_featured_projects(&env, start_index, limit)
    }

    /// Return the number of projects currently in the featured list.
    pub fn get_featured_count(env: Env) -> u32 {
        FeaturedRegistry::get_featured_count(&env)
    }

    // --- Featured Rotation (#927) ---

    /// Admin: schedule (or replace) an automatic rotation through `projects`,
    /// advancing to the next project every `rotation_period_days`.
    pub fn schedule_featured_rotation(
        env: Env,
        admin: Address,
        projects: Vec<u64>,
        rotation_period_days: u32,
    ) -> Result<FeaturedRotationSchedule, ContractError> {
        crate::featured_rotation_registry::FeaturedRotationRegistry::schedule_featured_rotation(
            &env,
            admin,
            projects,
            rotation_period_days,
        )
    }

    /// The rotation schedule evaluated at the current ledger time (lazy:
    /// the current project advances without any transaction).
    pub fn get_featured_rotation_schedule(env: Env) -> Option<FeaturedRotationSchedule> {
        crate::featured_rotation_registry::FeaturedRotationRegistry::get_featured_rotation_schedule(
            &env,
        )
    }

    /// The project currently featured by the rotation, if any.
    pub fn get_current_rotation_project(env: Env) -> Option<u64> {
        crate::featured_rotation_registry::FeaturedRotationRegistry::get_current_rotation_project(
            &env,
        )
    }

    /// Persist the current rotation slot and emit `FEATURED/ROTATED` when
    /// the featured project changed. Callable by anyone.
    pub fn advance_featured_rotation(env: Env) -> Result<FeaturedRotationSchedule, ContractError> {
        crate::featured_rotation_registry::FeaturedRotationRegistry::advance_featured_rotation(&env)
    }

    /// Admin: remove the featured rotation schedule.
    pub fn clear_featured_rotation(env: Env, admin: Address) -> Result<(), ContractError> {
        crate::featured_rotation_registry::FeaturedRotationRegistry::clear_featured_rotation(
            &env, admin,
        )
    }

    // --- Feature Campaigns (#926) ---

    /// Admin: create a time-bound feature campaign. Returns its ID.
    pub fn create_feature_campaign(
        env: Env,
        admin: Address,
        campaign_config: FeatureCampaignConfig,
    ) -> Result<u64, ContractError> {
        crate::campaign_registry::CampaignRegistry::create_feature_campaign(
            &env,
            admin,
            campaign_config,
        )
    }

    /// Admin: add a project to a campaign that has not yet ended.
    pub fn add_to_campaign(
        env: Env,
        admin: Address,
        project_id: u64,
        campaign_id: u64,
    ) -> Result<(), ContractError> {
        crate::campaign_registry::CampaignRegistry::add_to_campaign(
            &env,
            admin,
            project_id,
            campaign_id,
        )
    }

    /// Projects in a campaign, in insertion order, paginated.
    pub fn get_projects_by_campaign(
        env: Env,
        campaign_id: u64,
        start_index: u32,
        limit: u32,
    ) -> Result<Vec<Project>, ContractError> {
        crate::campaign_registry::CampaignRegistry::get_projects_by_campaign(
            &env,
            campaign_id,
            start_index,
            limit,
        )
    }

    /// Record a viewer impression or engagement on a project in an active
    /// campaign.
    pub fn record_campaign_interaction(
        env: Env,
        viewer: Address,
        campaign_id: u64,
        project_id: u64,
        interaction: CampaignInteraction,
    ) -> Result<(), ContractError> {
        crate::campaign_registry::CampaignRegistry::record_campaign_interaction(
            &env,
            viewer,
            campaign_id,
            project_id,
            interaction,
        )
    }

    pub fn get_feature_campaign(env: Env, campaign_id: u64) -> Option<FeatureCampaign> {
        crate::campaign_registry::CampaignRegistry::get_feature_campaign(&env, campaign_id)
    }

    pub fn list_feature_campaigns(env: Env, start_index: u32, limit: u32) -> Vec<FeatureCampaign> {
        crate::campaign_registry::CampaignRegistry::list_feature_campaigns(
            &env,
            start_index,
            limit,
        )
    }

    /// Campaigns running at the current ledger timestamp.
    pub fn get_active_campaigns(env: Env) -> Vec<FeatureCampaign> {
        crate::campaign_registry::CampaignRegistry::get_active_campaigns(&env)
    }

    /// Aggregate impression / engagement counters for a campaign.
    pub fn get_campaign_stats(env: Env, campaign_id: u64) -> CampaignStats {
        crate::campaign_registry::CampaignRegistry::get_campaign_stats(&env, campaign_id)
    }

    /// Impression / engagement counters for one project within a campaign.
    pub fn get_campaign_project_stats(env: Env, campaign_id: u64, project_id: u64) -> CampaignStats {
        crate::campaign_registry::CampaignRegistry::get_campaign_project_stats(
            &env,
            campaign_id,
            project_id,
        )
    }

    // --- Review Registry ---

    pub fn add_review(
        env: Env,
        project_id: u64,
        reviewer: Address,
        rating: u32,
        comment_cid: Option<String>,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::Reviews)?;
        ReviewRegistry::add_review(&env, project_id, reviewer, rating, comment_cid, None, None)
    }

    pub fn add_review_with_attribution(
        env: Env,
        project_id: u64,
        reviewer: Address,
        rating: u32,
        comment_cid: Option<String>,
        attribution: ReviewAttribution,
        reviewer_name: Option<String>,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::Reviews)?;
        ReviewRegistry::add_review_with_attribution(
            &env,
            project_id,
            reviewer,
            rating,
            comment_cid,
            None,
            attribution,
            reviewer_name,
        )
    }

    pub fn update_review(
        env: Env,
        project_id: u64,
        reviewer: Address,
        rating: u32,
        comment_cid: Option<String>,
        language_code: Option<String>,
    ) -> Result<(), ContractError> {
        ReviewRegistry::update_review(&env, project_id, reviewer, rating, comment_cid, language_code)
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
        language_code: Option<String>,
    ) -> Result<(), ContractError> {
        ReviewRegistry::submit_review(&env, project_id, reviewer, rating, review_cid, language_code)
    }

    pub fn submit_review_with_attribution(
        env: Env,
        project_id: u64,
        reviewer: Address,
        rating: u32,
        review_cid: String,
        attribution: ReviewAttribution,
        reviewer_name: Option<String>,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::Reviews)?;
        ReviewRegistry::submit_review_with_attribution(
            &env,
            project_id,
            reviewer,
            rating,
            review_cid,
            attribution,
            reviewer_name,
        )
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

    /// Batch-fetch reviews in one call (max 100). Same filtering as `get_reviews_by_ids`.
    pub fn get_reviews_batch(env: Env, ids: Vec<(u64, Address)>) -> Vec<Review> {
        ReviewRegistry::get_reviews_batch(&env, ids)
    }

    pub fn list_reviews(env: Env, project_id: u64, start_index: u32, limit: u32) -> Vec<Review> {
        ReviewRegistry::list_reviews(&env, project_id, start_index, limit)
    }

    pub fn list_anonymous_reviews(
        env: Env,
        project_id: u64,
        start_index: u32,
        limit: u32,
    ) -> Vec<PublicReview> {
        ReviewRegistry::list_public_reviews(
            &env,
            project_id,
            start_index,
            limit,
            ReviewAttribution::Anonymous,
        )
    }

    pub fn list_attributed_reviews(
        env: Env,
        project_id: u64,
        start_index: u32,
        limit: u32,
    ) -> Vec<PublicReview> {
        ReviewRegistry::list_public_reviews(
            &env,
            project_id,
            start_index,
            limit,
            ReviewAttribution::Attributed,
        )
    }

    /// Admin-only: archive reviews older than 2 years for a project.
    ///
    /// Moves eligible reviews to compact archived storage, removes them from
    /// primary storage and active indexes, and emits `ReviewArchivedEvent` for
    /// each archived review so off-chain jobs can persist the full payload to
    /// permanent storage (e.g., Arweave/IPFS).
    ///
    /// `batch_size` limits how many reviews are processed per call
    /// (capped at `MAX_ARCHIVE_BATCH_SIZE = 50`). Call repeatedly to process
    /// large projects. Returns the number of reviews archived.
    pub fn archive_old_reviews(
        env: Env,
        admin: Address,
        project_id: u64,
        batch_size: u32,
    ) -> Result<u32, ContractError> {
        ReviewRegistry::archive_old_reviews(&env, admin, project_id, batch_size)
    }

    /// Retrieve the compact archived record for a review that has been archived.
    ///
    /// Returns `None` when the review was never archived or its on-chain
    /// archived record has expired. For active reviews, use `get_review`.
    pub fn get_archived_review(
        env: Env,
        project_id: u64,
        reviewer: Address,
    ) -> Option<ArchivedReview> {
        ReviewRegistry::get_archived_review(&env, project_id, reviewer)
    }

    /// Return a paginated list of archived reviews for a project.
    ///
    /// Results are in archival order (oldest archived first). Use
    /// `start_index` / `limit` for pagination.
    pub fn list_archived_reviews(
        env: Env,
        project_id: u64,
        start_index: u32,
        limit: u32,
    ) -> Vec<ArchivedReview> {
        ReviewRegistry::list_archived_reviews(&env, project_id, start_index, limit)
    }

    /// Admin-only: record the Arweave transaction ID for an archived review.
    ///
    /// Off-chain archival jobs call this after successfully writing the full
    /// review payload to Arweave permanent storage.
    pub fn set_archived_review_arweave_tx(
        env: Env,
        admin: Address,
        project_id: u64,
        reviewer: Address,
        arweave_tx_id: String,
    ) -> Result<(), ContractError> {
        ReviewRegistry::set_archived_review_arweave_tx(
            &env,
            admin,
            project_id,
            reviewer,
            arweave_tx_id,
        )
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

    /// Admin hide multiple reviews in batch (admin-only).
    /// Each tuple is (project_id, reviewer).
    pub fn hide_reviews_batch(
        env: Env,
        reviews: Vec<(u64, Address)>,
        admin: Address,
    ) -> Result<(), ContractError> {
        admin.require_auth();

        // Check if admin
        if !crate::admin_manager::AdminManager::is_admin(&env, &admin) {
            return Err(ContractError::AdminOnly);
        }

        // Limit batch size for gas efficiency
        if reviews.len() > crate::constants::MAX_TTL_BATCH_SIZE as usize {
            return Err(ContractError::InvalidInput);
        }

        // Process each review
        for (project_id, reviewer) in reviews.iter() {
            // Check if project exists
            if ProjectRegistry::get_project(&env, *project_id).is_none() {
                return Err(ContractError::ProjectNotFound);
            }

            // Hide the review
            ReviewRegistry::hide_review(&env, *project_id, reviewer.clone(), admin.clone())?;
        }

        Ok(())
    }

    pub fn restore_review(
        env: Env,
        project_id: u64,
        reviewer: Address,
        admin: Address,
    ) -> Result<(), ContractError> {
        ReviewRegistry::restore_review(&env, project_id, reviewer, admin)
    }

    /// Hide, delete, flag, or restore up to `MAX_MODERATION_BATCH_SIZE`
    /// reviews in one call (admin-only). Each tuple is `(project_id, reviewer)`.
    ///
    /// Per-review failures are returned in `ModerateResult::failed` rather than
    /// aborting the batch. Every item, successful or not, counts toward the
    /// admin's `MODERATION_ACTIONS_PER_WINDOW` budget; a batch that would
    /// exceed it is rejected with `ModerationRateLimitExceeded` before any write.
    pub fn moderate_reviews_batch(
        env: Env,
        admin: Address,
        reviews: Vec<(u64, Address)>,
        action: crate::types::ModerateAction,
    ) -> Result<crate::types::ModerateResult, ContractError> {
        ReviewRegistry::moderate_reviews_batch(&env, admin, reviews, action)
    }

    /// Return the moderator flag on a review, if any.
    pub fn get_review_flag(
        env: Env,
        project_id: u64,
        reviewer: Address,
    ) -> Option<crate::types::ReviewFlag> {
        ReviewRegistry::get_review_flag(&env, project_id, reviewer)
    }

    /// Moderation actions `admin` may still attempt in the current window.
    pub fn get_moderation_budget(env: Env, admin: Address) -> u32 {
        ReviewRegistry::get_moderation_budget(&env, admin)
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

    /// List a bounded review page. Sort the returned pages client-side.
    pub fn list_reviews_sorted(
        env: Env,
        project_id: u64,
        start_index: u32,
        limit: u32,
        sort_mode: ReviewSortMode,
    ) -> Vec<Review> {
        ReviewRegistry::list_reviews_sorted(&env, project_id, start_index, limit, sort_mode)
    }

    // --- Review Content Integrity (#809) ---

    /// Verify the content integrity of a stored review.
    ///
    /// Recomputes the SHA-256 seal over the review's current on-chain fields
    /// (`project_id`, `reviewer`, `rating`, `content_cid`) and compares it
    /// against the integrity record written at create / update time.
    ///
    /// Returns:
    /// - `ReviewIntegrityStatus::Valid` — content matches the seal.
    /// - `ReviewIntegrityStatus::Tampered` — mismatch detected; a
    ///   `ReviewIntegrityViolationEvent` is emitted as a warning.
    /// - `ReviewIntegrityStatus::Unverifiable` — no seal exists (review
    ///   predates sealing, or the review itself does not exist).
    pub fn verify_review_integrity(
        env: Env,
        project_id: u64,
        reviewer: Address,
    ) -> crate::types::ReviewIntegrityStatus {
        ReviewRegistry::verify_review_integrity(&env, project_id, reviewer)
    }

    /// Return the stored integrity seal record for a review.
    ///
    /// Returns `None` when no seal exists (review predates integrity sealing).
    pub fn get_review_integrity_record(
        env: Env,
        project_id: u64,
        reviewer: Address,
    ) -> Option<crate::types::ReviewIntegrityRecord> {
        ReviewRegistry::get_review_integrity_record(&env, project_id, reviewer)
    }

    // --- Reviewer Rewards ---

    pub fn configure_reward_pool(env: Env, admin: Address, token: Address, period_duration: u64, max_reviewers: u32) -> Result<(), ContractError> {
        reward_registry::RewardRegistry::configure(&env, admin, token, period_duration, max_reviewers)
    }

    pub fn get_reward_pool_config(env: Env) -> Option<RewardPoolConfig> {
        reward_registry::RewardRegistry::get_config(&env)
    }

    pub fn fund_reward_pool(env: Env, admin: Address, amount: u128) -> Result<(), ContractError> {
        reward_registry::RewardRegistry::fund(&env, admin, amount)
    }

    pub fn set_review_quality_score(env: Env, admin: Address, project_id: u64, reviewer: Address, score: u32) -> Result<(), ContractError> {
        reward_registry::RewardRegistry::set_quality_score(&env, admin, project_id, reviewer, score)
    }

    pub fn finalize_reward_period(env: Env, admin: Address, start_at: u64, end_at: u64, candidates: Vec<Address>) -> Result<u64, ContractError> {
        reward_registry::RewardRegistry::finalize(&env, admin, start_at, end_at, candidates)
    }

    pub fn get_reward_period(env: Env, period_id: u64) -> Option<RewardPeriod> {
        reward_registry::RewardRegistry::get_period(&env, period_id)
    }

    pub fn claim_reward(env: Env, reviewer: Address, period_id: u64) -> Result<u128, ContractError> {
        reward_registry::RewardRegistry::claim(&env, reviewer, period_id)
    }

    // --- Verification Registry ---

    pub fn request_verification(
        env: Env,
        project_id: u64,
        requester: Address,
        evidence_cid: String,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::Verification)?;
        VerificationRegistry::request_verification(&env, project_id, requester, evidence_cid)
    }

    pub fn get_verification_risk_model(env: Env) -> VerificationRiskModel {
        VerificationRegistry::get_verification_risk_model(&env)
    }

    pub fn set_verification_risk_model(
        env: Env,
        admin: Address,
        model: VerificationRiskModel,
    ) -> Result<(), ContractError> {
        VerificationRegistry::set_verification_risk_model(&env, admin, model)
    }

    pub fn get_verification_risk_assessment(
        env: Env,
        request_id: u64,
    ) -> Option<VerificationRiskAssessment> {
        VerificationRegistry::get_verification_risk_assessment(&env, request_id)
    }

    pub fn get_high_risk_requests(env: Env) -> Vec<u64> {
        VerificationRegistry::get_high_risk_verification_requests(&env)
    }

    pub fn override_verification_risk(
        env: Env,
        request_id: u64,
        admin: Address,
        flagged: bool,
    ) -> Result<(), ContractError> {
        VerificationRegistry::override_verification_risk(&env, request_id, admin, flagged)
    }

    /// Update the verification evidence CID for a pending verification request.
    ///
    /// # Restrictions
    /// - Only the project owner can update the evidence.
    /// - Updates are allowed while the request is `Pending` or `Verified`.
    /// - Updating a verified request resets it to `Pending` and requires a new approval.
    /// - Rejected and revoked requests are immutable and reject further updates.
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

    /// Atomically approve or reject up to 100 verification requests.
    /// The entire batch is validated before any request is changed.
    pub fn decide_verifications_batch(
        env: Env,
        request_ids: Vec<u64>,
        admin: Address,
        action: VerificationBatchAction,
    ) -> Result<VerificationBatchReport, ContractError> {
        VerificationRegistry::decide_verifications_batch(&env, request_ids, admin, action)
    }

    pub fn approve_verifications_batch(
        env: Env,
        request_ids: Vec<u64>,
        admin: Address,
    ) -> Result<VerificationBatchReport, ContractError> {
        VerificationRegistry::approve_verifications_batch(&env, request_ids, admin)
    }

    pub fn reject_verifications_batch(
        env: Env,
        request_ids: Vec<u64>,
        admin: Address,
    ) -> Result<VerificationBatchReport, ContractError> {
        VerificationRegistry::reject_verifications_batch(&env, request_ids, admin)
    }

    /// Submit additional evidence to appeal a rejection.
    pub fn submit_verification_appeal(
        env: Env,
        project_id: u64,
        owner: Address,
        evidence_cid: String,
    ) -> Result<(), ContractError> {
        VerificationRegistry::submit_verification_appeal(&env, project_id, owner, evidence_cid)
    }

    /// Review the latest appeal for a rejected verification.
    pub fn review_verification_appeal(
        env: Env,
        project_id: u64,
        admin: Address,
        approved: bool,
    ) -> Result<(), ContractError> {
        VerificationRegistry::review_verification_appeal(&env, project_id, admin, approved)
    }

    /// Read the appeal history for a rejected verification request.
    pub fn get_verification_appeals(
        env: Env,
        project_id: u64,
    ) -> Vec<crate::types::VerificationAppeal> {
        VerificationRegistry::get_verification_appeals(&env, project_id)
    }

    pub fn revoke_verification(
        env: Env,
        project_id: u64,
        admin: Address,
        reason: String,
    ) -> Result<(), ContractError> {
        VerificationRegistry::revoke_verification(&env, project_id, admin, reason)
    }

    pub fn suspend_verification(
        env: Env,
        project_id: u64,
        admin: Address,
        reason: String,
        investigation_ticket: String,
    ) -> Result<(), ContractError> {
        VerificationRegistry::suspend_verification(
            &env,
            project_id,
            admin,
            reason,
            investigation_ticket,
        )
    }

    pub fn restore_verification(
        env: Env,
        project_id: u64,
        admin: Address,
    ) -> Result<(), ContractError> {
        VerificationRegistry::restore_verification(&env, project_id, admin)
    }

    pub fn get_verification(env: Env, project_id: u64) -> Option<VerificationRecord> {
        VerificationRegistry::get_verification(&env, project_id)
    }

    pub fn get_verification_record(env: Env, request_id: u64) -> Option<VerificationRecord> {
        VerificationRegistry::get_verification_record(&env, request_id)
    }

    pub fn get_evidence_versions(
        env: Env,
        request_id: u64,
    ) -> Vec<crate::types::VerificationEvidenceVersion> {
        VerificationRegistry::get_verification_evidence_versions(&env, request_id)
    }

    pub fn get_evidence_version(
        env: Env,
        request_id: u64,
        version: u32,
    ) -> Option<crate::types::VerificationEvidenceVersion> {
        VerificationRegistry::get_verification_evidence_version(&env, request_id, version)
    }

    pub fn compare_verification_evidence(
        env: Env,
        request_id: u64,
        first_version: u32,
        second_version: u32,
    ) -> Option<crate::types::VerificationEvidenceComparison> {
        VerificationRegistry::compare_verification_evidence(
            &env,
            request_id,
            first_version,
            second_version,
        )
    }

    pub fn get_pending_verifications(env: Env, start: u32, limit: u32) -> Vec<VerificationRecord> {
        VerificationRegistry::get_pending_verifications(&env, start, limit)
    }

    pub fn get_verifications_batch(env: Env, ids: Vec<u64>) -> Vec<(u64, VerificationRecord)> {
        VerificationRegistry::get_verifications_batch(&env, ids)
    }

    pub fn get_verification_records_batch(
        env: Env,
        request_ids: Vec<u64>,
    ) -> Vec<(u64, VerificationRecord)> {
        VerificationRegistry::get_verification_records_batch(&env, request_ids)
    }

    // --- Custom Verification Requirements ---

    /// Get the verification requirements for a project.
    ///
    /// Returns the list of IPFS CIDs pointing to requirement definition documents
    /// for the given project. These define additional criteria that must be met
    /// before verification approval.
    pub fn get_project_verification_requirements(
        env: Env,
        project_id: u64,
    ) -> Option<Vec<String>> {
        VerificationRegistry::get_project_verification_requirements(&env, project_id)
    }

    /// Audit verification requirements for a project to ensure they are consistent
    /// with the project's tier.
    ///
    /// Admins can use this to verify that custom requirements match what is expected
    /// for projects at a given verification tier. Returns an audit report with any
    /// inconsistencies found.
    pub fn audit_verification_requirements(
        env: Env,
        project_id: u64,
        admin: Address,
    ) -> Result<crate::types::VerificationRequirementAudit, ContractError> {
        VerificationRegistry::audit_verification_requirements(&env, project_id, admin)
    }

    // --- Probationary Verification & Enhanced Monitoring ---

    /// Returns the configured probationary duration in seconds (default: 30 days).
    pub fn get_probation_duration(env: Env) -> u64 {
        ProbationRegistry::get_probation_duration(&env)
    }

    /// Admin-only: configure probationary duration in seconds.
    pub fn set_probation_duration(
        env: Env,
        admin: Address,
        duration_secs: u64,
    ) -> Result<(), ContractError> {
        ProbationRegistry::set_probation_duration(&env, admin, duration_secs)
    }

    /// Checks if a project is currently within its 30-day probationary period.
    pub fn is_in_probation(env: Env, project_id: u64) -> bool {
        ProbationRegistry::is_in_probation(&env, project_id)
    }

    /// Fetches the probation status record for a project.
    pub fn get_probation_record(env: Env, project_id: u64) -> Option<ProbationRecord> {
        ProbationRegistry::get_probation_record(&env, project_id)
    }

    /// Returns effective verification status (Probationary if in active 30-day probation).
    pub fn get_effective_verification(env: Env, project_id: u64) -> Option<VerificationStatus> {
        ProbationRegistry::get_effective_verification_status(&env, project_id)
    }

    /// Auto-promotes a project to full verification after the 30-day probationary period.
    pub fn check_and_promote_probation(env: Env, project_id: u64) -> Result<bool, ContractError> {
        ProbationRegistry::check_and_promote_probation(&env, project_id)
    }

    /// Fast-track revocation during probation without requiring full review.
    pub fn revoke_during_probation(
        env: Env,
        admin: Address,
        project_id: u64,
        reason: String,
    ) -> Result<(), ContractError> {
        ProbationRegistry::revoke_during_probation(&env, admin, project_id, reason)
    }

    /// Enhanced monitoring: records an incident or discrepancy against a project in probation.
    pub fn record_probation_incident(
        env: Env,
        reporter: Address,
        project_id: u64,
        details: String,
    ) -> Result<u32, ContractError> {
        ProbationRegistry::record_probation_incident(&env, reporter, project_id, details)
    }

    /// Returns the incident count recorded for a project during probation.
    pub fn get_probation_incident_count(env: Env, project_id: u64) -> u32 {
        ProbationRegistry::get_probation_incident_count(&env, project_id)
    }

    /// Lists project IDs currently under active probation and enhanced monitoring.
    pub fn list_probationary_projects(env: Env, start_index: u32, limit: u32) -> Vec<u64> {
        ProbationRegistry::list_probationary_projects(&env, start_index, limit)
    }

    /// Read the refund recorded after a rejected verification (issue #472).
    ///
    /// Returns `None` if the project has no refund on record. A record with
    /// `claimed_at: Some(_)` has already been paid out.
    pub fn get_fee_refund(env: Env, project_id: u64) -> Option<FeeRefundRecord> {
        FeeManager::get_fee_refund(&env, project_id)
    }

    /// Pay out a recorded refund to the original fee payer.
    ///
    /// Callable by the payer or any admin. Funds always go to the recorded
    /// payer, so an admin settling on someone's behalf cannot redirect them.
    /// The transaction must also carry the treasury's authorization.
    pub fn claim_fee_refund(
        env: Env,
        caller: Address,
        project_id: u64,
    ) -> Result<(), ContractError> {
        FeeManager::claim_fee_refund(&env, caller, project_id)
    }

    pub fn is_verification_active(env: Env, project_id: u64) -> bool {
        VerificationRegistry::is_verification_active(&env, project_id)
    }

    pub fn renew_verification(
        env: Env,
        project_id: u64,
        admin: Address,
    ) -> Result<(), ContractError> {
        VerificationRegistry::renew_verification(&env, project_id, admin)
    }

    pub fn get_verification_history(env: Env, project_id: u64) -> Vec<VerificationRecord> {
        VerificationRegistry::get_verification_history(&env, project_id)
    }

    pub fn get_suspension_timeline(env: Env, project_id: u64) -> Vec<VerificationSuspension> {
        VerificationRegistry::get_verification_suspension_timeline(&env, project_id)
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

    pub fn process_verification_expiry(env: Env, project_id: u64) -> Result<bool, ContractError> {
        VerificationRegistry::process_verification_expiry(&env, project_id)
    }

    /// Returns whether a non-expired verification will expire within the
    /// supplied threshold. This is a read-only renewal-warning helper.
    pub fn is_verification_expiring_soon(
        env: Env,
        project_id: u64,
        threshold_seconds: u64,
    ) -> Result<bool, ContractError> {
        VerificationRegistry::is_verification_expiring_soon(&env, project_id, threshold_seconds)
    }

    pub fn process_expiry_notification(env: Env, project_id: u64) -> Result<bool, ContractError> {
        VerificationRegistry::process_verification_expiry_notification(&env, project_id)
    }

    pub fn get_expiry_notification(
        env: Env,
        project_id: u64,
    ) -> Option<VerificationExpiryNotification> {
        VerificationRegistry::get_verification_expiry_notification(&env, project_id)
    }

    pub fn record_expiry_notification_sent(
        env: Env,
        project_id: u64,
        admin: Address,
        delivered: bool,
    ) -> Result<(), ContractError> {
        VerificationRegistry::record_verification_expiry_notification_delivery(
            &env, project_id, admin, delivered,
        )
    }

    pub fn get_notification_delivery_status(
        env: Env,
        project_id: u64,
    ) -> Option<NotificationDeliveryStatus> {
        VerificationRegistry::get_verification_expiry_notification(&env, project_id)
            .map(|notification| notification.delivery_status)
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

    // --- Verification Assignment & Specialized Admin Routing ---

    /// Admin: assign a pending verification request to an admin with specific expertise.
    pub fn assign_verification_expertise(
        env: Env,
        project_id: u64,
        admin: Address,
        assignee: Address,
        expertise: String,
    ) -> Result<u64, ContractError> {
        VerificationAssignmentRegistry::assign_verification_with_expertise(
            &env,
            project_id,
            admin,
            assignee,
            Some(expertise),
        )
    }

    /// Admin: automatically route a pending verification request to an admin specialized in the given expertise.
    pub fn route_verification_to_expert(
        env: Env,
        project_id: u64,
        admin: Address,
        expertise: String,
    ) -> Result<Address, ContractError> {
        VerificationAssignmentRegistry::route_verification_to_expert(
            &env, project_id, admin, expertise,
        )
    }

    /// Admin: set or update the specialized expertise domains for an admin.
    pub fn set_admin_expertise(
        env: Env,
        caller: Address,
        admin: Address,
        expertise: Vec<String>,
    ) -> Result<(), ContractError> {
        VerificationAssignmentRegistry::set_admin_expertise(&env, caller, admin, expertise)
    }

    /// Get the expertise domains registered for an admin.
    pub fn get_admin_expertise(env: Env, admin: Address) -> Vec<String> {
        VerificationAssignmentRegistry::get_admin_expertise(&env, admin)
    }

    /// Get all admins registered with a specific expertise domain.
    pub fn get_admins_by_expertise(env: Env, expertise: String) -> Vec<Address> {
        VerificationAssignmentRegistry::get_admins_by_expertise(&env, expertise)
    }

    /// Assigned Admin: accept the verification assignment to begin review.
    pub fn accept_verification_assignment(
        env: Env,
        project_id: u64,
        admin: Address,
    ) -> Result<(), ContractError> {
        VerificationAssignmentRegistry::accept_verification_assignment(&env, project_id, admin)
    }

    /// Assigned Admin: decline the verification assignment with a reason, releasing it for reassignment.
    pub fn decline_verification_assignment(
        env: Env,
        project_id: u64,
        admin: Address,
        reason: String,
    ) -> Result<(), ContractError> {
        VerificationAssignmentRegistry::decline_verification_assignment(
            &env, project_id, admin, reason,
        )
    }

    /// Get the active assignment record for a project's verification request, if any.
    pub fn get_current_assignment(env: Env, project_id: u64) -> Option<VerificationAssignment> {
        VerificationAssignmentRegistry::get_current_verification_assignment(&env, project_id)
    }

    /// Get a specific verification assignment by its unique assignment ID.
    pub fn get_verification_assignment(
        env: Env,
        assignment_id: u64,
    ) -> Option<VerificationAssignment> {
        VerificationAssignmentRegistry::get_verification_assignment(&env, assignment_id)
    }

    /// Get the complete historical log of verification assignments for a project.
    pub fn get_assignment_history(env: Env, project_id: u64) -> Vec<VerificationAssignment> {
        VerificationAssignmentRegistry::get_verification_assignment_history(&env, project_id)
    }

    /// Get paginated verification assignment history for a project.
    pub fn list_assignment_history(
        env: Env,
        project_id: u64,
        start_index: u32,
        limit: u32,
    ) -> Vec<VerificationAssignment> {
        VerificationAssignmentRegistry::get_verification_assignment_history_paginated(
            &env,
            project_id,
            start_index,
            limit,
        )
    }

    /// Get all verification assignments associated with a specific admin.
    pub fn get_admin_assignments(env: Env, admin: Address) -> Vec<VerificationAssignment> {
        VerificationAssignmentRegistry::get_admin_assignments(&env, admin)
    }

    /// Get workload statistics for an admin.
    pub fn get_admin_workload(env: Env, admin: Address) -> AdminWorkload {
        VerificationAssignmentRegistry::get_admin_workload(&env, admin)
    }

    /// Admin: set the global SLA duration (in seconds) for verification assignments.
    pub fn set_verification_sla(
        env: Env,
        admin: Address,
        sla_seconds: u64,
    ) -> Result<(), ContractError> {
        VerificationAssignmentRegistry::set_verification_sla(&env, admin, sla_seconds)
    }

    /// Get the current verification review SLA duration (in seconds).
    pub fn get_verification_sla(env: Env) -> u64 {
        VerificationAssignmentRegistry::get_verification_sla(&env)
    }

    /// Check if the active assignment for a project has breached its review SLA deadline.
    pub fn is_assignment_sla_breached(env: Env, project_id: u64) -> bool {
        VerificationAssignmentRegistry::is_assignment_sla_breached(&env, project_id)
    }

    /// Get remaining seconds until the active assignment's SLA deadline lapses.
    pub fn get_assignment_sla_remaining(env: Env, project_id: u64) -> Option<u64> {
        VerificationAssignmentRegistry::get_assignment_sla_remaining(&env, project_id)
    }

    /// Admin: escalate an overdue or unhandled verification assignment that breached SLA.
    pub fn escalate_verification_assignment(
        env: Env,
        caller: Address,
        project_id: u64,
        reason: String,
    ) -> Result<(), ContractError> {
        VerificationAssignmentRegistry::escalate_verification_assignment(
            &env, caller, project_id, reason,
        )
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
        EmergencyPause::require_not_paused(&env, PauseOperation::Fees)?;
        FeeManager::pay_fee(&env, payer, project_id, token)
    }

    pub fn cancel_fee_payment(
        env: Env,
        caller: Address,
        project_id: u64,
    ) -> Result<(), ContractError> {
        if !AdminManager::is_admin(&env, &caller) {
            EmergencyPause::require_not_paused(&env, PauseOperation::Fees)?;
        }
        FeeManager::cancel_fee_payment(&env, caller, project_id)
    }

    pub fn is_fee_paid(env: Env, project_id: u64) -> bool {
        FeeManager::is_fee_paid(&env, project_id)
    }

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

    pub fn get_fee_config_history(env: Env) -> Vec<FeeConfigHistoryEntry> {
        FeeManager::get_fee_config_history(&env)
    }

    /// Get fee payment details for a project (payer, amount, token, timestamp).
    pub fn get_fee_payment_details(env: Env, project_id: u64) -> Option<FeePaymentRecord> {
        FeeManager::get_fee_payment_details(&env, project_id)
    }

    /// Get registration fee payment details for an address.
    pub fn get_reg_fee_payment_details(env: Env, address: Address) -> Option<FeePaymentRecord> {
        FeeManager::get_registration_fee_payment_details(&env, &address)
    }

    // --- Fee Waivers ---

    /// Apply for a fee waiver (owner only).
    ///
    /// `proof_cid` must be an IPFS CID of a document proving the project
    /// qualifies for the requested `waiver_type` (GitHub repo, npm package,
    /// academic affiliation, nonprofit registration, etc.).
    /// If the CID is pre-approved by an admin the waiver is auto-granted.
    pub fn apply_for_fee_waiver(
        env: Env,
        project_id: u64,
        proof_cid: String,
        waiver_type: FeeWaiverType,
    ) -> Result<FeeWaiverRecord, ContractError> {
        EmergencyPause::require_not_paused(&env)?;
        FeeExtensions::apply_for_fee_waiver(&env, project_id, proof_cid, waiver_type)
    }

    /// Admin: approve or reject a pending fee waiver application.
    pub fn decide_fee_waiver(
        env: Env,
        admin: Address,
        project_id: u64,
        approve: bool,
    ) -> Result<FeeWaiverRecord, ContractError> {
        FeeExtensions::decide_fee_waiver(&env, admin, project_id, approve)
    }

    /// Admin: register an IPFS proof CID for automatic waiver granting.
    pub fn pre_approve_waiver_proof(
        env: Env,
        admin: Address,
        proof_cid: String,
        waiver_type: FeeWaiverType,
    ) -> Result<(), ContractError> {
        FeeExtensions::pre_approve_waiver_proof(&env, admin, proof_cid, waiver_type)
    }

    /// Read the fee waiver record for a project (any status).
    pub fn get_fee_waiver(env: Env, project_id: u64) -> Option<FeeWaiverRecord> {
        FeeExtensions::get_fee_waiver(&env, project_id)
    }

    /// Check whether a project has an active (approved) fee waiver.
    pub fn has_active_fee_waiver(env: Env, project_id: u64) -> bool {
        FeeExtensions::has_active_waiver(&env, project_id)
    }

    /// Admin dashboard: list all project IDs with a fee waiver record.
    pub fn list_waiver_project_ids(env: Env) -> Vec<u64> {
        FeeExtensions::list_waiver_project_ids(&env)
    }

    // --- Auto-Refunds ---

    /// Project owner: request a partial refund with a free-form reason.
    ///
    /// Creates (or accumulates) an `AutoRefundRecord` with
    /// `RefundTrigger::Manual`. The actual payout is done via
    /// `claim_auto_refund`.
    pub fn request_partial_refund(
        env: Env,
        project_id: u64,
        caller: Address,
        reason: String,
        amount: u128,
    ) -> Result<AutoRefundRecord, ContractError> {
        EmergencyPause::require_not_paused(&env)?;
        FeeExtensions::request_partial_refund(&env, project_id, caller, reason, amount)
    }

    /// Claim a pending auto-refund (payer or admin).
    ///
    /// `payout_amount` must be ≤ `remaining_amount` on the record.
    /// When fully settled, `settled_at` is stamped on the record.
    pub fn claim_auto_refund(
        env: Env,
        caller: Address,
        project_id: u64,
        payout_amount: u128,
    ) -> Result<(), ContractError> {
        FeeExtensions::claim_auto_refund(&env, caller, project_id, payout_amount)
    }

    /// Read the auto-refund record for a project.
    pub fn get_auto_refund(env: Env, project_id: u64) -> Option<AutoRefundRecord> {
        FeeExtensions::get_auto_refund(&env, project_id)
    }

    /// List project IDs that have pending (unsettled) auto-refunds.
    pub fn list_pending_auto_refund_ids(env: Env) -> Vec<u64> {
        FeeExtensions::list_pending_auto_refund_ids(&env)
    }

    // --- Fee Receipts ---

    /// Fetch a fee receipt by its unique `payment_id`.
    pub fn get_fee_receipt(env: Env, payment_id: u64) -> Option<FeeReceipt> {
        FeeExtensions::get_fee_receipt(&env, payment_id)
    }

    /// Admin: attach an IPFS CID and cryptographic signature to a receipt.
    ///
    /// `manager_signature` is a base64-encoded signature over `receipt_cid`
    /// produced by the fee manager's key. Indexers can verify it off-chain.
    pub fn sign_fee_receipt(
        env: Env,
        admin: Address,
        payment_id: u64,
        receipt_cid: String,
        manager_signature: String,
    ) -> Result<FeeReceipt, ContractError> {
        FeeExtensions::sign_fee_receipt(&env, admin, payment_id, receipt_cid, manager_signature)
    }

    /// List all receipt IDs for a project (verification fees).
    pub fn get_project_fee_receipt_ids(env: Env, project_id: u64) -> Vec<u64> {
        FeeExtensions::get_project_fee_receipt_ids(&env, project_id)
    }

    /// List all receipt IDs for a payer address (registration fees or any payment).
    pub fn get_address_fee_receipt_ids(env: Env, address: Address) -> Vec<u64> {
        FeeExtensions::get_address_fee_receipt_ids(&env, address)
    }

    /// Batch-export receipts by payment IDs (for tax/accounting).
    /// Missing IDs are silently skipped.
    pub fn batch_export_receipts(env: Env, payment_ids: Vec<u64>) -> Vec<FeeReceipt> {
        FeeExtensions::batch_export_receipts(&env, payment_ids)
    }

    // --- Fee Escrow / Dispute ---

    /// Project owner: dispute a pending fee payment, locking it in escrow.
    ///
    /// The fee must be in a paid-but-not-yet-consumed state. A 7-day
    /// lock window is set; the admin resolves the dispute via
    /// `resolve_fee_dispute`.
    pub fn dispute_fee(
        env: Env,
        project_id: u64,
        caller: Address,
        reason: String,
    ) -> Result<EscrowedFee, ContractError> {
        EmergencyPause::require_not_paused(&env)?;
        FeeExtensions::dispute_fee(&env, project_id, caller, reason)
    }

    /// Admin: resolve a fee dispute.
    ///
    /// - `FeeDisputeResolution::FullRefund` — full amount returned to payer.
    /// - `FeeDisputeResolution::FeeMaintained` — amount stays in treasury.
    /// - `FeeDisputeResolution::PartialRefund(n)` — `n` tokens refunded;
    ///   remainder stays in treasury.
    pub fn resolve_fee_dispute(
        env: Env,
        admin: Address,
        project_id: u64,
        resolution: FeeDisputeResolution,
    ) -> Result<EscrowedFee, ContractError> {
        FeeExtensions::resolve_fee_dispute(&env, admin, project_id, resolution)
    }

    /// Read the escrow record for a project.
    pub fn get_escrowed_fee(env: Env, project_id: u64) -> Option<EscrowedFee> {
        FeeExtensions::get_escrowed_fee(&env, project_id)
    }

    /// List project IDs with an active (unresolved) fee escrow.
    pub fn list_active_escrow_project_ids(env: Env) -> Vec<u64> {
        FeeExtensions::list_active_escrow_project_ids(&env)
    }

    // --- TTL Management ---

    /// Extend TTL for a specific project and its related data
    pub fn extend_project_ttl(env: Env, project_id: u64) {
        if let Some(project) = ProjectRegistry::get_project(&env, project_id) {
            StorageManager::extend_project_full_ttl(&env, project_id, &project.name);
        }
    }

    /// Extend TTL for many project IDs.
    ///
    /// ## Semantics (closes #666)
    ///
    /// - **Batch size guard**: returns `InvalidInput` immediately when the input
    ///   exceeds `MAX_TTL_BATCH_SIZE` — no work is done.
    /// - **Continue on missing**: a project ID that does not exist in storage is
    ///   recorded in `BatchTtlResult::skipped_ids`; processing continues for the
    ///   rest of the batch. This is *not* a failure — the caller can inspect
    ///   `skipped_ids` to see which IDs were not found.
    /// - **Fail-fast on hard errors**: if the underlying storage layer panics
    ///   (budget exhausted, ledger entry too large, etc.) the transaction is
    ///   aborted atomically. Partial-state is only possible across separate
    ///   invocations, never within a single call.
    ///
    /// Use `result.skipped_ids.len() == 0` to confirm all-or-nothing success.
    pub fn extend_projects_ttl(
        env: Env,
        project_ids: Vec<u64>,
    ) -> Result<BatchTtlResult, ContractError> {
        if project_ids.len() > crate::constants::MAX_TTL_BATCH_SIZE {
            return Err(ContractError::InvalidInput);
        }

        let mut refreshed = 0u32;
        let mut skipped_ids: Vec<u64> = Vec::new(&env);

        for i in 0..project_ids.len() {
            if let Some(project_id) = project_ids.get(i) {
                if let Some(project) = ProjectRegistry::get_project(&env, project_id) {
                    StorageManager::extend_project_full_ttl(&env, project_id, &project.name);
                    refreshed = refreshed.saturating_add(1);
                } else {
                    skipped_ids.push_back(project_id);
                }
            }
        }

        Ok(BatchTtlResult {
            refreshed,
            skipped_ids,
        })
    }

    /// Extend TTL for a specific review
    pub fn extend_review_ttl(env: Env, project_id: u64, reviewer: Address) {
        StorageManager::extend_review_ttl(&env, project_id, &reviewer);
    }

    /// Extend TTL for many review records.
    ///
    /// ## Semantics (closes #666)
    ///
    /// Same continue-on-missing / fail-fast rules as `extend_projects_ttl`.
    /// `BatchTtlResult::skipped_ids` contains the **project IDs** of reviews
    /// that could not be found (the reviewer index within the input slice is
    /// discarded because `Vec<u64>` cannot carry `Address` values).
    ///
    /// Use `result.skipped_ids.len() == 0` to confirm all-or-nothing success.
    pub fn extend_reviews_ttl(
        env: Env,
        review_ids: Vec<(u64, Address)>,
    ) -> Result<BatchTtlResult, ContractError> {
        if review_ids.len() > crate::constants::MAX_TTL_BATCH_SIZE {
            return Err(ContractError::InvalidInput);
        }

        let mut refreshed = 0u32;
        let mut skipped_ids: Vec<u64> = Vec::new(&env);

        for i in 0..review_ids.len() {
            if let Some((project_id, reviewer)) = review_ids.get(i) {
                if ReviewRegistry::get_review(&env, project_id, reviewer.clone()).is_some() {
                    StorageManager::extend_review_ttl(&env, project_id, &reviewer);
                    StorageManager::extend_project_reviews_ttl(&env, project_id);
                    StorageManager::extend_project_stats_ttl(&env, project_id);
                    StorageManager::extend_user_reviews_ttl(&env, &reviewer);
                    refreshed = refreshed.saturating_add(1);
                } else {
                    skipped_ids.push_back(project_id);
                }
            }
        }

        Ok(BatchTtlResult {
            refreshed,
            skipped_ids,
        })
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
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
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

    /// Report a project under a category and return the report ID. The
    /// report is auto-routed if the category has a routing rule - Issue #920
    pub fn report_project_with_category(
        env: Env,
        project_id: u64,
        reporter: Address,
        reason_cid: String,
        category: crate::types::ReportCategory,
    ) -> Result<u64, ContractError> {
        EmergencyPause::require_not_paused(&env)?;
        ReportRegistry::report_project_with_category(
            &env, project_id, reporter, reason_cid, category,
        )
    }

    /// Get a report by its global ID - Issue #920
    pub fn get_report(env: Env, report_id: u64) -> Option<ProjectReport> {
        ReportRegistry::get_report(&env, report_id)
    }

    /// Admin: route a report to the next moderator for a category - Issue #920
    pub fn route_report_to_moderator(
        env: Env,
        admin: Address,
        report_id: u64,
        category: crate::types::ReportCategory,
    ) -> Result<Address, ContractError> {
        ReportRegistry::route_report_to_moderator(&env, admin, report_id, category)
    }

    /// Admin: reassign several reports to one moderator - Issue #920
    pub fn bulk_reassign_reports(
        env: Env,
        admin: Address,
        report_ids: Vec<u64>,
        new_moderator: Address,
    ) -> Result<u32, ContractError> {
        ReportRegistry::bulk_reassign_reports(&env, admin, report_ids, new_moderator)
    }

    /// Admin: create or replace the routing rule for a category - Issue #920
    pub fn set_report_routing_rule(
        env: Env,
        admin: Address,
        category: crate::types::ReportCategory,
        moderators: Vec<Address>,
    ) -> Result<(), ContractError> {
        ReportRegistry::set_routing_rule(&env, admin, category, moderators)
    }

    /// Admin: remove the routing rule for a category - Issue #920
    pub fn remove_report_routing_rule(
        env: Env,
        admin: Address,
        category: crate::types::ReportCategory,
    ) -> Result<(), ContractError> {
        ReportRegistry::remove_routing_rule(&env, admin, category)
    }

    /// Get the routing rule for a category - Issue #920
    pub fn get_report_routing_rule(
        env: Env,
        category: crate::types::ReportCategory,
    ) -> Option<crate::types::ReportRoutingRule> {
        ReportRegistry::get_routing_rule(&env, category)
    }

    /// Get the current assignment for a report - Issue #920
    pub fn get_report_assignment(
        env: Env,
        report_id: u64,
    ) -> Option<crate::types::ReportAssignment> {
        ReportRegistry::get_report_assignment(&env, report_id)
    }

    /// Get report IDs assigned to a moderator - Issue #920
    pub fn get_moderator_report_assignments(env: Env, moderator: Address) -> Vec<u64> {
        ReportRegistry::get_moderator_assignments(&env, moderator)
    }

    /// Attach an evidence CID to a report (reporter, assigned moderator or
    /// admin; max 5 per report, append-only) - Issue #921
    pub fn attach_evidence_to_report(
        env: Env,
        report_id: u64,
        submitter: Address,
        evidence_cid: String,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env)?;
        ReportRegistry::attach_evidence_to_report(&env, report_id, submitter, evidence_cid)
    }

    /// Get the evidence CIDs attached to a report - Issue #921
    pub fn get_report_evidence(env: Env, report_id: u64) -> Vec<String> {
        ReportRegistry::get_report_evidence(&env, report_id)
    }

    /// Get the evidence attachment audit trail for a report - Issue #921
    pub fn get_report_evidence_audit(
        env: Env,
        report_id: u64,
    ) -> Vec<crate::types::ReportEvidenceAttachment> {
        ReportRegistry::get_report_evidence_audit(&env, report_id)
    }

    /// Record a moderation decision on a report (assigned moderator or
    /// admin) - Issue #922
    pub fn resolve_report(
        env: Env,
        moderator: Address,
        report_id: u64,
        decision: crate::types::ReportDecision,
        notes_cid: Option<String>,
    ) -> Result<(), ContractError> {
        crate::moderation_appeal_registry::ModerationAppealRegistry::resolve_report(
            &env, moderator, report_id, decision, notes_cid,
        )
    }

    /// Get the moderation decision for a report - Issue #922
    pub fn get_report_decision(
        env: Env,
        report_id: u64,
    ) -> Option<crate::types::ReportDecisionRecord> {
        crate::moderation_appeal_registry::ModerationAppealRegistry::get_report_decision(
            &env, report_id,
        )
    }

    /// Appeal a report's moderation decision (reporter or project owner).
    /// Routed to a senior moderator; returns the appeal ID - Issue #922
    pub fn appeal_moderation_decision(
        env: Env,
        appellant: Address,
        original_report_id: u64,
        appeal_reason: String,
    ) -> Result<u64, ContractError> {
        EmergencyPause::require_not_paused(&env)?;
        crate::moderation_appeal_registry::ModerationAppealRegistry::appeal_moderation_decision(
            &env,
            appellant,
            original_report_id,
            appeal_reason,
        )
    }

    /// Senior moderator: uphold or overturn an appeal - Issue #922
    pub fn resolve_moderation_appeal(
        env: Env,
        senior_moderator: Address,
        appeal_id: u64,
        overturn: bool,
        resolution: String,
    ) -> Result<(), ContractError> {
        crate::moderation_appeal_registry::ModerationAppealRegistry::resolve_moderation_appeal(
            &env,
            senior_moderator,
            appeal_id,
            overturn,
            resolution,
        )
    }

    /// Get a moderation appeal by ID - Issue #922
    pub fn get_moderation_appeal(env: Env, appeal_id: u64) -> Option<crate::types::AppealRecord> {
        crate::moderation_appeal_registry::ModerationAppealRegistry::get_appeal(&env, appeal_id)
    }

    /// Get the appeal ID filed against a report, if any - Issue #922
    pub fn get_report_appeal_id(env: Env, report_id: u64) -> Option<u64> {
        crate::moderation_appeal_registry::ModerationAppealRegistry::get_report_appeal_id(
            &env, report_id,
        )
    }

    /// Admin: designate an admin as a senior moderator - Issue #922
    pub fn add_senior_moderator(
        env: Env,
        admin: Address,
        moderator: Address,
    ) -> Result<(), ContractError> {
        crate::moderation_appeal_registry::ModerationAppealRegistry::add_senior_moderator(
            &env, admin, moderator,
        )
    }

    /// Admin: remove a senior moderator - Issue #922
    pub fn remove_senior_moderator(
        env: Env,
        admin: Address,
        moderator: Address,
    ) -> Result<(), ContractError> {
        crate::moderation_appeal_registry::ModerationAppealRegistry::remove_senior_moderator(
            &env, admin, moderator,
        )
    }

    /// List senior moderators - Issue #922
    pub fn get_senior_moderators(env: Env) -> Vec<Address> {
        crate::moderation_appeal_registry::ModerationAppealRegistry::get_senior_moderators(&env)
    }

    /// Pending appeal IDs assigned to a senior moderator - Issue #922
    pub fn get_senior_appeal_queue(env: Env, senior_moderator: Address) -> Vec<u64> {
        crate::moderation_appeal_registry::ModerationAppealRegistry::get_senior_appeal_queue(
            &env,
            senior_moderator,
        )
    }

    /// Appeal statistics for decisions made by a moderator - Issue #922
    pub fn get_moderator_appeal_stats(
        env: Env,
        moderator: Address,
    ) -> crate::types::AppealStats {
        crate::moderation_appeal_registry::ModerationAppealRegistry::get_moderator_appeal_stats(
            &env, moderator,
        )
    }

    /// Appeal statistics across all moderators - Issue #922
    pub fn get_global_appeal_stats(env: Env) -> crate::types::AppealStats {
        crate::moderation_appeal_registry::ModerationAppealRegistry::get_global_appeal_stats(&env)
    }

    /// List projects by tag - Issue #125
    /// Look projects up by one or more tags using the inverted index (issue #483).
    ///
    /// `list_projects_by_tag` scans every project id on every call. This serves
    /// the indexed range directly and scans only the range a backfill has not
    /// reached yet, so a single call can also cover several tags at once instead
    /// of one round trip per tag.
    pub fn get_projects_by_tag_batch(env: Env, tags: Vec<String>, limit: u32) -> Vec<Project> {
        ProjectRegistry::get_projects_by_tag_batch(&env, tags, limit)
    }

    /// Backfill the tag index for projects registered before it existed.
    /// Admin only. Processes at most `limit` ids and returns the new watermark.
    pub fn reindex_tags(env: Env, caller: Address, limit: u32) -> Result<u64, ContractError> {
        ProjectRegistry::reindex_tags(&env, caller, limit)
    }

    /// Highest project id guaranteed to be present in the tag index.
    pub fn get_tag_index_watermark(env: Env) -> u64 {
        ProjectRegistry::get_tag_index_watermark(&env)
    }

    pub fn list_projects_by_tag(
        env: Env,
        tag: String,
        start_index: u32,
        limit: u32,
    ) -> Vec<Project> {
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

    /// Admin: create a collection with explicit visibility.
    pub fn create_collection_vis(
        env: Env,
        admin: Address,
        name: String,
        description: String,
        is_public: bool,
    ) -> Result<u64, ContractError> {
        CollectionRegistry::create_collection_with_visibility(&env, admin, name, description, is_public)
    }

    /// User: create a collection owned by the caller.
    pub fn create_user_collection(
        env: Env,
        creator: Address,
        name: String,
        description: String,
        is_public: bool,
    ) -> Result<u64, ContractError> {
        CollectionRegistry::create_user_collection(&env, creator, name, description, is_public)
    }

    // --- Collection hierarchy & nesting (#905) ---

    /// Create a collection nested inside another collection (issue #905).
    ///
    /// The parent must exist and the hierarchy depth must stay within
    /// `MAX_COLLECTION_DEPTH` (5 levels).
    pub fn create_nested_collection(
        env: Env,
        creator: Address,
        name: String,
        description: String,
        is_public: bool,
        parent_collection_id: u64,
    ) -> Result<u64, ContractError> {
        CollectionRegistry::create_nested_collection(
            &env,
            creator,
            name,
            description,
            is_public,
            parent_collection_id,
        )
    }

    /// Move a collection under a new parent, or promote it to the top level
    /// by passing `None` (owner or admin only, issue #905).
    ///
    /// Rejects assignments that would create a cycle or exceed the maximum
    /// hierarchy depth.
    pub fn set_collection_parent(
        env: Env,
        caller: Address,
        collection_id: u64,
        parent_collection_id: Option<u64>,
    ) -> Result<(), ContractError> {
        CollectionRegistry::set_collection_parent(&env, caller, collection_id, parent_collection_id)
    }

    /// Return the subtree rooted at `root_id` (issue #905).
    ///
    /// The nodes are returned breadth-first; each node carries its depth and
    /// the identifiers of its direct children.
    pub fn get_collection_tree(
        env: Env,
        root_id: u64,
    ) -> Result<Vec<CollectionTreeNode>, ContractError> {
        CollectionRegistry::get_collection_tree(&env, root_id)
    }

    /// Return the breadcrumb path of a collection: root first, the requested
    /// collection last (issue #905).
    pub fn get_collection_path(
        env: Env,
        collection_id: u64,
    ) -> Result<Vec<Collection>, ContractError> {
        CollectionRegistry::get_collection_path(&env, collection_id)
    }

    /// Direct children of a collection (issue #905).
    pub fn get_collection_children(env: Env, collection_id: u64) -> Vec<u64> {
        CollectionRegistry::get_collection_children(&env, collection_id)
    }

    /// Depth of a collection in the hierarchy (`1` = top level, issue #905).
    pub fn get_collection_depth(env: Env, collection_id: u64) -> Result<u32, ContractError> {
        CollectionRegistry::get_collection_depth(&env, collection_id)
    }

    /// Toggle collection visibility between public and private (owner or admin only).
    pub fn toggle_collection_visibility(
        env: Env,
        caller: Address,
        collection_id: u64,
    ) -> Result<bool, ContractError> {
        CollectionRegistry::toggle_collection_visibility(&env, caller, collection_id)
    }

    /// Set explicit collection visibility (owner or admin only).
    pub fn set_collection_visibility(
        env: Env,
        caller: Address,
        collection_id: u64,
        is_public: bool,
    ) -> Result<(), ContractError> {
        CollectionRegistry::set_collection_visibility(&env, caller, collection_id, is_public)
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

    /// Get a collection by ID (returns Some only if public).
    pub fn get_collection(env: Env, collection_id: u64) -> Option<Collection> {
        CollectionRegistry::get_collection(&env, collection_id)
    }

    /// Get a collection with caller authorization check (owner can see private collection).
    pub fn get_collection_for_caller(
        env: Env,
        caller: Address,
        collection_id: u64,
    ) -> Result<Collection, ContractError> {
        CollectionRegistry::get_collection_for_caller(&env, caller, collection_id)
    }

    /// List all public collections with pagination.
    pub fn list_collections(env: Env, start_index: u32, limit: u32) -> Vec<Collection> {
        CollectionRegistry::list_collections(&env, start_index, limit)
    }

    /// List all public collections with pagination (explicit alias).
    pub fn list_public_collections(env: Env, start_index: u32, limit: u32) -> Vec<Collection> {
        CollectionRegistry::list_public_collections(&env, start_index, limit)
    }

    /// List all collections owned by a specific user (both public and private).
    pub fn list_user_collections(
        env: Env,
        owner: Address,
        start_index: u32,
        limit: u32,
    ) -> Result<Vec<Collection>, ContractError> {
        CollectionRegistry::list_user_collections(&env, owner, start_index, limit)
    }

    /// Generate a cryptographic share link for a collection (owner or admin only).
    pub fn generate_collection_share_link(
        env: Env,
        caller: Address,
        collection_id: u64,
    ) -> Result<String, ContractError> {
        CollectionRegistry::generate_collection_share_link(&env, caller, collection_id)
    }

    /// Retrieve a collection using a valid share token (read access even if private).
    pub fn get_collection_by_share_token(
        env: Env,
        collection_id: u64,
        share_token: String,
    ) -> Result<Collection, ContractError> {
        CollectionRegistry::get_collection_by_share_token(&env, collection_id, share_token)
    }

    /// Revoke the active share link for a collection (owner or admin only).
    pub fn revoke_collection_share_link(
        env: Env,
        caller: Address,
        collection_id: u64,
    ) -> Result<(), ContractError> {
        CollectionRegistry::revoke_collection_share_link(&env, caller, collection_id)
    }

    /// List project IDs in a public collection with pagination.
    pub fn list_collection_projects(
        env: Env,
        collection_id: u64,
        start_index: u32,
        limit: u32,
    ) -> Vec<u64> {
        CollectionRegistry::list_collection_projects(&env, collection_id, start_index, limit)
    }

    /// List project IDs in a collection with caller authorization.
    pub fn list_col_projects_for_caller(
        env: Env,
        caller: Address,
        collection_id: u64,
        start_index: u32,
        limit: u32,
    ) -> Result<Vec<u64>, ContractError> {
        CollectionRegistry::list_collection_projects_for_caller(&env, caller, collection_id, start_index, limit)
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
    pub fn list_admin_actions(env: Env, start_index: u32, limit: u32) -> Vec<AdminActionEntry> {
        AdminActionLog::list_admin_actions(&env, start_index, limit)
    }

    /// List admin action log entries filtered to a specific admin address (most recent first).
    ///
    /// Uses a per-admin index for efficiency — no full scan needed.
    /// `start` is a zero-based offset; `limit` is capped at `MAX_ADMIN_ACTION_LOG_PAGE`.
    pub fn get_admin_action_log_by_admin(
        env: Env,
        admin: Address,
        start_index: u32,
        limit: u32,
    ) -> Vec<AdminActionEntry> {
        AdminActionLog::get_admin_action_log_by_admin(&env, admin, start_index, limit)
    }

    /// Get the total number of admin action log entries.
    pub fn get_admin_action_log_count(env: Env) -> u64 {
        AdminActionLog::get_action_log_count(&env)
    }

    // --- Admin Activity Report & Compliance Audit ---

    /// Implement get_admin_activity_report(admin: Address, start: u64, end: u64) function
    /// with category breakdown and suspicious activity detection.
    pub fn get_admin_activity_report(
        env: Env,
        admin: Address,
        start: u64,
        end: u64,
    ) -> AdminActivityReport {
        AdminActionLog::get_admin_activity_report(&env, admin, start, end)
    }

    /// Support filtering by action type / category: verification, collection, pause, config.
    pub fn get_admin_activity_report_filtered(
        env: Env,
        admin: Address,
        start: u64,
        end: u64,
        category: AdminActionCategory,
    ) -> AdminActivityReport {
        AdminActionLog::get_admin_activity_report_filtered(&env, admin, start, end, category)
    }

    /// Generate audit reports in standard JSON format.
    pub fn generate_admin_audit_report_json(
        env: Env,
        admin: Address,
        start: u64,
        end: u64,
    ) -> String {
        AdminActionLog::generate_admin_audit_report_json(&env, admin, start, end)
    }

    /// Generate audit reports in standard CSV format.
    pub fn generate_admin_audit_report_csv(
        env: Env,
        admin: Address,
        start: u64,
        end: u64,
    ) -> String {
        AdminActionLog::generate_admin_audit_report_csv(&env, admin, start, end)
    }

    // --- Multi-Admin Sensitive Action Approval Workflow ---

    /// Configure approval workflow for sensitive action types (stages and quorum).
    pub fn set_approval_workflow(
        env: Env,
        admin: Address,
        workflow: ApprovalWorkflow,
    ) -> Result<(), ContractError> {
        GovernanceSecurityManager::set_approval_workflow(&env, admin, workflow)
    }

    /// Get configured approval workflow for a sensitive action type.
    pub fn get_approval_workflow(
        env: Env,
        action_type: SensitiveActionType,
    ) -> ApprovalWorkflow {
        GovernanceSecurityManager::get_approval_workflow(&env, action_type)
    }

    /// Propose a sensitive admin action requiring multiple approvals.
    pub fn propose_sensitive_action(
        env: Env,
        proposer: Address,
        action_type: SensitiveActionType,
        payload: SensitiveActionPayload,
    ) -> Result<u64, ContractError> {
        GovernanceSecurityManager::propose_sensitive_action(&env, proposer, action_type, payload)
    }

    /// Cast approval on a proposed sensitive action.
    pub fn approve_proposed_action(
        env: Env,
        admin: Address,
        proposal_id: u64,
    ) -> Result<(), ContractError> {
        GovernanceSecurityManager::approve_proposed_action(&env, admin, proposal_id)
    }

    /// Reject a proposed sensitive action.
    pub fn reject_proposed_action(
        env: Env,
        admin: Address,
        proposal_id: u64,
        reason: String,
    ) -> Result<(), ContractError> {
        GovernanceSecurityManager::reject_proposed_action(&env, admin, proposal_id, reason)
    }

    /// Execute an approved sensitive action.
    pub fn execute_proposed_action(
        env: Env,
        caller: Address,
        proposal_id: u64,
    ) -> Result<(), ContractError> {
        GovernanceSecurityManager::execute_proposed_action(&env, caller, proposal_id)
    }

    /// Get a sensitive action proposal by ID.
    pub fn get_sensitive_proposal(
        env: Env,
        proposal_id: u64,
    ) -> Option<SensitiveActionProposal> {
        GovernanceSecurityManager::get_sensitive_proposal(&env, proposal_id)
    }

    /// List sensitive action proposals with pagination.
    pub fn list_sensitive_proposals(
        env: Env,
        start_index: u32,
        limit: u32,
    ) -> Vec<SensitiveActionProposal> {
        GovernanceSecurityManager::list_sensitive_proposals(&env, start_index, limit)
    }

    // --- Emergency Override of Timelock Delays ---

    /// Configure emergency bypass threshold (supermajority or unanimous).
    pub fn set_emergency_bypass_threshold(
        env: Env,
        admin: Address,
        threshold: u32,
    ) -> Result<(), ContractError> {
        GovernanceSecurityManager::set_emergency_bypass_threshold(&env, admin, threshold)
    }

    /// Get the emergency bypass threshold.
    pub fn get_emergency_bypass_threshold(env: Env) -> u32 {
        GovernanceSecurityManager::get_emergency_bypass_threshold(&env)
    }

    /// Propose an emergency bypass of a timelock delay with justification CID.
    pub fn propose_emergency_bypass(
        env: Env,
        admin: Address,
        timelock_id: u64,
        justification_cid: String,
    ) -> Result<u64, ContractError> {
        GovernanceSecurityManager::propose_emergency_bypass(
            &env,
            admin,
            timelock_id,
            justification_cid,
        )
    }

    /// Approve an emergency bypass proposal.
    pub fn approve_emergency_bypass(
        env: Env,
        admin: Address,
        bypass_id: u64,
    ) -> Result<(), ContractError> {
        GovernanceSecurityManager::approve_emergency_bypass(&env, admin, bypass_id)
    }

    /// Execute an approved emergency bypass.
    pub fn execute_emergency_bypass(
        env: Env,
        admin: Address,
        bypass_id: u64,
    ) -> Result<(), ContractError> {
        GovernanceSecurityManager::execute_emergency_bypass(&env, admin, bypass_id)
    }

    /// Get an emergency bypass proposal by ID.
    pub fn get_emergency_bypass(
        env: Env,
        bypass_id: u64,
    ) -> Option<EmergencyBypassProposal> {
        GovernanceSecurityManager::get_emergency_bypass(&env, bypass_id)
    }

    /// List emergency bypass proposals with pagination.
    pub fn list_emergency_bypasses(
        env: Env,
        start_index: u32,
        limit: u32,
    ) -> Vec<EmergencyBypassProposal> {
        GovernanceSecurityManager::list_emergency_bypasses(&env, start_index, limit)
    }

    /// Check if a timelock action has been emergency bypassed.
    pub fn is_timelock_bypassed(env: Env, timelock_id: u64) -> bool {
        GovernanceSecurityManager::is_timelock_bypassed(&env, timelock_id)
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

    /// Versions of a project dependency that satisfy its version requirement,
    /// drawn from the dependency project's changelog - Issue #923
    pub fn get_compatible_versions(
        env: Env,
        project_id: u64,
        dependency_project_id: u64,
    ) -> Result<Vec<String>, ContractError> {
        crate::dependency_registry::DependencyRegistry::get_compatible_versions(
            &env,
            project_id,
            dependency_project_id,
        )
    }

    /// Parse and validate a dependency version spec such as "^1.2" - Issue #923
    pub fn parse_version_spec(
        env: Env,
        raw: String,
    ) -> Result<crate::types::VersionSpec, ContractError> {
        let _ = env;
        crate::version_spec::VersionSpecParser::parse(&raw)
    }

    /// Returns the number of dependencies for a project without fetching
    /// the full dependency list.  Useful for UI count badges.
    pub fn get_project_dependency_count(env: Env, project_id: u64) -> u32 {
        crate::dependency_registry::DependencyRegistry::get_dependency_count(&env, project_id)
    }

    /// Retrieve the full transitive dependency graph (project IDs only)
    pub fn get_transitive_dependencies(env: Env, project_id: u64) -> Vec<u64> {
        crate::dependency_registry::DependencyRegistry::get_transitive_dependencies(&env, project_id)
    }

    /// Structured dependency graph for indexers and frontends (issue #924).
    ///
    /// Returns typed `DependsOn` / `UsedBy` / `RelatedTo` edges reachable
    /// from `project_id`, walking transitive dependencies and dependents up
    /// to `max_depth` hops (clamped to `MAX_DEPENDENCY_GRAPH_DEPTH`; `0`
    /// selects the maximum). Cycles are reported in `has_cycle` /
    /// `cycle_nodes`.
    pub fn get_dependency_graph(
        env: Env,
        project_id: u64,
        max_depth: u32,
    ) -> Result<DependencyGraph, ContractError> {
        crate::dependency_registry::DependencyRegistry::get_dependency_graph(
            &env, project_id, max_depth,
        )
    }

    // --- Dependency Deprecation (#925) ---

    /// Admin: deprecate a project and alert its dependents via
    /// `DependencyDeprecated` events (first `MAX_DEPRECATION_NOTIFY_BATCH`
    /// dependents; page through the rest with `notify_deprecation_dependents`).
    pub fn deprecate_project(
        env: Env,
        admin: Address,
        project_id: u64,
        reason: String,
        migration_guide_cid: Option<String>,
    ) -> Result<DeprecationRecord, ContractError> {
        crate::deprecation_registry::DeprecationRegistry::deprecate_project(
            &env,
            admin,
            project_id,
            reason,
            migration_guide_cid,
        )
    }

    /// Admin: batch-emit `DependencyDeprecated` alerts for one page of a
    /// deprecated project's dependents. Returns the number of alerts sent.
    pub fn notify_deprecation_dependents(
        env: Env,
        admin: Address,
        deprecated_project_id: u64,
        start_index: u32,
        limit: u32,
    ) -> Result<u32, ContractError> {
        crate::deprecation_registry::DeprecationRegistry::notify_deprecation_dependents(
            &env,
            admin,
            deprecated_project_id,
            start_index,
            limit,
        )
    }

    /// Projects affected by a deprecation (direct dependents). Empty when
    /// the project is not deprecated.
    pub fn get_projects_depending_on(env: Env, deprecated_project_id: u64) -> Vec<u64> {
        crate::deprecation_registry::DeprecationRegistry::get_projects_depending_on(
            &env,
            deprecated_project_id,
        )
    }

    pub fn get_deprecation_record(env: Env, project_id: u64) -> Option<DeprecationRecord> {
        crate::deprecation_registry::DeprecationRegistry::get_deprecation_record(&env, project_id)
    }

    pub fn list_deprecated_projects(env: Env, start_index: u32, limit: u32) -> Vec<u64> {
        crate::deprecation_registry::DeprecationRegistry::list_deprecated_projects(
            &env,
            start_index,
            limit,
        )
    }

    /// Retrieve the reverse dependency graph (projects that depend on this project)
    pub fn get_dependent_projects(env: Env, project_id: u64) -> Vec<u64> {
        crate::dependency_registry::DependencyRegistry::get_dependent_projects(&env, project_id)
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

    /// Find projects similar to `project_id` exceeding similarity threshold (#913).
    /// `threshold` is provided in basis points (0..10_000) or percentage shorthand (1..100).
    pub fn find_similar_projects(
        env: Env,
        project_id: u64,
        threshold: u32,
    ) -> Vec<SimilarProject> {
        SimilarityRegistry::find_similar_projects(&env, project_id, threshold)
    }

    /// Open a duplicate dispute integrated with similarity verification (#913).
    pub fn open_duplicate_dispute_with_similarity(
        env: Env,
        project_id: u64,
        original_project_id: u64,
        creator: Address,
        evidence_cid: String,
        min_threshold: u32,
    ) -> Result<u64, ContractError> {
        EmergencyPause::require_not_paused(&env)?;
        SimilarityRegistry::open_duplicate_dispute_with_similarity(
            &env,
            project_id,
            original_project_id,
            creator,
            evidence_cid,
            min_threshold,
        )
    }


    // --- Project Changelog ---

    /// Add a new changelog entry for a project (owner only).
    ///
    /// # Arguments
    /// - `project_id`: The project ID to add changelog for
    /// - `owner`: The project owner (must be authenticated)
    /// - `cid`: IPFS CID containing the changelog content
    /// - `description`: Optional description/title for the changelog entry
    /// - `version`: Optional semver string for this release (e.g. "1.2.3")
    /// - `changelog_cid`: Optional secondary IPFS CID for a machine-readable release-notes document
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
        version: Option<String>,
        changelog_cid: Option<String>,
    ) -> Result<u64, ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        ChangelogRegistry::add_changelog_entry(
            &env,
            project_id,
            owner,
            cid,
            description,
            version,
            changelog_cid,
        )
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
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
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
        start_index: u32,
        limit: u32,
        sort_mode: ChangelogSortMode,
    ) -> Vec<ChangelogEntry> {
        ChangelogRegistry::get_project_changelog(&env, project_id, start_index, limit, sort_mode)
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

    pub fn follow_project(
        env: Env,
        project_id: u64,
        follower: Address,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
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
        start_index: u32,
        limit: u32,
    ) -> Vec<Address> {
        crate::subscription_registry::SubscriptionRegistry::get_project_followers(
            &env,
            project_id,
            start_index,
            limit,
        )
    }

    pub fn get_user_subscriptions(
        env: Env,
        user: Address,
        start_index: u32,
        limit: u32,
    ) -> Vec<u64> {
        crate::subscription_registry::SubscriptionRegistry::get_user_subscriptions(
            &env,
            user,
            start_index,
            limit,
        )
    }

    // --- Bulk Subscriptions (#932) ---

    /// Create up to `MAX_SUBSCRIPTION_BATCH_SIZE` subscriptions in one call.
    /// Each distinct subscriber must authorize. Duplicates and per-entry
    /// failures are reported in the result rather than aborting the batch.
    pub fn subscribe_to_events_batch(
        env: Env,
        subscriptions: Vec<crate::types::EventSubscription>,
    ) -> Result<crate::types::BatchSubscriptionResult, ContractError> {
        EmergencyPause::require_not_paused(&env)?;
        crate::subscription_bulk::SubscriptionBulk::subscribe_to_events_batch(&env, subscriptions)
    }

    /// Export `subscriber`'s subscriptions for a later `import_subscriptions`.
    pub fn export_subscriptions(env: Env, subscriber: Address) -> crate::types::SubscriptionExport {
        crate::subscription_bulk::SubscriptionBulk::export_subscriptions(&env, subscriber)
    }

    /// Re-create the subscriptions in `export` for `subscriber`.
    pub fn import_subscriptions(
        env: Env,
        subscriber: Address,
        export: crate::types::SubscriptionExport,
    ) -> Result<crate::types::BatchSubscriptionResult, ContractError> {
        EmergencyPause::require_not_paused(&env)?;
        crate::subscription_bulk::SubscriptionBulk::import_subscriptions(&env, subscriber, export)
    }

    /// Admin-only: subscribe `target` to every project `source` follows.
    pub fn clone_subscriptions_from_address(
        env: Env,
        admin: Address,
        source: Address,
        target: Address,
    ) -> Result<crate::types::BatchSubscriptionResult, ContractError> {
        EmergencyPause::require_not_paused(&env)?;
        crate::subscription_bulk::SubscriptionBulk::clone_subscriptions_from_address(
            &env, admin, source, target,
        )
    }

    // --- State Snapshots (#933) ---

    /// Export one page of contract state for audit or recovery (admin-only).
    /// `limit` project IDs from `start_project_id` (0 or over
    /// `MAX_SNAPSHOT_PAGE_SIZE` uses the maximum); `Configs` ignores the range.
    pub fn export_contract_state_snapshot(
        env: Env,
        admin: Address,
        scope: crate::types::SnapshotScope,
        start_project_id: u64,
        limit: u32,
    ) -> Result<crate::types::ContractStateSnapshot, ContractError> {
        crate::state_export::StateExport::export_contract_state_snapshot(
            &env,
            admin,
            scope,
            start_project_id,
            limit,
        )
    }

    /// Record that the snapshot for `scope` and page was published to IPFS
    /// at `ipfs_cid` (admin-only). Returns the new snapshot ID.
    pub fn record_state_snapshot(
        env: Env,
        admin: Address,
        scope: crate::types::SnapshotScope,
        start_project_id: u64,
        limit: u32,
        ipfs_cid: String,
        export_sha256: soroban_sdk::BytesN<32>,
    ) -> Result<u64, ContractError> {
        crate::state_export::StateExport::record_state_snapshot(
            &env,
            admin,
            scope,
            start_project_id,
            limit,
            ipfs_cid,
            export_sha256,
        )
    }

    pub fn get_state_snapshot(
        env: Env,
        snapshot_id: u64,
    ) -> Option<crate::types::StateSnapshotRecord> {
        crate::state_export::StateExport::get_state_snapshot(&env, snapshot_id)
    }

    pub fn get_state_snapshot_count(env: Env) -> u64 {
        crate::state_export::StateExport::get_state_snapshot_count(&env)
    }

    /// Check whether current state still matches a recorded snapshot.
    pub fn verify_state_snapshot(
        env: Env,
        snapshot_id: u64,
    ) -> Result<crate::types::SnapshotVerification, ContractError> {
        crate::state_export::StateExport::verify_state_snapshot(&env, snapshot_id)
    }

    // --- Analytics Export (#934) ---

    /// Export registry metrics for `[start, end]` (inclusive timestamps) as
    /// JSON or CSV. Scans the first `MAX_ANALYTICS_SCAN_SIZE` project IDs;
    /// use `export_analytics_data_filtered` for later pages and filters.
    pub fn export_analytics_data(
        env: Env,
        start: u64,
        end: u64,
        format: crate::types::ExportFormat,
    ) -> Result<crate::types::AnalyticsExport, ContractError> {
        crate::analytics_export::AnalyticsExportRegistry::export_analytics_data(
            &env,
            start,
            end,
            format,
            crate::analytics_export::AnalyticsExportRegistry::default_filter(),
        )
    }

    /// `export_analytics_data` with category / verification filters and
    /// project-ID paging.
    pub fn export_analytics_data_filtered(
        env: Env,
        start: u64,
        end: u64,
        format: crate::types::ExportFormat,
        filter: crate::types::AnalyticsFilter,
    ) -> Result<crate::types::AnalyticsExport, ContractError> {
        crate::analytics_export::AnalyticsExportRegistry::export_analytics_data(
            &env, start, end, format, filter,
        )
    }

    // --- Migration Export (#935) ---

    /// Package one page of state for migration to contract `version`
    /// (admin-only). Pages work as in `export_contract_state_snapshot`;
    /// follow `metadata.next_project_id` until it is `None`.
    pub fn export_for_migration(
        env: Env,
        admin: Address,
        version: String,
        start_project_id: u64,
        limit: u32,
    ) -> Result<crate::types::MigrationBundle, ContractError> {
        crate::migration_export::MigrationExport::export_for_migration(
            &env,
            admin,
            version,
            start_project_id,
            limit,
        )
    }

    /// Check a migration bundle's integrity and compatibility with
    /// `target_version`. Read-only.
    pub fn validate_migration_bundle(
        env: Env,
        bundle: crate::types::MigrationBundle,
        target_version: String,
    ) -> crate::types::MigrationCompatibilityReport {
        crate::migration_export::MigrationExport::validate_migration_bundle(
            &env,
            bundle,
            target_version,
        )
    }

    // --- Notification Preferences ---

    pub fn set_notification_prefs(
        env: Env,
        user: Address,
        opted_out: bool,
        notify_on_all: bool,
        digest_frequency: crate::types::DigestFrequency,
        kinds: soroban_sdk::Vec<crate::types::NotificationKind>,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        crate::notification_registry::NotificationRegistry::set_notification_prefs(
            &env,
            user,
            opted_out,
            notify_on_all,
            digest_frequency,
            kinds,
        )
    }

    pub fn get_notification_prefs(
        env: Env,
        user: Address,
    ) -> Option<crate::types::UserNotificationPrefs> {
        crate::notification_registry::NotificationRegistry::get_notification_prefs(&env, user)
    }

    pub fn set_project_notif_override(
        env: Env,
        user: Address,
        project_id: u64,
        opted_out: bool,
        kinds: Option<soroban_sdk::Vec<crate::types::NotificationKind>>,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        crate::notification_registry::NotificationRegistry::set_project_notification_override(
            &env, user, project_id, opted_out, kinds,
        )
    }

    pub fn get_project_notif_override(
        env: Env,
        user: Address,
        project_id: u64,
    ) -> Option<crate::types::ProjectNotificationOverride> {
        crate::notification_registry::NotificationRegistry::get_project_notification_override(
            &env, user, project_id,
        )
    }

    pub fn get_digest_queue(env: Env, user: Address, start_index: u32, limit: u32) -> Vec<u64> {
        crate::notification_registry::NotificationRegistry::get_digest_queue(
            &env,
            user,
            start_index,
            limit,
        )
    }

    pub fn flush_digest_queue(env: Env, user: Address) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        crate::notification_registry::NotificationRegistry::flush_digest_queue(&env, user)
    }

    // --- Bookmark Registry ---

    pub fn bookmark_project(env: Env, project_id: u64, user: Address) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        crate::bookmark_registry::BookmarkRegistry::bookmark_project(&env, project_id, user)
    }

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

    pub fn get_user_bookmarks(env: Env, user: Address, start_index: u32, limit: u32) -> Vec<u64> {
        crate::bookmark_registry::BookmarkRegistry::get_user_bookmarks(
            &env,
            user,
            start_index,
            limit,
        )
    }

    // --- Bookmark Folders (#815) ---

    /// Create a new folder for organising bookmarks.
    pub fn create_bookmark_folder(
        env: Env,
        user: Address,
        name: String,
        parent_id: Option<u64>,
    ) -> Result<u64, ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        crate::bookmark_registry::BookmarkRegistry::create_folder(&env, user, name, parent_id)
    }

    /// Delete a bookmark folder (bookmarks are retained in the flat list).
    pub fn delete_bookmark_folder(
        env: Env,
        user: Address,
        folder_id: u64,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        crate::bookmark_registry::BookmarkRegistry::delete_folder(&env, user, folder_id)
    }

    /// Rename a bookmark folder.
    pub fn rename_bookmark_folder(
        env: Env,
        user: Address,
        folder_id: u64,
        new_name: String,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        crate::bookmark_registry::BookmarkRegistry::rename_folder(&env, user, folder_id, new_name)
    }

    /// Get a bookmark folder by ID.
    pub fn get_bookmark_folder_by_id(
        env: Env,
        user: Address,
        folder_id: u64,
    ) -> Option<BookmarkFolder> {
        crate::bookmark_registry::BookmarkRegistry::get_folder(&env, user, folder_id)
    }

    /// List all folders owned by the user.
    pub fn list_bookmark_folders(env: Env, user: Address) -> Vec<BookmarkFolder> {
        crate::bookmark_registry::BookmarkRegistry::list_folders(&env, user)
    }

    /// List direct child folders of a given parent folder.
    pub fn list_child_bookmark_folders(
        env: Env,
        user: Address,
        parent_id: u64,
    ) -> Vec<BookmarkFolder> {
        crate::bookmark_registry::BookmarkRegistry::list_child_folders(&env, user, parent_id)
    }

    /// Move a bookmarked project into a folder.
    pub fn move_bookmark_to_folder(
        env: Env,
        user: Address,
        project_id: u64,
        folder_id: u64,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        crate::bookmark_registry::BookmarkRegistry::move_bookmark_to_folder(
            &env, user, project_id, folder_id,
        )
    }

    /// Remove a bookmark from its current folder.
    pub fn remove_bookmark_from_folder(
        env: Env,
        user: Address,
        project_id: u64,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        crate::bookmark_registry::BookmarkRegistry::remove_bookmark_from_folder(
            &env, user, project_id,
        )
    }

    /// Get bookmarks in a folder, paginated.
    pub fn get_folder_bookmarks(
        env: Env,
        user: Address,
        folder_id: u64,
        start_index: u32,
        limit: u32,
    ) -> Vec<u64> {
        crate::bookmark_registry::BookmarkRegistry::get_folder_bookmarks(
            &env,
            user,
            folder_id,
            start_index,
            limit,
        )
    }

    /// Get the folder ID that a bookmarked project currently lives in, if any.
    pub fn get_bookmark_folder(env: Env, user: Address, project_id: u64) -> Option<u64> {
        crate::bookmark_registry::BookmarkRegistry::get_bookmark_folder(&env, project_id, &user)
    }

    // --- Smart Folders (#815) ---

    /// Create a smart folder (dynamically filtered bookmark view).
    pub fn create_smart_folder(
        env: Env,
        user: Address,
        name: String,
        filter: SmartFolderFilter,
    ) -> Result<u64, ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        crate::bookmark_registry::BookmarkRegistry::create_smart_folder(&env, user, name, filter)
    }

    /// Delete a smart folder.
    pub fn delete_smart_folder(
        env: Env,
        user: Address,
        smart_folder_id: u64,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        crate::bookmark_registry::BookmarkRegistry::delete_smart_folder(&env, user, smart_folder_id)
    }

    /// Get a smart folder by ID.
    pub fn get_smart_folder(env: Env, user: Address, smart_folder_id: u64) -> Option<SmartFolder> {
        crate::bookmark_registry::BookmarkRegistry::get_smart_folder(&env, user, smart_folder_id)
    }

    /// List all smart folders owned by the user.
    pub fn list_smart_folders(env: Env, user: Address) -> Vec<SmartFolder> {
        crate::bookmark_registry::BookmarkRegistry::list_smart_folders(&env, user)
    }

    /// Get the resolved bookmark list for a smart folder (paginated).
    pub fn get_smart_folder_bookmarks(
        env: Env,
        user: Address,
        smart_folder_id: u64,
        start_index: u32,
        limit: u32,
    ) -> Result<Vec<u64>, ContractError> {
        crate::bookmark_registry::BookmarkRegistry::get_smart_folder_bookmarks(
            &env,
            user,
            smart_folder_id,
            start_index,
            limit,
        )
    }

    // --- Smart Bookmark Collections (#914) ---

    /// Create a smart bookmark collection with filtering rules (#914).
    pub fn create_smart_bookmark_collection(
        env: Env,
        user: Address,
        name: String,
        rules: SmartBookmarkRules,
    ) -> Result<u64, ContractError> {
        EmergencyPause::require_not_paused(&env)?;
        SmartBookmarkRegistry::create_smart_bookmark_collection(&env, user, name, rules)
    }

    /// Refresh a smart bookmark collection's auto-populated member projects (#914).
    pub fn refresh_smart_bookmark_collection(
        env: Env,
        collection_id: u64,
    ) -> Result<Vec<u64>, ContractError> {
        EmergencyPause::require_not_paused(&env)?;
        SmartBookmarkRegistry::refresh_smart_bookmark_collection(&env, collection_id)
    }

    /// Get a smart bookmark collection by ID (#914).
    pub fn get_smart_bookmark_collection(
        env: Env,
        collection_id: u64,
    ) -> Option<SmartBookmarkCollection> {
        SmartBookmarkRegistry::get_smart_bookmark_collection(&env, collection_id)
    }

    /// List all smart bookmark collections owned by the user (#914).
    pub fn list_smart_bookmark_collections(
        env: Env,
        user: Address,
    ) -> Vec<SmartBookmarkCollection> {
        SmartBookmarkRegistry::list_smart_bookmark_collections(&env, user)
    }

    /// Delete a smart bookmark collection (#914).
    pub fn delete_smart_bookmark_collection(
        env: Env,
        user: Address,
        collection_id: u64,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env)?;
        SmartBookmarkRegistry::delete_smart_bookmark_collection(&env, user, collection_id)
    }

    // --- Dynamic Trending Projects (#912) ---

    /// Get dynamically calculated trending projects for a specified time window (#912).
    /// `time_window` supports 86400 (24h), 604800 (7d), 2592000 (30d), and 0 (all-time).
    pub fn get_trending_projects(
        env: Env,
        time_window: u64,
        limit: u32,
    ) -> Vec<TrendingProject> {
        TrendingRegistry::get_trending_projects(&env, time_window, limit)
    }

    /// Recalculate trending projects cache (for background job / keeper) (#912).
    pub fn recalculate_trending_projects(
        env: Env,
        time_window: u64,
    ) -> Vec<TrendingProject> {
        TrendingRegistry::recalculate_trending(&env, time_window)
    }

    // --- Endorsement Registry ---


    pub fn endorse_project(env: Env, project_id: u64, user: Address) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        crate::endorsement_registry::EndorsementRegistry::endorse_project(&env, project_id, user)
    }

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

    pub fn get_project_endorsements(
        env: Env,
        project_id: u64,
        start_index: u32,
        limit: u32,
    ) -> Vec<Address> {
        crate::endorsement_registry::EndorsementRegistry::get_project_endorsements(
            &env,
            project_id,
            start_index,
            limit,
        )
    }

    pub fn has_endorsed(env: Env, project_id: u64, user: Address) -> bool {
        crate::endorsement_registry::EndorsementRegistry::has_endorsed(&env, project_id, &user)
    }

    // --- Conditional Endorsements (#918) ---

    /// Endorse a project for specific expertise areas, optionally expiring
    /// at `expires_at`.  Returns the endorsement ID.
    pub fn endorse_project_with_conditions(
        env: Env,
        project_id: u64,
        user: Address,
        areas: Vec<String>,
        expires_at: Option<u64>,
    ) -> Result<u64, ContractError> {
        EmergencyPause::require_not_paused(&env)?;
        crate::endorsement_registry::EndorsementRegistry::endorse_project_with_conditions(
            &env, project_id, user, areas, expires_at,
        )
    }

    /// Renew a time-limited endorsement.  Returns the new expiry timestamp.
    pub fn renew_endorsement(
        env: Env,
        endorser: Address,
        endorsement_id: u64,
    ) -> Result<u64, ContractError> {
        EmergencyPause::require_not_paused(&env)?;
        crate::endorsement_registry::EndorsementRegistry::renew_endorsement(
            &env,
            endorser,
            endorsement_id,
        )
    }

    pub fn get_endorsement(env: Env, endorsement_id: u64) -> Option<crate::types::Endorsement> {
        crate::endorsement_registry::EndorsementRegistry::get_endorsement(&env, endorsement_id)
    }

    pub fn get_user_project_endorsement(
        env: Env,
        project_id: u64,
        user: Address,
    ) -> Option<crate::types::Endorsement> {
        crate::endorsement_registry::EndorsementRegistry::get_user_project_endorsement(
            &env,
            project_id,
            &user,
        )
    }

    /// Whether `user` holds an unexpired endorsement on `project_id`.
    pub fn has_active_endorsement(env: Env, project_id: u64, user: Address) -> bool {
        crate::endorsement_registry::EndorsementRegistry::has_active_endorsement(
            &env,
            project_id,
            &user,
        )
    }

    /// Active endorsements of `project_id` covering `area`, paginated.
    pub fn get_endorsements_by_area(
        env: Env,
        project_id: u64,
        area: String,
        start_index: u32,
        limit: u32,
    ) -> Vec<crate::types::Endorsement> {
        crate::endorsement_registry::EndorsementRegistry::get_endorsements_by_area(
            &env,
            project_id,
            area,
            start_index,
            limit,
        )
    }

    /// Endorsers of `project_id` whose active endorsement covers all `areas`.
    pub fn filter_endorsers_by_areas(
        env: Env,
        project_id: u64,
        areas: Vec<String>,
    ) -> Vec<Address> {
        crate::endorsement_registry::EndorsementRegistry::filter_endorsers_by_areas(
            &env, project_id, areas,
        )
    }

    // --- Endorsement Delegation (#919) ---

    /// Delegate endorsement authority on `project_id` to `delegate`, limited
    /// to `scope` expertise areas (empty = the delegator's full scope).
    pub fn delegate_endorsement_authority(
        env: Env,
        project_id: u64,
        delegator: Address,
        delegate: Address,
        scope: Vec<String>,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env)?;
        crate::endorsement_delegation::EndorsementDelegationRegistry::delegate_endorsement_authority(
            &env, project_id, delegator, delegate, scope,
        )
    }

    /// Revoke previously delegated endorsement authority.
    pub fn revoke_endorsement_delegation(
        env: Env,
        project_id: u64,
        delegator: Address,
        delegate: Address,
    ) -> Result<(), ContractError> {
        crate::endorsement_delegation::EndorsementDelegationRegistry::revoke_endorsement_delegation(
            &env, project_id, delegator, delegate,
        )
    }

    /// Endorse a project using delegated authority.  Returns the endorsement ID.
    pub fn endorse_project_on_behalf(
        env: Env,
        project_id: u64,
        delegate: Address,
        areas: Vec<String>,
        expires_at: Option<u64>,
    ) -> Result<u64, ContractError> {
        EmergencyPause::require_not_paused(&env)?;
        crate::endorsement_delegation::EndorsementDelegationRegistry::endorse_project_on_behalf(
            &env, project_id, delegate, areas, expires_at,
        )
    }

    pub fn get_endorsement_delegation(
        env: Env,
        project_id: u64,
        delegate: Address,
    ) -> Option<crate::types::EndorsementDelegation> {
        crate::endorsement_delegation::EndorsementDelegationRegistry::get_delegation(
            &env, project_id, delegate,
        )
    }

    /// Delegates appointed by `delegator` for `project_id`.
    pub fn get_endorsement_delegates(env: Env, project_id: u64, delegator: Address) -> Vec<Address> {
        crate::endorsement_delegation::EndorsementDelegationRegistry::get_delegates(
            &env, project_id, delegator,
        )
    }

    /// Delegation chain from `address` up to its root endorser.
    pub fn get_endorsement_delegation_chain(
        env: Env,
        project_id: u64,
        address: Address,
    ) -> Vec<Address> {
        crate::endorsement_delegation::EndorsementDelegationRegistry::get_delegation_chain(
            &env, project_id, address,
        )
    }

    /// Whether `address` may endorse `project_id` directly or via delegation.
    pub fn has_endorsement_authority(env: Env, project_id: u64, address: Address) -> bool {
        crate::endorsement_delegation::EndorsementDelegationRegistry::has_endorsement_authority(
            &env, project_id, address,
        )
    }

    // --- Endorsement Graph (#917) ---

    /// `endorser` vouches for `endorsee` in the address endorsement graph.
    pub fn endorse_address(
        env: Env,
        endorser: Address,
        endorsee: Address,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env)?;
        crate::endorsement_graph::EndorsementGraph::endorse_address(&env, endorser, endorsee)
    }

    /// Remove an address endorsement.
    pub fn revoke_address_endorsement(
        env: Env,
        endorser: Address,
        endorsee: Address,
    ) -> Result<(), ContractError> {
        crate::endorsement_graph::EndorsementGraph::revoke_address_endorsement(
            &env, endorser, endorsee,
        )
    }

    pub fn has_endorsed_address(env: Env, endorser: Address, endorsee: Address) -> bool {
        crate::endorsement_graph::EndorsementGraph::has_endorsed_address(&env, &endorser, &endorsee)
    }

    /// Addresses directly endorsed by `user`.
    pub fn get_address_endorsements(env: Env, user: Address) -> Vec<Address> {
        crate::endorsement_graph::EndorsementGraph::get_address_endorsements(&env, user)
    }

    /// Addresses directly endorsing `user`.
    pub fn get_address_endorsers(env: Env, user: Address) -> Vec<Address> {
        crate::endorsement_graph::EndorsementGraph::get_address_endorsers(&env, user)
    }

    /// Reputation (direct endorser count) of `user`.
    pub fn get_endorsement_reputation(env: Env, user: Address) -> u32 {
        crate::endorsement_graph::EndorsementGraph::get_endorsement_reputation(&env, user)
    }

    /// Shortest transitive endorsement chain from `from` to `to`.
    pub fn get_endorsement_path(
        env: Env,
        from: Address,
        to: Address,
    ) -> Option<crate::types::EndorsementChain> {
        crate::endorsement_graph::EndorsementGraph::get_endorsement_path(&env, from, to)
    }

    /// Who can vouch for `target` within `max_depth` hops (0 = default cap).
    pub fn get_vouchers(
        env: Env,
        target: Address,
        max_depth: u32,
    ) -> Vec<crate::types::EndorsementVoucher> {
        crate::endorsement_graph::EndorsementGraph::get_vouchers(&env, target, max_depth)
    }

    /// Shortest endorsement cycle through `start`, if any.
    pub fn detect_endorsement_cycle(env: Env, start: Address) -> Option<Vec<Address>> {
        crate::endorsement_graph::EndorsementGraph::detect_endorsement_cycle(&env, start)
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

    pub fn list_scheduled_actions(env: Env, start_index: u32, limit: u32) -> Vec<TimelockAction> {
        TimelockManager::list_scheduled_actions(&env, start_index, limit)
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

    // ── Recommendation Registry (Issue #820) ──────────────────────────────

    /// Create a new recommendation. The `creator` address is always authenticated.
    pub fn create_recommendation(
        env: Env,
        creator: Address,
        target_project_id: u64,
        algorithm: RecommendationAlgorithm,
        reference_project_id: Option<u64>,
        audience: Option<Address>,
        score: Option<u64>,
        label: Option<String>,
    ) -> Result<u64, ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        crate::recommendation_registry::RecommendationRegistry::create_recommendation(
            &env,
            creator,
            target_project_id,
            algorithm,
            reference_project_id,
            audience,
            score,
            label,
        )
    }

    /// Look up a single recommendation by id.
    pub fn get_recommendation(env: Env, recommendation_id: u64) -> Option<Recommendation> {
        crate::recommendation_registry::RecommendationRegistry::get_recommendation(
            &env,
            recommendation_id,
        )
    }

    /// Total number of recommendations currently stored.
    pub fn get_recommendation_count(env: Env) -> u32 {
        crate::recommendation_registry::RecommendationRegistry::get_recommendation_count(&env)
    }

    /// Number of recommendations stored against a specific target project.
    pub fn get_project_recommendation_count(env: Env, target_project_id: u64) -> u32 {
        crate::recommendation_registry::RecommendationRegistry::get_recommendation_count_for_project(
            &env,
            target_project_id,
        )
    }

    /// List recommendations with pagination (oldest-first insertion order).
    pub fn list_recommendations(env: Env, start_index: u32, limit: u32) -> Vec<Recommendation> {
        crate::recommendation_registry::RecommendationRegistry::list_recommendations(
            &env,
            start_index,
            limit,
        )
    }

    /// List recommendations targeting a specific project.
    pub fn list_recommendations_for_project(
        env: Env,
        target_project_id: u64,
        start_index: u32,
        limit: u32,
    ) -> Vec<Recommendation> {
        crate::recommendation_registry::RecommendationRegistry::list_recommendations_for_project(
            &env,
            target_project_id,
            start_index,
            limit,
        )
    }

    /// Record that a recommendation was shown to `viewer` (impression / denominator
    /// for CTR). Idempotent per viewer: repeat calls do not double-count.
    pub fn record_recommendation_impression(
        env: Env,
        recommendation_id: u64,
        viewer: Address,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        crate::recommendation_registry::RecommendationRegistry::record_impression(
            &env,
            recommendation_id,
            viewer,
        )
    }

    /// Record that `viewer` clicked the recommendation (CTR numerator / click-through tracking).
    /// Requires `record_recommendation_impression` to have been called first for the same viewer.
    pub fn record_recommendation_click(
        env: Env,
        recommendation_id: u64,
        viewer: Address,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        crate::recommendation_registry::RecommendationRegistry::record_click(
            &env,
            recommendation_id,
            viewer,
        )
    }

    /// Generic engagement recorder for follow / bookmark / endorse / review
    /// downstream signals. Impressions and clicks have dedicated entry points and
    /// are not repeated here.
    pub fn record_recommendation_engagement(
        env: Env,
        recommendation_id: u64,
        user: Address,
        kind: RecommendationEngagementKind,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        crate::recommendation_registry::RecommendationRegistry::record_engagement(
            &env,
            recommendation_id,
            user,
            kind,
        )
    }

    /// Record a user's thumbs-up or thumbs-down recommendation feedback.
    /// Append-only: once a user has submitted feedback for a recommendation they
    /// cannot change it — keeps the on-chain audit trail honest for the
    /// recommendation-improvement loop.
    pub fn give_recommendation_feedback(
        env: Env,
        recommendation_id: u64,
        user: Address,
        helpful: bool,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        crate::recommendation_registry::RecommendationRegistry::give_feedback(
            &env,
            recommendation_id,
            user,
            helpful,
        )
    }

    /// Look up what feedback a specific user left (if any) on a recommendation.
    pub fn get_recommendation_user_feedback(
        env: Env,
        recommendation_id: u64,
        user: Address,
    ) -> Option<RecommendationFeedback> {
        crate::recommendation_registry::RecommendationRegistry::get_user_feedback(
            &env,
            recommendation_id,
            user,
        )
    }

    /// Produce the aggregated RecommendationAnalytics snapshot (CTR, helpful ratio,
    /// composite effectiveness score). Always emits `RecommendationAnalyticsSnapshotEvent`
    /// so indexers can consume the result without additional reads.
    pub fn get_recommendation_analytics(
        env: Env,
        recommendation_id: u64,
    ) -> Option<RecommendationAnalytics> {
        crate::recommendation_registry::RecommendationRegistry::get_analytics(
            &env,
            recommendation_id,
        )
    }

    /// Return recommendations ordered from highest effectiveness score to lowest.
    /// Provides the "improvement based on feedback" primitive: callers and future
    /// on-chain recommendation engines use this list to surface the recommendations
    /// that users actually find useful.
    pub fn list_recommendations_sorted(
        env: Env,
        start_index: u32,
        limit: u32,
    ) -> Vec<Recommendation> {
        crate::recommendation_registry::RecommendationRegistry::list_sorted_by_effectiveness(
            &env,
            start_index,
            limit,
        )
    }

    /// Record a project view used for bounded on-chain personalization.
    pub fn record_project_view(
        env: Env,
        viewer: Address,
        project_id: u64,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        crate::recommendation_registry::RecommendationRegistry::record_project_view(
            &env, viewer, project_id,
        )
    }

    /// Return a viewer's oldest-first, deduplicated project-view history.
    pub fn get_recommendation_view_history(
        env: Env,
        viewer: Address,
    ) -> Vec<ProjectView> {
        crate::recommendation_registry::RecommendationRegistry::get_view_history(&env, &viewer)
    }

    /// Generate up to `limit` personalized recommendations for a source project.
    pub fn recommend_projects(
        env: Env,
        viewer: Address,
        source_project_id: u64,
        limit: u32,
    ) -> Result<Vec<Recommendation>, ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        crate::recommendation_registry::RecommendationRegistry::recommend_projects(
            &env,
            viewer,
            source_project_id,
            limit,
        )
    }

    /// Configure deterministic A/B assignment between recommendation algorithms.
    pub fn set_recommendation_ab_config(
        env: Env,
        admin: Address,
        config: RecommendationABConfig,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        crate::recommendation_registry::RecommendationRegistry::set_ab_config(
            &env, admin, config,
        )
    }

    pub fn get_recommendation_ab_config(env: Env) -> RecommendationABConfig {
        crate::recommendation_registry::RecommendationRegistry::get_ab_config(&env)
    }

    /// Return the viewer's deterministic A/B assignment.
    pub fn get_recommendation_variant(
        env: Env,
        viewer: Address,
    ) -> RecommendationVariant {
        crate::recommendation_registry::RecommendationRegistry::get_variant(&env, viewer)
    }

    /// Record one unique downstream conversion for a previously impressed rec.
    pub fn record_recommendation_conversion(
        env: Env,
        recommendation_id: u64,
        user: Address,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        crate::recommendation_registry::RecommendationRegistry::record_conversion(
            &env,
            recommendation_id,
            user,
        )
    }

    /// Return aggregate funnel conversion metrics for one experiment arm.
    pub fn get_recommendation_ab_analytics(
        env: Env,
        variant: RecommendationVariant,
    ) -> RecommendationABAnalytics {
        crate::recommendation_registry::RecommendationRegistry::get_ab_analytics(&env, variant)
    }

    // ── Community Collections (Issue #821) ──────────────────────────────────

    /// Create a community collection. Any authenticated address can call this
    /// (unlike admin-only `Collection`).
    pub fn create_community_collection(
        env: Env,
        creator: Address,
        name: String,
        description: String,
        tags: Option<String>,
        approval_threshold: Option<u32>,
        disapproval_threshold: Option<u32>,
        creator_revenue_share_bps: Option<u32>,
        initial_curators: Option<Vec<Address>>,
    ) -> Result<u64, ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        crate::community_collection_registry::CommunityCollectionRegistry::create(
            &env,
            creator,
            name,
            description,
            tags,
            approval_threshold,
            disapproval_threshold,
            creator_revenue_share_bps,
            initial_curators,
        )
    }

    /// Admin-only: create a built-in template collection that users can clone
    /// (AC4 — templates for common collections).
    pub fn create_ccol_template(
        env: Env,
        admin: Address,
        template_id: CommunityCollectionTemplateId,
        override_name: Option<String>,
        override_description: Option<String>,
        seed_projects: Option<Vec<u64>>,
    ) -> Result<u64, ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        crate::community_collection_registry::CommunityCollectionRegistry::create_template(
            &env,
            admin,
            template_id,
            override_name,
            override_description,
            seed_projects,
        )
    }

    /// Clone any community collection (typically a template collection with
    /// `is_template = true`) into a brand-new non-template one owned by the
    /// caller. Copies metadata and project set skeleton (AC4).
    pub fn create_ccol_from_template(
        env: Env,
        caller: Address,
        source_collection_id: u64,
        name: String,
        description: String,
        tags: Option<String>,
        approval_threshold: Option<u32>,
        disapproval_threshold: Option<u32>,
        creator_revenue_share_bps: Option<u32>,
        initial_curators: Option<Vec<Address>>,
    ) -> Result<u64, ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        crate::community_collection_registry::CommunityCollectionRegistry::create_from_template(
            &env,
            caller,
            source_collection_id,
            name,
            description,
            tags,
            approval_threshold,
            disapproval_threshold,
            creator_revenue_share_bps,
            initial_curators,
        )
    }

    pub fn get_community_collection(env: Env, id: u64) -> Option<CommunityCollection> {
        crate::community_collection_registry::CommunityCollectionRegistry::get(&env, id)
    }

    pub fn get_community_collection_count(env: Env) -> u32 {
        crate::community_collection_registry::CommunityCollectionRegistry::get_count(&env)
    }

    pub fn list_community_collections(
        env: Env,
        start_index: u32,
        limit: u32,
    ) -> Vec<CommunityCollection> {
        crate::community_collection_registry::CommunityCollectionRegistry::list(
            &env,
            start_index,
            limit,
        )
    }

    pub fn list_ccols_by_creator(
        env: Env,
        creator: Address,
        start_index: u32,
        limit: u32,
    ) -> Vec<CommunityCollection> {
        crate::community_collection_registry::CommunityCollectionRegistry::list_by_creator(
            &env,
            creator,
            start_index,
            limit,
        )
    }

    pub fn list_ccols_by_curator(
        env: Env,
        curator: Address,
        start_index: u32,
        limit: u32,
    ) -> Vec<CommunityCollection> {
        crate::community_collection_registry::CommunityCollectionRegistry::list_by_curator(
            &env,
            curator,
            start_index,
            limit,
        )
    }

    pub fn get_ccol_project_count(env: Env, id: u64) -> u32 {
        crate::community_collection_registry::CommunityCollectionRegistry::get_project_count(
            &env, id,
        )
    }

    pub fn list_ccol_projects(env: Env, id: u64, start_index: u32, limit: u32) -> Vec<u64> {
        crate::community_collection_registry::CommunityCollectionRegistry::list_projects(
            &env,
            id,
            start_index,
            limit,
        )
    }

    pub fn update_ccol_metadata(
        env: Env,
        id: u64,
        updater: Address,
        name: String,
        description: String,
        tags: Option<String>,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        crate::community_collection_registry::CommunityCollectionRegistry::update_metadata(
            &env,
            id,
            updater,
            name,
            description,
            tags,
        )
    }

    pub fn set_ccol_thresholds(
        env: Env,
        id: u64,
        updater: Address,
        approval_threshold: u32,
        disapproval_threshold: u32,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        crate::community_collection_registry::CommunityCollectionRegistry::set_thresholds(
            &env,
            id,
            updater,
            approval_threshold,
            disapproval_threshold,
        )
    }

    pub fn set_ccol_revenue_share(
        env: Env,
        id: u64,
        updater: Address,
        creator_revenue_share_bps: u32,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        crate::community_collection_registry::CommunityCollectionRegistry::set_revenue_share(
            &env,
            id,
            updater,
            creator_revenue_share_bps,
        )
    }

    pub fn add_community_collection_curator(
        env: Env,
        id: u64,
        actor: Address,
        new_curator: Address,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        crate::community_collection_registry::CommunityCollectionRegistry::add_curator(
            &env,
            id,
            actor,
            new_curator,
        )
    }

    pub fn remove_ccol_curator(
        env: Env,
        id: u64,
        actor: Address,
        curator_to_remove: Address,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        crate::community_collection_registry::CommunityCollectionRegistry::remove_curator(
            &env,
            id,
            actor,
            curator_to_remove,
        )
    }

    pub fn curator_add_ccol_project(
        env: Env,
        id: u64,
        curator: Address,
        project_id: u64,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        crate::community_collection_registry::CommunityCollectionRegistry::curator_add_project(
            &env, id, curator, project_id,
        )
    }

    pub fn curator_remove_ccol_project(
        env: Env,
        id: u64,
        curator: Address,
        project_id: u64,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        crate::community_collection_registry::CommunityCollectionRegistry::curator_remove_project(
            &env, id, curator, project_id,
        )
    }

    // AC2 — community voting / curation mechanism
    pub fn cast_community_collection_vote(
        env: Env,
        id: u64,
        voter: Address,
        project_id: u64,
        approve: bool,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        crate::community_collection_registry::CommunityCollectionRegistry::cast_vote(
            &env, id, voter, project_id, approve,
        )
    }

    pub fn get_community_collection_vote(
        env: Env,
        id: u64,
        project_id: u64,
        voter: Address,
    ) -> Option<CommunityCollectionVote> {
        crate::community_collection_registry::CommunityCollectionRegistry::get_vote(
            &env, id, project_id, voter,
        )
    }

    pub fn get_ccol_inclusion_status(
        env: Env,
        id: u64,
        project_id: u64,
    ) -> CommunityColInclusionStatus {
        crate::community_collection_registry::CommunityCollectionRegistry::get_inclusion_status(
            &env, id, project_id,
        )
    }

    // AC1 — featured community collections (admin-only, FIFO eviction at cap)
    pub fn feature_community_collection(
        env: Env,
        admin: Address,
        id: u64,
        featured: bool,
    ) -> Result<(), ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        crate::community_collection_registry::CommunityCollectionRegistry::set_featured(
            &env, admin, id, featured,
        )
    }

    pub fn list_featured_ccols(env: Env, start_index: u32, limit: u32) -> Vec<CommunityCollection> {
        crate::community_collection_registry::CommunityCollectionRegistry::list_featured(
            &env,
            start_index,
            limit,
        )
    }

    pub fn get_featured_ccol_count(env: Env) -> u32 {
        crate::community_collection_registry::CommunityCollectionRegistry::get_featured_count(&env)
    }

    // AC3 — revenue sharing (recording cumulative attribution for off-chain payout)
    pub fn attribute_ccol_revenue(
        env: Env,
        caller: Address,
        id: u64,
        total_amount_scaled: u128,
    ) -> Result<CommunityColRevenueSnapshot, ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        crate::community_collection_registry::CommunityCollectionRegistry::attribute_revenue(
            &env,
            caller,
            id,
            total_amount_scaled,
        )
    }

    pub fn get_ccol_revenue_snapshot(env: Env, id: u64) -> Option<CommunityColRevenueSnapshot> {
        crate::community_collection_registry::CommunityCollectionRegistry::get_revenue_snapshot(
            &env, id,
        )
    }

    // ── Social Analytics (Issue #822) ──────────────────────────────────────

    // AC1 — Growth metrics over time: daily checkpoint snapshots that
    // accumulate and support any-window delta computations.

    /// Record today's social-signal snapshot (followers, endorsements,
    /// bookmarks, reviews, rating) for the project. Idempotent per-day;
    /// repeated calls on the same day overwrite the prior snapshot of the
    /// day with the latest counts. When the rolling 730-day window is full,
    /// the oldest day's checkpoint is FIFO-evicted. Any authenticated caller
    /// may checkpoint (keepers / indexers are the expected operators).
    pub fn record_social_daily_checkpoint(
        env: Env,
        caller: Address,
        project_id: u64,
    ) -> Result<u32, ContractError> {
        EmergencyPause::require_not_paused(&env, PauseOperation::All)?;
        crate::social_analytics_registry::SocialAnalyticsRegistry::record_daily_checkpoint(
            &env, caller, project_id,
        )
    }

    /// Retrieve a single checkpoint by project + day (if persisted).
    pub fn get_project_social_checkpoint(
        env: Env,
        project_id: u64,
        day_index: u32,
    ) -> Option<ProjectSocialDailyCheckpoint> {
        crate::social_analytics_registry::SocialAnalyticsRegistry::get_checkpoint(
            &env, project_id, day_index,
        )
    }

    /// List stored checkpoints for a project in oldest-first order, paginated.
    pub fn list_project_social_checkpoints(
        env: Env,
        project_id: u64,
        start_index: u32,
        limit: u32,
    ) -> Vec<ProjectSocialDailyCheckpoint> {
        crate::social_analytics_registry::SocialAnalyticsRegistry::list_checkpoints(
            &env,
            project_id,
            start_index,
            limit,
        )
    }

    /// How many daily checkpoints currently exist for a project.
    pub fn get_social_checkpoint_count(env: Env, project_id: u64) -> u32 {
        crate::social_analytics_registry::SocialAnalyticsRegistry::get_checkpoint_count(
            &env, project_id,
        )
    }

    /// Return (oldest_day, newest_day) of stored checkpoints for a project,
    /// both optional (None if no checkpoints yet written). Fast metadata
    /// without scanning the whole list.
    pub fn get_social_checkpoint_bounds(env: Env, project_id: u64) -> (Option<u32>, Option<u32>) {
        crate::social_analytics_registry::SocialAnalyticsRegistry::get_oldest_newest_checkpoint_days(
            &env, project_id,
        )
    }

    // AC2 — Engagement rate calculations over arbitrary windows.

    /// Engagement metric (ppm rate, gains by signal, rating delta) for a
    /// custom window [window_start_day, window_end_day] inclusive.
    pub fn compute_engagement_metric(
        env: Env,
        project_id: u64,
        window_start_day: u32,
        window_end_day: u32,
    ) -> Result<ProjectEngagementMetric, ContractError> {
        crate::social_analytics_registry::SocialAnalyticsRegistry::compute_project_engagement_metric(
            &env,
            project_id,
            window_start_day,
            window_end_day,
        )
    }

    // AC3 — Peer comparison.

    /// Last-30-day engagement-rate comparison of the target project with
    /// other projects in the same category, sorted by rate descending. The
    /// target project is always included (so percentile is pos/len on the
    /// client side), and up to `max_peers` additional projects are returned
    /// (cap = SOCIAL_ANALYTICS_MAX_PEERS = 50).
    pub fn compare_social_peers(
        env: Env,
        project_id: u64,
        max_peers: u32,
    ) -> Result<Vec<ProjectSocialPeerRow>, ContractError> {
        crate::social_analytics_registry::SocialAnalyticsRegistry::compare_similar_projects(
            &env, project_id, max_peers,
        )
    }

    // AC4 — Export analytics report: one-shot payload combining everything.

    /// Compute and return a `ProjectSocialAnalyticsExport` payload combining
    /// checkpoints horizon, last-7d + last-30d engagement metrics, 30-day
    /// growth deltas, peer comparison ranking, and a monotonically
    /// incrementing report nonce so export consumers can dedupe repeated
    /// runs. Also emits `ProjectSocialAnalyticsExportEvent` with the key
    /// numbers for indexer consumption.
    pub fn export_social_analytics_report(
        env: Env,
        project_id: u64,
    ) -> Result<ProjectSocialAnalyticsExport, ContractError> {
        crate::social_analytics_registry::SocialAnalyticsRegistry::export_report(&env, project_id)
    }

    /// Last-generated export report nonce for a project. 0 if `export_*` has
    /// never been called.
    pub fn get_social_report_nonce(env: Env, project_id: u64) -> u64 {
        crate::social_analytics_registry::SocialAnalyticsRegistry::get_export_report_nonce(
            &env, project_id,
        )
    }

    // ── Verification Performance Metrics (#perf) ──────────────────────────────

    /// Return the global performance snapshot for a given month.
    ///
    /// `month_num` is the compact `YYYYMM` integer, e.g. `202609` for
    /// September 2026. Returns a zeroed-default when no data has been
    /// recorded for that month.
    pub fn get_verification_performance_snapshot(
        env: Env,
        month_num: u32,
    ) -> VerificationPerformanceSnapshot {
        crate::performance_metrics::PerformanceMetrics::get_global_snapshot(&env, month_num)
    }

    /// Return per-admin performance for `admin` in the given month.
    ///
    /// `month_num` uses the same `YYYYMM` encoding as
    /// `get_verification_performance_snapshot`.
    pub fn get_admin_verification_performance(
        env: Env,
        admin: Address,
        month_num: u32,
    ) -> AdminVerificationPerformance {
        crate::performance_metrics::PerformanceMetrics::get_admin_performance(&env, admin, month_num)
    }

    /// Return trend data across all tracked months (oldest first, up to 24).
    ///
    /// Each `VerificationTrendPoint` covers one calendar month and includes
    /// request volume, approval / rejection counts, appeal rate (bps),
    /// reversal rate (bps), and average approval time.
    pub fn get_verification_trend(env: Env) -> Vec<VerificationTrendPoint> {
        crate::performance_metrics::PerformanceMetrics::get_trend(&env)
    }

    /// Return a complete monthly performance report for the given month.
    ///
    /// The report includes:
    /// - Global aggregate snapshot
    /// - Per-admin summaries for every admin active that month
    /// - Full 24-month trend series
    ///
    /// Useful for generating the monthly PDF / dashboard report required
    /// by the acceptance criteria.
    pub fn get_monthly_verification_report(
        env: Env,
        month_num: u32,
    ) -> VerificationPerformanceReport {
        crate::performance_metrics::PerformanceMetrics::get_monthly_report(&env, month_num)
    }

    /// Return the ordered list of month numbers that have recorded data
    /// (oldest first, up to 24 entries).
    ///
    /// Callers can use this to discover which months to query for reports
    /// without guessing.
    pub fn get_tracked_performance_months(env: Env) -> Vec<u32> {
        crate::performance_metrics::PerformanceMetrics::get_tracked_months(&env)
    }

    // ── #940: Runtime Invariant Checks ────────────────────────────────────

    /// Admin-only debug endpoint: run enabled invariant checks
    /// (project count, review counts, admin count, fee balance),
    /// log violations, and return them.
    pub fn verify_contract_invariants(
        env: Env,
        admin: Address,
    ) -> Result<Vec<crate::invariant_checker::InvariantViolation>, ContractError> {
        crate::invariant_checker::InvariantChecker::verify_contract_invariants(&env, admin)
    }

    /// Admin-only: enable or disable a single invariant check.
    pub fn set_invariant_check_enabled(
        env: Env,
        admin: Address,
        check: crate::invariant_checker::InvariantCheckId,
        enabled: bool,
    ) -> Result<(), ContractError> {
        crate::invariant_checker::InvariantChecker::set_check_enabled(&env, admin, check, enabled)
    }

    /// Return all logged invariant violations (oldest first).
    pub fn get_invariant_violations(
        env: Env,
    ) -> Vec<crate::invariant_checker::InvariantViolation> {
        crate::invariant_checker::InvariantChecker::get_violations(&env)
    }

    // ── #946: Proof-of-Work for Expensive Operations ──────────────────────

    /// Issue a PoW challenge scaled to the operation cost (e.g. batch size).
    /// See `pow_challenge` docs for the algorithm and difficulty levels.
    pub fn issue_pow_challenge(env: Env, op_cost: u32) -> crate::pow_challenge::PowChallenge {
        crate::pow_challenge::PowGate::issue(&env, op_cost)
    }

    /// Difficulty (leading-zero bits) required for the given operation cost.
    pub fn pow_difficulty_for(op_cost: u32) -> u32 {
        crate::pow_challenge::PowGate::difficulty_for(op_cost)
    }

    /// Validate a PoW solution before running a resource-heavy function.
    pub fn verify_pow(
        env: Env,
        challenge: crate::pow_challenge::PowChallenge,
        nonce: u64,
        op_cost: u32,
    ) -> Result<(), ContractError> {
        crate::pow_challenge::PowGate::verify(&env, &challenge, nonce, op_cost)
    }

    // ── #947: Restricted-Mode Compliance Deployments ─────────────────────

    /// Admin-only: set compliance mode config (restricted features, geo/KYC).
    pub fn set_compliance_config(
        env: Env,
        admin: Address,
        config: crate::compliance_mode::ComplianceConfig,
    ) -> Result<(), ContractError> {
        crate::compliance_mode::ComplianceMode::set_config(&env, admin, config)
    }

    /// Return the current compliance mode config.
    pub fn get_compliance_config(env: Env) -> crate::compliance_mode::ComplianceConfig {
        crate::compliance_mode::ComplianceMode::get_config(&env)
    }

    /// Set the geographic country code for a project (compliance geo rule).
    pub fn set_project_geo(
        env: Env,
        caller: Address,
        project_id: u64,
        country_code: String,
    ) -> Result<(), ContractError> {
        crate::compliance_mode::ComplianceMode::set_project_geo(&env, caller, project_id, country_code)
    }

    /// Admin-only KYC/AML hook: approve or revoke a user.
    pub fn set_kyc_approved(
        env: Env,
        admin: Address,
        user: Address,
        approved: bool,
    ) -> Result<(), ContractError> {
        crate::compliance_mode::ComplianceMode::set_kyc_approved(&env, admin, user, approved)
    }

    /// Return the compliance audit log.
    pub fn get_compliance_audit_log(
        env: Env,
    ) -> Vec<crate::compliance_mode::ComplianceAuditEntry> {
        crate::compliance_mode::ComplianceMode::get_audit_log(&env)
    }

    // ── #941: Benchmark Suite & Regression Alerts ────────────────────────

    /// Record a benchmark sample for a critical function (admin-only).
    pub fn record_benchmark(
        env: Env,
        caller: Address,
        name: String,
        storage_reads: u32,
        exec_units: u64,
    ) -> Result<(), ContractError> {
        crate::benchmark_suite::BenchmarkSuite::record(&env, caller, name, storage_reads, exec_units)
    }

    /// Admin-only: pin the baseline for a benchmark.
    pub fn set_benchmark_baseline(
        env: Env,
        admin: Address,
        name: String,
        storage_reads: u32,
        exec_units: u64,
    ) -> Result<(), ContractError> {
        crate::benchmark_suite::BenchmarkSuite::set_baseline(&env, admin, name, storage_reads, exec_units)
    }

    /// Compare latest samples vs baselines; returns regressions over 10%.
    pub fn check_benchmark_regressions(
        env: Env,
    ) -> Vec<crate::benchmark_suite::BenchmarkRegression> {
        crate::benchmark_suite::BenchmarkSuite::check_regressions(&env)
    }

    // ── #757: Security Contact Email Verification ─────────────────────────

    /// Initiate a challenge-response verification for a project's security
    /// contact. Emits a `SC_CHALL` event containing the one-time token.
    /// Caller must be the project owner.
    pub fn initiate_security_contact_verification(
        env: Env,
        project_id: u64,
        caller: Address,
    ) -> Result<SecurityContactVerificationRecord, ContractError> {
        SecurityContactVerificationRegistry::initiate_verification(&env, project_id, &caller)
    }

    /// Confirm receipt of the challenge token to mark the security contact as
    /// verified. Caller must be the project owner.
    pub fn confirm_security_contact_verification(
        env: Env,
        project_id: u64,
        caller: Address,
        token: String,
    ) -> Result<SecurityContactVerificationRecord, ContractError> {
        SecurityContactVerificationRegistry::confirm_verification(&env, project_id, &caller, token)
    }

    /// Return the current security contact verification status for a project.
    pub fn get_security_contact_verification_status(
        env: Env,
        project_id: u64,
    ) -> SecurityContactVerificationStatus {
        SecurityContactVerificationRegistry::get_status(&env, project_id)
    }

    /// Admin: revoke a security contact verification to force re-verification.
    pub fn admin_revoke_security_contact_verification(
        env: Env,
        project_id: u64,
        admin: Address,
    ) -> Result<(), ContractError> {
        SecurityContactVerificationRegistry::admin_revoke(&env, project_id, &admin)
    }

    /// Check whether annual re-verification is required for a project's
    /// security contact.
    pub fn security_contact_requires_reverification(env: Env, project_id: u64) -> bool {
        SecurityContactVerificationRegistry::requires_reverification(&env, project_id)
    }

    // ── #756: Project Health Score ────────────────────────────────────────

    /// Return (and lazily compute) the current health score for a project.
    pub fn get_project_health_score(
        env: Env,
        project_id: u64,
    ) -> Result<ProjectHealthScore, ContractError> {
        HealthScoreRegistry::get_health_score(&env, project_id)
    }

    /// Return paginated historical health score snapshots for a project
    /// (oldest-first, up to 100 per page).
    pub fn get_project_health_history(
        env: Env,
        project_id: u64,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<HealthScoreSnapshot>, ContractError> {
        HealthScoreRegistry::get_health_history(&env, project_id, offset, limit)
    }

    /// Admin: recompute and store the health score for a project immediately.
    pub fn refresh_project_health_score(
        env: Env,
        project_id: u64,
        caller: Address,
    ) -> Result<ProjectHealthScore, ContractError> {
        HealthScoreRegistry::refresh_health_score(&env, project_id, &caller)
    }

    /// Admin: configure the health score weights and update frequency.
    /// `rating_weight + activity_weight + verification_weight + engagement_weight` must equal 100.
    pub fn set_health_score_config(
        env: Env,
        admin: Address,
        config: HealthScoreConfig,
    ) -> Result<(), ContractError> {
        HealthScoreRegistry::set_config(&env, &admin, config)
    }

    /// Return the current health score configuration.
    pub fn get_health_score_config(env: Env) -> HealthScoreConfig {
        HealthScoreRegistry::get_config(&env)
    }

    // ── #759: Project Activity Feed / Timeline ───────────────────────────

    /// Return a paginated page of the activity feed for a project, optionally
    /// filtered by activity kind. Results are newest-first, max 100 per page.
    pub fn get_project_activity_feed(
        env: Env,
        project_id: u64,
        filter: Option<ActivityKind>,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<ActivityEntry>, ContractError> {
        ActivityFeedRegistry::get_activity_feed(&env, project_id, filter, offset, limit)
    }

    /// Return the total number of activity entries for a project.
    pub fn get_project_activity_count(
        env: Env,
        project_id: u64,
    ) -> Result<u32, ContractError> {
        ActivityFeedRegistry::get_activity_count(&env, project_id)
    }

    // ── #760: Automatic Metadata Enrichment ──────────────────────────────

    /// Admin/relayer: submit an enrichment suggestion for a project.
    /// Manual approval by the owner is required before changes are applied.
    pub fn submit_enrichment_suggestion(
        env: Env,
        project_id: u64,
        caller: Address,
        source: String,
        fields: MetadataEnrichmentFields,
    ) -> Result<EnrichmentSuggestion, ContractError> {
        MetadataEnrichmentRegistry::submit_suggestion(&env, project_id, &caller, source, fields)
    }

    /// Owner: approve a pending enrichment suggestion and apply the suggested
    /// fields to the project.
    pub fn approve_enrichment_suggestion(
        env: Env,
        project_id: u64,
        suggestion_id: u64,
        owner: Address,
    ) -> Result<(), ContractError> {
        MetadataEnrichmentRegistry::approve_suggestion(&env, project_id, suggestion_id, &owner)
    }

    /// Owner: reject a pending enrichment suggestion without applying it.
    pub fn reject_enrichment_suggestion(
        env: Env,
        project_id: u64,
        suggestion_id: u64,
        owner: Address,
    ) -> Result<(), ContractError> {
        MetadataEnrichmentRegistry::reject_suggestion(&env, project_id, suggestion_id, &owner)
    }

    /// Return all enrichment suggestions (pending and reviewed) for a project.
    pub fn get_enrichment_suggestions(
        env: Env,
        project_id: u64,
    ) -> Result<Vec<EnrichmentSuggestion>, ContractError> {
        MetadataEnrichmentRegistry::get_suggestions(&env, project_id)
    }

    /// Return only pending enrichment suggestions for a project.
    pub fn get_pending_enrichment_suggestions(
        env: Env,
        project_id: u64,
    ) -> Result<Vec<EnrichmentSuggestion>, ContractError> {
        MetadataEnrichmentRegistry::get_pending_suggestions(&env, project_id)
    }

    // =========================================================================
    // Trust and Safety Features (#788, #789, #790, #791)
    // =========================================================================

    /// Get fraud record for a project (#788)
    pub fn get_fraud_record(env: Env, project_id: u64) -> crate::types::FraudRecord {
        crate::trust_and_safety::TrustAndSafety::get_fraud_record(&env, project_id)
    }

    /// Admin flags a project for fraud manually (#788)
    pub fn flag_project_fraud(env: Env, admin: Address, project_id: u64, reason: String) -> Result<(), ContractError> {
        crate::trust_and_safety::TrustAndSafety::flag_project_fraud(&env, &admin, project_id, reason)
    }

    /// Set approved licenses for a category (#789)
    pub fn set_category_license_config(
        env: Env,
        admin: Address,
        category: String,
        approved_licenses: Vec<String>,
        exceptions_allowed: bool,
    ) -> Result<(), ContractError> {
        crate::trust_and_safety::TrustAndSafety::set_category_license_config(&env, &admin, category, approved_licenses, exceptions_allowed)
    }

    /// Get approved licenses config for a category (#789)
    pub fn get_category_license_config(env: Env, category: String) -> Option<crate::types::CategoryLicenseConfig> {
        crate::trust_and_safety::TrustAndSafety::get_category_license_config(&env, &category)
    }

    /// Verify reviewer identity (#790)
    pub fn verify_reviewer_identity(
        env: Env,
        admin: Address,
        reviewer: Address,
        email_verified: bool,
        social_proof_verified: bool,
        verification_method: Option<String>,
    ) -> Result<(), ContractError> {
        crate::trust_and_safety::TrustAndSafety::verify_reviewer_identity(&env, &admin, reviewer, email_verified, social_proof_verified, verification_method)
    }

    /// Get reviewer identity (#790)
    pub fn get_reviewer_identity(env: Env, reviewer: Address) -> crate::types::ReviewerIdentity {
        crate::trust_and_safety::TrustAndSafety::get_reviewer_identity(&env, &reviewer)
    }

    /// Get reviewer points (#791)
    pub fn get_reviewer_points(env: Env, reviewer: Address) -> crate::types::ReviewerPoints {
        crate::trust_and_safety::TrustAndSafety::get_reviewer_points(&env, &reviewer)
    }

    // ── #766: per-project access control lists ─────────────────────────────

    /// Owner/admin: set the project access mode (Public/Private/Custom).
    pub fn set_project_acl(
        env: Env,
        caller: Address,
        project_id: u64,
        mode: crate::project_controls::AccessMode,
    ) -> Result<(), ContractError> {
        crate::project_controls::ProjectControls::set_acl_mode(&env, caller, project_id, mode)
    }

    /// Owner/admin: add or update a whitelisted address.
    pub fn acl_add_address(
        env: Env,
        caller: Address,
        project_id: u64,
        target: Address,
        permission: crate::project_controls::AclPermission,
    ) -> Result<(), ContractError> {
        crate::project_controls::ProjectControls::acl_add_address(
            &env,
            caller,
            project_id,
            target,
            permission,
        )
    }

    /// Owner/admin: remove a whitelisted address.
    pub fn acl_remove_address(
        env: Env,
        caller: Address,
        project_id: u64,
        target: Address,
    ) -> Result<(), ContractError> {
        crate::project_controls::ProjectControls::acl_remove_address(&env, caller, project_id, target)
    }

    /// Current ACL for a project (defaults to Public when unset).
    pub fn get_project_acl(env: Env, project_id: u64) -> crate::project_controls::ProjectAcl {
        crate::project_controls::ProjectControls::get_acl_for(&env, project_id)
    }

    /// Access-change history log for a project (#766).
    pub fn get_acl_change_history(
        env: Env,
        project_id: u64,
    ) -> Vec<crate::project_controls::AclChangeRecord> {
        crate::project_controls::ProjectControls::get_acl_history(&env, project_id)
    }

    /// Whether `user` may view the project under its ACL.
    pub fn acl_check_view(env: Env, project_id: u64, user: Address) -> bool {
        crate::project_controls::ProjectControls::check_view_access(&env, project_id, &user)
    }

    /// Whether `user` may edit the project under its ACL.
    pub fn acl_check_edit(env: Env, project_id: u64, user: Address) -> bool {
        crate::project_controls::ProjectControls::check_edit_access(&env, project_id, &user)
    }

    /// Whether `user` may administer the project under its ACL.
    pub fn acl_check_admin(env: Env, project_id: u64, user: Address) -> bool {
        crate::project_controls::ProjectControls::check_admin_access(&env, project_id, &user)
    }

    // ── #767: collaboration tools ──────────────────────────────────────────

    /// Owner/admin: add a collaborator with a role.
    pub fn add_project_collaborator(
        env: Env,
        caller: Address,
        project_id: u64,
        collaborator: Address,
        role: crate::project_controls::CollaboratorRole,
    ) -> Result<(), ContractError> {
        crate::project_controls::ProjectControls::add_collaborator(
            &env,
            caller,
            project_id,
            collaborator,
            role,
        )
    }

    /// Owner/admin: change a collaborator's role.
    pub fn set_project_collaborator_role(
        env: Env,
        caller: Address,
        project_id: u64,
        collaborator: Address,
        role: crate::project_controls::CollaboratorRole,
    ) -> Result<(), ContractError> {
        crate::project_controls::ProjectControls::set_collaborator_role(
            &env,
            caller,
            project_id,
            collaborator,
            role,
        )
    }

    /// Owner/admin: remove a collaborator.
    pub fn remove_project_collaborator(
        env: Env,
        caller: Address,
        project_id: u64,
        collaborator: Address,
    ) -> Result<(), ContractError> {
        crate::project_controls::ProjectControls::remove_collaborator(
            &env,
            caller,
            project_id,
            collaborator,
        )
    }

    /// Collaborators for a project.
    pub fn get_project_collaborators(
        env: Env,
        project_id: u64,
    ) -> Vec<crate::project_controls::Collaborator> {
        crate::project_controls::ProjectControls::get_collaborator_list(&env, project_id)
    }

    /// Create a change proposal for a project.
    pub fn create_change_proposal(
        env: Env,
        caller: Address,
        project_id: u64,
        description: String,
    ) -> Result<u64, ContractError> {
        crate::project_controls::ProjectControls::create_proposal(&env, caller, project_id, description)
    }

    /// Approve a pending change proposal.
    pub fn approve_change_proposal(
        env: Env,
        caller: Address,
        proposal_id: u64,
    ) -> Result<(), ContractError> {
        crate::project_controls::ProjectControls::approve_proposal(&env, caller, proposal_id)
    }

    /// Reject a pending change proposal.
    pub fn reject_change_proposal(
        env: Env,
        caller: Address,
        proposal_id: u64,
    ) -> Result<(), ContractError> {
        crate::project_controls::ProjectControls::reject_proposal(&env, caller, proposal_id)
    }

    /// Fetch a change proposal by ID.
    pub fn get_change_proposal(
        env: Env,
        proposal_id: u64,
    ) -> Option<crate::project_controls::ChangeProposal> {
        crate::project_controls::ProjectControls::get_proposal(&env, proposal_id)
    }

    /// Proposal IDs for a project.
    pub fn get_project_proposals(env: Env, project_id: u64) -> Vec<u64> {
        crate::project_controls::ProjectControls::get_project_proposals(&env, project_id)
    }

    /// Add an entry to the shared changelog (contributors only).
    pub fn add_shared_changelog(
        env: Env,
        caller: Address,
        project_id: u64,
        cid: String,
        note: String,
    ) -> Result<u64, ContractError> {
        crate::project_controls::ProjectControls::add_changelog(&env, caller, project_id, cid, note)
    }

    /// Edit a shared changelog entry (contributors only).
    pub fn edit_shared_changelog(
        env: Env,
        caller: Address,
        entry_id: u64,
        cid: String,
        note: String,
    ) -> Result<(), ContractError> {
        crate::project_controls::ProjectControls::edit_changelog(&env, caller, entry_id, cid, note)
    }

    /// Fetch a shared changelog entry by ID.
    pub fn get_shared_changelog_entry(
        env: Env,
        entry_id: u64,
    ) -> Option<crate::project_controls::SharedChangelogEntry> {
        crate::project_controls::ProjectControls::get_changelog_entry(&env, entry_id)
    }

    /// Shared changelog entry IDs for a project.
    pub fn get_shared_changelog_ids(env: Env, project_id: u64) -> Vec<u64> {
        crate::project_controls::ProjectControls::get_project_changelog(&env, project_id)
    }

    /// Audit trail of collaboration/ACL/quarantine actions (#767).
    pub fn get_project_audit_trail(
        env: Env,
        project_id: u64,
    ) -> Vec<crate::project_controls::AuditEntry> {
        crate::project_controls::ProjectControls::get_audit_trail(&env, project_id)
    }

    // ── #765: field-level encrypted metadata ───────────────────────────────

    /// Owner: store a metadata field, optionally encrypted to `pubkey`.
    pub fn set_project_metadata_field(
        env: Env,
        caller: Address,
        project_id: u64,
        field: String,
        is_encrypted: bool,
        pubkey: String,
        payload: String,
    ) -> Result<(), ContractError> {
        crate::project_controls::ProjectControls::set_metadata_field(
            &env,
            caller,
            project_id,
            field,
            is_encrypted,
            pubkey,
            payload,
        )
    }

    /// Read a metadata field record.
    pub fn get_project_metadata_field(
        env: Env,
        project_id: u64,
        field: String,
    ) -> Option<crate::project_controls::EncryptedMetadata> {
        crate::project_controls::ProjectControls::get_metadata_field(&env, project_id, field)
    }

    /// Owner-gated read of a field payload (encrypted fields: owner only).
    pub fn read_project_metadata_payload(
        env: Env,
        caller: Address,
        project_id: u64,
        field: String,
    ) -> Result<String, ContractError> {
        crate::project_controls::ProjectControls::read_metadata_payload(
            &env,
            caller,
            project_id,
            field,
        )
    }

    /// Metadata field names stored for a project.
    pub fn list_project_metadata_fields(env: Env, project_id: u64) -> Vec<String> {
        crate::project_controls::ProjectControls::list_metadata_fields(&env, project_id)
    }

    // ── #764: project quarantine ───────────────────────────────────────────

    /// Admin: quarantine a project with a reason (30-day review period).
    pub fn quarantine_project(
        env: Env,
        admin: Address,
        project_id: u64,
        reason: String,
    ) -> Result<(), ContractError> {
        crate::project_controls::ProjectControls::quarantine_project(&env, admin, project_id, reason)
    }

    /// Quarantine state for a project, if quarantined.
    pub fn get_quarantine_state(
        env: Env,
        project_id: u64,
    ) -> Option<crate::project_controls::QuarantineState> {
        crate::project_controls::ProjectControls::get_quarantine_state(&env, project_id)
    }

    /// Whether a project is currently quarantined.
    pub fn is_project_quarantined(env: Env, project_id: u64) -> bool {
        crate::project_controls::ProjectControls::is_quarantined(&env, project_id)
    }

    /// Whether the 30-day quarantine review period has elapsed.
    pub fn quarantine_review_elapsed(env: Env, project_id: u64) -> bool {
        crate::project_controls::ProjectControls::review_period_elapsed(&env, project_id)
    }

    /// IDs of all currently quarantined projects.
    pub fn list_quarantined_projects(env: Env) -> Vec<u64> {
        crate::project_controls::ProjectControls::list_quarantined(&env)
    }

    /// Admin decision: restore a quarantined project.
    pub fn restore_quarantined_project(
        env: Env,
        admin: Address,
        project_id: u64,
    ) -> Result<(), ContractError> {
        crate::project_controls::ProjectControls::restore_quarantine(&env, admin, project_id)
    }

    /// Admin decision: delete (archive) a quarantined project.
    pub fn delete_quarantined_project(
        env: Env,
        admin: Address,
        project_id: u64,
    ) -> Result<(), ContractError> {
        crate::project_controls::ProjectControls::delete_quarantine(&env, admin, project_id)
    }

    // --- Community bounties (#950) ---

    /// Post a bounty for a project. Only the project owner may post.
    pub fn post_bounty(
        env: Env,
        project_id: u64,
        owner: Address,
        title: String,
        amount: u128,
        deadline: u64,
        acceptance_criteria: String,
    ) -> Result<u64, ContractError> {
        crate::bounty_registry::BountyRegistry::post_bounty(
            &env,
            project_id,
            owner,
            title,
            amount,
            deadline,
            acceptance_criteria,
        )
    }

    /// Claim an open bounty with a work CID.
    pub fn claim_bounty(
        env: Env,
        bounty_id: u64,
        claimant: Address,
        work_cid: String,
    ) -> Result<(), ContractError> {
        crate::bounty_registry::BountyRegistry::claim_bounty(&env, bounty_id, claimant, work_cid)
    }

    /// Owner approves a claimed bounty and records payout history.
    pub fn approve_bounty(
        env: Env,
        bounty_id: u64,
        owner: Address,
    ) -> Result<(), ContractError> {
        crate::bounty_registry::BountyRegistry::approve_bounty(&env, bounty_id, owner)
    }

    /// Get a bounty by id.
    pub fn get_bounty(env: Env, bounty_id: u64) -> Option<crate::types::Bounty> {
        crate::bounty_registry::BountyRegistry::get_bounty(&env, bounty_id)
    }

    /// List bounty ids for a project and payout history for a bounty.
    pub fn list_project_bounties(env: Env, project_id: u64) -> Vec<u64> {
        crate::bounty_registry::BountyRegistry::list_project_bounties(&env, project_id)
    }

    /// Payout history for a bounty.
    pub fn bounty_payout_history(env: Env, bounty_id: u64) -> Vec<crate::types::BountyPayout> {
        crate::bounty_registry::BountyRegistry::payout_history(&env, bounty_id)
    }
}
