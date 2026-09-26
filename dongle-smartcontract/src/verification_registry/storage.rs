//! Verification registry storage mutations: request/approve/reject, renewal, and assignment.

use crate::admin_action_log::AdminActionLog;
use crate::admin_manager::AdminManager;
use crate::auth::{require_admin_auth, require_owner_auth};
use crate::constants::MAX_PAGE_LIMIT;
use crate::errors::ContractError;
use crate::events::{
    publish_verification_approved_event, publish_verification_appeal_reviewed_event,
    publish_verification_appeal_submitted_event, publish_verification_evidence_updated_event,
    publish_verification_expired_event, publish_verification_rejected_event,
    publish_verification_expiry_notification_event,
    publish_verification_renewal_approved_event, publish_verification_renewal_rejected_event,
    publish_verification_renewal_requested_event, publish_verification_renewed_event,
    publish_verification_requested_event, publish_verification_revoked_event,
    publish_verification_restored_event, publish_verification_suspended_event,
};
use crate::fee_manager::FeeManager;
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::{ExtensionKey, ExtensionKey2, StorageKey};
use crate::storage_keys::NotificationKey;
use crate::types::{
    AdminActionType, VerificationEvidenceComparison, VerificationEvidenceVersion,
    VerificationBatchAction, VerificationBatchReport, VerificationBatchResult, VerificationRecord,
    VerificationRenewalRecord, VerificationStatus,
    AdminActionType, NotificationDeliveryStatus, VerificationAppeal, VerificationExpiryNotification,
    VerificationRecord, VerificationRenewalRecord, VerificationRejectionState,
    VerificationRiskAssessment, VerificationRiskModel, VerificationStatus, VerificationSuspension,
};
use crate::utils::Utils;
use crate::verification_registry::state_machine::VerificationStateMachine;
use crate::verification_registry::validation::VerificationValidation;
use soroban_sdk::{Address, Env, String, Vec};
use crate::storage_keys::ExtensionKey2;

pub struct VerificationRegistry;

impl VerificationRegistry {
    pub fn get_verification_risk_model(env: &Env) -> VerificationRiskModel {
        env.storage()
            .persistent()
            .get(&ExtensionKey2::VerificationRiskModel)
            .unwrap_or(VerificationRiskModel {
                model_version: 1,
                age_weight: 400,
                reputation_weight: 300,
                rating_weight: 300,
                threshold: 600,
            })
    }

    pub fn set_verification_risk_model(
        env: &Env,
        admin: Address,
        model: VerificationRiskModel,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;
        if model.age_weight == 0 && model.reputation_weight == 0 && model.rating_weight == 0 {
            return Err(ContractError::InvalidInput);
        }
        if model.threshold > 1000 {
            return Err(ContractError::InvalidInput);
        }
        env.storage()
            .persistent()
            .set(&ExtensionKey2::VerificationRiskModel, &model);
        AdminActionLog::record_action(
            env,
            admin,
            AdminActionType::VerificationRiskModelUpdated,
            None,
            None,
            None,
        );
        Ok(())
    }

    fn calculate_risk_assessment(
        env: &Env,
        project_id: u64,
        request_id: u64,
        created_at: u64,
    ) -> VerificationRiskAssessment {
        let model = Self::get_verification_risk_model(env);
        let now = env.ledger().timestamp();
        let age = now.saturating_sub(created_at);
        let project_age_score = if age < crate::constants::RISK_MODEL_MONTH_SECONDS {
            1000
        } else if age < crate::constants::RISK_MODEL_YEAR_SECONDS {
            500
        } else {
            0
        };
        let stats = env
            .storage()
            .persistent()
            .get::<_, crate::types::ProjectStats>(&StorageKey::ProjectStats(project_id))
            .unwrap_or(crate::types::ProjectStats {
                rating_sum: 0,
                review_count: 0,
                average_rating: 0,
            });
        let reputation_score = if stats.review_count == 0 {
            1000
        } else if stats.review_count < 3 {
            700
        } else if stats.review_count < 10 {
            350
        } else {
            0
        };
        let rating_score = if stats.review_count == 0 || stats.average_rating <= 200 {
            900
        } else if stats.average_rating <= 300 {
            600
        } else if stats.average_rating <= 400 {
            250
        } else {
            0
        };
        let total_weight = (model.age_weight as u64)
            .saturating_add(model.reputation_weight as u64)
            .saturating_add(model.rating_weight as u64);
        let score = ((project_age_score as u64 * model.age_weight as u64)
            .saturating_add(reputation_score as u64 * model.reputation_weight as u64)
            .saturating_add(rating_score as u64 * model.rating_weight as u64)
            / total_weight) as u32;
        VerificationRiskAssessment {
            request_id,
            project_id,
            model_version: model.model_version,
            project_age_score,
            reputation_score,
            rating_score,
            score,
            threshold: model.threshold,
            flagged: score > model.threshold,
            override_flag: None,
            overridden_by: None,
            assessed_at: now,
        }
    }

    pub fn get_verification_risk_assessment(
        env: &Env,
        request_id: u64,
    ) -> Option<VerificationRiskAssessment> {
        env.storage()
            .persistent()
            .get(&ExtensionKey2::VerificationRiskAssessment(request_id))
    }

    pub fn get_high_risk_verification_requests(env: &Env) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&ExtensionKey2::HighRiskVerificationRequests)
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn override_verification_risk(
        env: &Env,
        request_id: u64,
        admin: Address,
        flagged: bool,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;
        let mut assessment = Self::get_verification_risk_assessment(env, request_id)
            .ok_or(ContractError::VerificationNotFound)?;
        assessment.override_flag = Some(flagged);
        assessment.overridden_by = Some(admin.clone());
        assessment.flagged = flagged;
        env.storage().persistent().set(
            &ExtensionKey2::VerificationRiskAssessment(request_id),
            &assessment,
        );
        let mut high_risk = Self::get_high_risk_verification_requests(env);
        if flagged {
            Utils::add_unique_to_vec(&mut high_risk, &request_id);
        } else {
            high_risk = Utils::remove_item_from_vec(env, &high_risk, &request_id);
        }
        env.storage()
            .persistent()
            .set(&ExtensionKey2::HighRiskVerificationRequests, &high_risk);
        AdminActionLog::record_action(
            env,
            admin,
            AdminActionType::VerificationRiskOverridden,
            Some(assessment.project_id),
            None,
            None,
        );
        Ok(())
    }

    pub fn request_verification(
        env: &Env,
        project_id: u64,
        requester: Address,
        evidence_cid: String,
    ) -> Result<(), ContractError> {
        // 1. Validate project existence and ownership
        let mut project =
            ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        require_owner_auth(&requester, &project.owner)?;

        // 2. Check minimum project age
        let min_age = Self::get_min_project_age(env);
        let current_time = env.ledger().timestamp();
        if current_time < project.created_at + min_age {
            return Err(ContractError::ProjectTooYoung);
        }

        // 2.5 Auto-process expiry if verification has expired
        if project.verification_status == VerificationStatus::Verified
            && Self::is_verification_expired(env, project_id).unwrap_or(false)
        {
            Self::process_verification_expiry(env, project_id)?;
            project = ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;
        }

        // 3. Check if project can request verification using state machine
        if !VerificationStateMachine::can_request_verification(project.verification_status) {
            return Err(ContractError::InvalidStatus);
        }

        // 4. Validate state transition using centralized state machine
        VerificationStateMachine::validate_transition(
            project.verification_status,
            VerificationStatus::Pending,
        )?;

        // 5. Validate evidence before any storage mutation, including fee consumption.
        VerificationValidation::validate_evidence_cid(&evidence_cid)?;

        // Capture the previous request (if any) before it's superseded below.
        // The previous `VerificationRecord` is never mutated or removed here —
        // it remains reachable via `get_verification_record` and
        // `get_verification_history` exactly as it was decided, preserving its
        // original status and evidence CID. Only the "current" pointer
        // (`StorageKey::Verification`) and the project's `current_verification_id`
        // move to the new request.
        let previous_request_id = project.current_verification_id;

        // 6. Consume fee payment when configured
        let fee_amount = match FeeManager::get_fee_config(env) {
            Ok(config) if config.verification_fee > 0 => {
                FeeManager::consume_fee_payment(
                    env,
                    project_id,
                    requester.clone(),
                    config.verification_fee,
                )?;
                config.verification_fee
            }
            Ok(config) => config.verification_fee,
            Err(_) => 0,
        };

        // 7. Generate a unique request ID
        let mut request_id = env
            .storage()
            .persistent()
            .get::<_, u64>(&StorageKey::NextVerificationRequestId)
            .unwrap_or(0);
        request_id += 1;
        env.storage()
            .persistent()
            .set(&StorageKey::NextVerificationRequestId, &request_id);

        // 7. Create record
        let now = env.ledger().timestamp();
        let record = VerificationRecord {
            request_id,
            project_id,
            requester: requester.clone(),
            status: VerificationStatus::Pending,
            evidence_cid: evidence_cid.clone(),
            requested_at: now,
            decided_at: 0,
            fee_amount,
            revoke_reason: None,
            expires_at: 0,
            last_renewed_at: 0,
            assigned_admin: None,
        };

        let assessment = Self::calculate_risk_assessment(env, project_id, request_id, project.created_at);
        env.storage().persistent().set(
            &ExtensionKey2::VerificationRiskAssessment(request_id),
            &assessment,
        );
        if assessment.flagged {
            let mut high_risk = Self::get_high_risk_verification_requests(env);
            Utils::add_unique_to_vec(&mut high_risk, &request_id);
            env.storage()
                .persistent()
                .set(&ExtensionKey2::HighRiskVerificationRequests, &high_risk);
        }

        // 8. Save to historical record
        env.storage()
            .persistent()
            .set(&StorageKey::VerificationRecord(request_id), &record);

        let mut evidence_versions = Vec::new(env);
        evidence_versions.push_back(VerificationEvidenceVersion {
            version: 1,
            evidence_cid: evidence_cid.clone(),
            submitted_by: requester.clone(),
            submitted_at: now,
        });
        env.storage().persistent().set(
            &ExtensionKey2::VerificationEvidenceVersions(request_id),
            &evidence_versions,
        );

        // 9. Save to current/latest backward-compatible record
        env.storage()
            .persistent()
            .set(&StorageKey::Verification(project_id), &request_id);

        // 10. Append request_id to ProjectVerificationHistory
        let mut history = env
            .storage()
            .persistent()
            .get::<_, Vec<u64>>(&StorageKey::ProjectVerificationHistory(project_id))
            .unwrap_or_else(|| Vec::new(env));
        history.push_back(request_id);
        env.storage().persistent().set(
            &StorageKey::ProjectVerificationHistory(project_id),
            &history,
        );

        let mut pending = env
            .storage()
            .persistent()
            .get::<_, Vec<u64>>(&ExtensionKey::PendingVerificationRequests)
            .unwrap_or_else(|| Vec::new(env));
        Utils::add_unique_to_vec(&mut pending, &request_id);
        env.storage()
            .persistent()
            .set(&ExtensionKey::PendingVerificationRequests, &pending);

        env.storage()
            .persistent()
            .remove(&ExtensionKey2::VerificationAppeals(project_id));
        env.storage()
            .persistent()
            .remove(&ExtensionKey2::VerificationRejection(project_id));

        // 11. Update project status to Pending
        project.verification_status = VerificationStatus::Pending;
        project.current_verification_id = Some(request_id);
        project.updated_at = now;
        env.storage()
            .persistent()
            .set(&StorageKey::Project(project_id), &project);

        publish_verification_requested_event(
            env,
            project_id,
            requester,
            evidence_cid,
            request_id,
            previous_request_id,
        );
        Ok(())
    }

    /// Updates the verification evidence CID for a pending or verified request.
    ///
    /// This can only be called by the project owner when the request is in the
    /// Pending status. A verified request is reset to Pending after a changed CID.
    /// The supplied CID is validated using the standard CID validation rules.
    /// Once updated successfully, it persists a new immutable version and publishes
    /// a `VerificationEvidenceUpdated` event.
    pub fn update_verification_evidence(
        env: &Env,
        project_id: u64,
        caller: Address,
        new_evidence_cid: String,
    ) -> Result<(), ContractError> {
        // 1. Validate project existence and ownership
        let project =
            ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        require_owner_auth(&caller, &project.owner)?;

        // 2. Retrieve verification record
        let mut record =
            Self::get_verification(env, project_id).ok_or(ContractError::VerificationNotFound)?;

        // 3. Finalized rejected requests cannot be edited. A verified request
        // may be edited, but the approval is reset below.
        if record.status == VerificationStatus::Rejected
            || record.status == VerificationStatus::Unverified
        {
            return Err(ContractError::InvalidStatus);
        }

        // 4. Validate CID before state mutation
        VerificationValidation::validate_evidence_cid(&new_evidence_cid)?;

        if record.evidence_cid == new_evidence_cid {
            return Ok(());
        }

        // 5. Append a new immutable evidence snapshot.
        let old_evidence_cid = record.evidence_cid;
        let mut evidence_versions = env
            .storage()
            .persistent()
            .get::<_, Vec<VerificationEvidenceVersion>>(
                &ExtensionKey2::VerificationEvidenceVersions(record.request_id),
            )
            .unwrap_or_else(|| Vec::new(env));
        let version = evidence_versions.len().saturating_add(1);
        evidence_versions.push_back(VerificationEvidenceVersion {
            version,
            evidence_cid: new_evidence_cid.clone(),
            submitted_by: caller.clone(),
            submitted_at: env.ledger().timestamp(),
        });
        env.storage().persistent().set(
            &ExtensionKey2::VerificationEvidenceVersions(record.request_id),
            &evidence_versions,
        );

        // 6. Update the current snapshot. Evidence changes invalidate prior approval.
        record.evidence_cid = new_evidence_cid.clone();
        if record.status == VerificationStatus::Verified {
            record.status = VerificationStatus::Pending;
            record.decided_at = 0;
            record.expires_at = 0;
            let mut project = ProjectRegistry::get_project(env, project_id)
                .ok_or(ContractError::ProjectNotFound)?;
            project.verification_status = VerificationStatus::Pending;
            project.updated_at = env.ledger().timestamp();
            env.storage()
                .persistent()
                .set(&StorageKey::Project(project_id), &project);

            let mut pending = env
                .storage()
                .persistent()
                .get::<_, Vec<u64>>(&ExtensionKey::PendingVerificationRequests)
                .unwrap_or_else(|| Vec::new(env));
            Utils::add_unique_to_vec(&mut pending, &record.request_id);
            env.storage()
                .persistent()
                .set(&ExtensionKey::PendingVerificationRequests, &pending);
        }

        env.storage()
            .persistent()
            .set(&StorageKey::VerificationRecord(record.request_id), &record);

        // 7. Emit event
        publish_verification_evidence_updated_event(
            env,
            project_id,
            caller,
            old_evidence_cid,
            new_evidence_cid,
        );

        Ok(())
    }

    pub fn approve_verification(
        env: &Env,
        project_id: u64,
        admin: Address,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;

        if crate::admin_manager::AdminManager::get_admin_approval_threshold(env) > 1 {
            return Err(ContractError::Unauthorized);
        }

        // Get project
        let mut project =
            ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        // Get verification record first - returns VerificationNotFound if missing
        let mut record =
            Self::get_verification(env, project_id).ok_or(ContractError::VerificationNotFound)?;

        // Verify integrity hash: ensure project metadata (name, slug, category,
        // description) has not changed since the hash was last written by
        // register_project or update_project. The canonical payload uses a versioned
        // format (`project-integrity-v1|name|slug|category|description`), while
        // the legacy unversioned hash remains accepted for backward compatibility.
        if let Some(stored_hash) = ProjectRegistry::get_project_integrity_hash(env, project_id) {
            let matches_current_or_legacy = ProjectRegistry::hash_matches_current_or_legacy(
                env,
                &project.name,
                &project.slug,
                &project.category,
                &project.description,
                &stored_hash,
            );
            if !matches_current_or_legacy {
                return Err(ContractError::InvalidProjectData);
            }
        }

        // Then validate state transition
        VerificationStateMachine::validate_transition(
            project.verification_status,
            VerificationStatus::Verified,
        )?;

        let now = env.ledger().timestamp();

        // Update record – stamp the expiry timestamp
        let duration = AdminManager::get_verification_duration(env);
        record.status = VerificationStatus::Verified;
        record.expires_at = now.saturating_add(duration);
        record.decided_at = now;
        env.storage()
            .persistent()
            .set(&StorageKey::Verification(project_id), &record.request_id);
        env.storage()
            .persistent()
            .set(&StorageKey::VerificationRecord(record.request_id), &record);

        Self::remove_pending_request(env, record.request_id);

        // Update project
        project.verification_status = VerificationStatus::Verified;
        project.current_verification_id = Some(record.request_id);
        project.updated_at = now;
        env.storage()
            .persistent()
            .set(&StorageKey::Project(project_id), &project);

        publish_verification_approved_event(env, project_id, admin.clone(), now);
        crate::notification_registry::NotificationRegistry::emit_project_notification(
            env,
            project_id,
            crate::types::NotificationKind::VerificationApproved,
        );

        AdminActionLog::record_action(
            env,
            admin.clone(),
            AdminActionType::VerificationApproved,
            Some(project_id),
            None,
            None,
        );

        // Initiate 30-day probationary period under enhanced monitoring
        let _ = crate::probation_registry::ProbationRegistry::start_probation(
            env,
            project_id,
            record.request_id,
            &admin,
        );

        Ok(())
    }

    pub fn reject_verification(
        env: &Env,
        project_id: u64,
        admin: Address,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;

        if crate::admin_manager::AdminManager::get_admin_approval_threshold(env) > 1 {
            return Err(ContractError::Unauthorized);
        }

        // Get project
        let mut project =
            ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        // Get verification record first - returns VerificationNotFound if missing
        let mut record =
            Self::get_verification(env, project_id).ok_or(ContractError::VerificationNotFound)?;

        // Then validate state transition
        VerificationStateMachine::validate_transition(
            project.verification_status,
            VerificationStatus::Rejected,
        )?;

        let now = env.ledger().timestamp();

        // Update record
        record.status = VerificationStatus::Rejected;
        record.decided_at = now;
        env.storage()
            .persistent()
            .set(&StorageKey::Verification(project_id), &record.request_id);
        env.storage()
            .persistent()
            .set(&StorageKey::VerificationRecord(record.request_id), &record);

        // Update project
        project.verification_status = VerificationStatus::Rejected;
        project.current_verification_id = Some(record.request_id);
        project.updated_at = now;
        env.storage()
            .persistent()
            .set(&StorageKey::Project(project_id), &project);

        // Issue #472: a rejected request must not keep the requester's fee.
        // The payout is recorded as claimable rather than transferred here —
        // moving tokens out of the treasury needs `treasury.require_auth()`,
        // which the rejecting admin cannot generally supply. See
        // `FeeManager::record_verification_refund`.
        FeeManager::record_verification_refund(
            env,
            project_id,
            record.request_id,
            record.requester.clone(),
            record.fee_amount,
        )?;

        Self::remove_pending_request(env, record.request_id);

        publish_verification_rejected_event(env, project_id, admin.clone(), now);
        crate::notification_registry::NotificationRegistry::emit_project_notification(
            env,
            project_id,
            crate::types::NotificationKind::VerificationRejected,
        );

        env.storage()
            .persistent()
            .remove(&ExtensionKey2::VerificationAppeals(project_id));

        let rejection_state = VerificationRejectionState {
            project_id,
            request_id: record.request_id,
            rejected_by: admin.clone(),
            rejected_at: now,
            appeal_count: 0,
        };
        env.storage().persistent().set(
            &ExtensionKey2::VerificationRejection(project_id),
            &rejection_state,
        );

        AdminActionLog::record_action(
            env,
            admin,
            AdminActionType::VerificationRejected,
            Some(project_id),
            None,
            None,
        );

        Ok(())
    }

    /// Atomically approve or reject up to 100 pending verification requests.
    /// Every request is validated before the first state mutation.
    pub fn decide_verifications_batch(
        env: &Env,
        request_ids: Vec<u64>,
        admin: Address,
        action: VerificationBatchAction,
    ) -> Result<VerificationBatchReport, ContractError> {
        require_admin_auth(env, &admin)?;

        if crate::admin_manager::AdminManager::get_admin_approval_threshold(env) > 1 {
            return Err(ContractError::Unauthorized);
        }
        if request_ids.is_empty() || request_ids.len() > 100 {
            return Err(ContractError::InvalidInput);
        }

        let mut pending_records = Vec::new(env);
        for i in 0..request_ids.len() {
            let request_id = request_ids.get(i).ok_or(ContractError::InvalidInput)?;
            if request_ids.iter().take(i).any(|id| id == request_id) {
                return Err(ContractError::InvalidInput);
            }

            let record = Self::get_verification_record(env, request_id)
                .ok_or(ContractError::VerificationNotFound)?;
            if record.status != VerificationStatus::Pending {
                return Err(ContractError::InvalidStatus);
            }

            let project = ProjectRegistry::get_project(env, record.project_id)
                .ok_or(ContractError::ProjectNotFound)?;
            if project.current_verification_id != Some(request_id) {
                return Err(ContractError::InvalidStatus);
            }
            VerificationStateMachine::validate_transition(
                project.verification_status,
                match action {
                    VerificationBatchAction::Approve => VerificationStatus::Verified,
                    VerificationBatchAction::Reject => VerificationStatus::Rejected,
                },
            )?;

            if action == VerificationBatchAction::Approve {
                if let Some(stored_hash) = ProjectRegistry::get_project_integrity_hash(
                    env,
                    record.project_id,
                ) {
                    if !ProjectRegistry::hash_matches_current_or_legacy(
                        env,
                        &project.name,
                        &project.slug,
                        &project.category,
                        &project.description,
                        &stored_hash,
                    ) {
                        return Err(ContractError::InvalidProjectData);
                    }
                }
            }

            pending_records.push_back((project, record));
        }

        let now = env.ledger().timestamp();
        let mut results = Vec::new(env);
        for i in 0..pending_records.len() {
            let (mut project, mut record) = pending_records
                .get(i)
                .ok_or(ContractError::VerificationNotFound)?;
            let status = match action {
                VerificationBatchAction::Approve => {
                    record.status = VerificationStatus::Verified;
                    record.expires_at = now.saturating_add(
                        AdminManager::get_verification_duration(env),
                    );
                    record.decided_at = now;
                    project.verification_status = VerificationStatus::Verified;
                    VerificationStatus::Verified
                }
                VerificationBatchAction::Reject => {
                    record.status = VerificationStatus::Rejected;
                    record.decided_at = now;
                    project.verification_status = VerificationStatus::Rejected;
                    FeeManager::record_verification_refund(
                        env,
                        record.project_id,
                        record.request_id,
                        record.requester.clone(),
                        record.fee_amount,
                    )?;
                    VerificationStatus::Rejected
                }
            };

            project.current_verification_id = Some(record.request_id);
            project.updated_at = now;
            env.storage().persistent().set(
                &StorageKey::VerificationRecord(record.request_id),
                &record,
            );
            env.storage()
                .persistent()
                .set(&StorageKey::Project(record.project_id), &project);
            Self::remove_pending_request(env, record.request_id);

            match action {
                VerificationBatchAction::Approve => {
                    publish_verification_approved_event(
                        env,
                        record.project_id,
                        admin.clone(),
                        now,
                    );
                    crate::notification_registry::NotificationRegistry::emit_project_notification(
                        env,
                        record.project_id,
                        crate::types::NotificationKind::VerificationApproved,
                    );
                    AdminActionLog::record_action(
                        env,
                        admin.clone(),
                        AdminActionType::VerificationApproved,
                        Some(record.project_id),
                        None,
                        None,
                    );
                    let _ = crate::probation_registry::ProbationRegistry::start_probation(
                        env,
                        record.project_id,
                        record.request_id,
                        &admin,
                    );
                }
                VerificationBatchAction::Reject => {
                    publish_verification_rejected_event(
                        env,
                        record.project_id,
                        admin.clone(),
                        now,
                    );
                    crate::notification_registry::NotificationRegistry::emit_project_notification(
                        env,
                        record.project_id,
                        crate::types::NotificationKind::VerificationRejected,
                    );
                    AdminActionLog::record_action(
                        env,
                        admin.clone(),
                        AdminActionType::VerificationRejected,
                        Some(record.project_id),
                        None,
                        None,
                    );
                }
            }

            results.push_back(VerificationBatchResult {
                request_id: record.request_id,
                project_id: record.project_id,
                status,
                decided_at: now,
            });
        }

        Ok(VerificationBatchReport {
            action,
            total: results.len(),
            results,
        })
    }

    pub fn approve_verifications_batch(
        env: &Env,
        request_ids: Vec<u64>,
        admin: Address,
    ) -> Result<VerificationBatchReport, ContractError> {
        Self::decide_verifications_batch(
            env,
            request_ids,
            admin,
            VerificationBatchAction::Approve,
        )
    }

    pub fn reject_verifications_batch(
        env: &Env,
        request_ids: Vec<u64>,
        admin: Address,
    ) -> Result<VerificationBatchReport, ContractError> {
        Self::decide_verifications_batch(
            env,
            request_ids,
            admin,
            VerificationBatchAction::Reject,
        )
    }

    pub fn submit_verification_appeal(
        env: &Env,
        project_id: u64,
        owner: Address,
        evidence_cid: String,
    ) -> Result<(), ContractError> {
        let project =
            ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;
        require_owner_auth(&owner, &project.owner)?;

        let record = Self::get_verification(env, project_id).ok_or(ContractError::VerificationNotFound)?;
        if record.status != VerificationStatus::Rejected {
            return Err(ContractError::InvalidStatus);
        }

        VerificationValidation::validate_evidence_cid(&evidence_cid)?;

        let mut rejection_state = env
            .storage()
            .persistent()
            .get::<_, VerificationRejectionState>(&ExtensionKey2::VerificationRejection(project_id))
            .unwrap_or(VerificationRejectionState {
                project_id,
                request_id: record.request_id,
                rejected_by: project.owner.clone(),
                rejected_at: record.decided_at,
                appeal_count: 0,
            });

        if rejection_state.request_id != record.request_id {
            rejection_state = VerificationRejectionState {
                project_id,
                request_id: record.request_id,
                rejected_by: record
                    .assigned_admin
                    .clone()
                    .unwrap_or_else(|| project.owner.clone()),
                rejected_at: record.decided_at,
                appeal_count: 0,
            };
        }

        let mut appeals = env
            .storage()
            .persistent()
            .get::<_, Vec<VerificationAppeal>>(&ExtensionKey2::VerificationAppeals(project_id))
            .unwrap_or_else(|| Vec::new(env));

        if rejection_state.appeal_count >= 2 || appeals.len() >= 2 {
            return Err(ContractError::AppealLimitExceeded);
        }

        let now = env.ledger().timestamp();
        let appeal = VerificationAppeal {
            project_id,
            request_id: record.request_id,
            owner: owner.clone(),
            rejected_by: rejection_state.rejected_by.clone(),
            evidence_cid: evidence_cid.clone(),
            submitted_at: now,
            reviewed_by: None,
            approved: None,
            reviewed_at: 0,
        };
        appeals.push_back(appeal);
        rejection_state.appeal_count = appeals.len() as u32;
        env.storage()
            .persistent()
            .set(&ExtensionKey2::VerificationAppeals(project_id), &appeals);
        env.storage().persistent().set(
            &ExtensionKey2::VerificationRejection(project_id),
            &rejection_state,
        );

        publish_verification_appeal_submitted_event(
            env,
            project_id,
            owner,
            evidence_cid,
            rejection_state.appeal_count,
        );

        AdminActionLog::record_action(
            env,
            owner,
            AdminActionType::VerificationAppealSubmitted,
            Some(project_id),
            None,
            None,
        );

        Ok(())
    }

    pub fn review_verification_appeal(
        env: &Env,
        project_id: u64,
        admin: Address,
        approved: bool,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;

        let rejection_state = env
            .storage()
            .persistent()
            .get::<_, VerificationRejectionState>(&ExtensionKey2::VerificationRejection(project_id))
            .ok_or(ContractError::AppealNotFound)?;

        if admin == rejection_state.rejected_by {
            return Err(ContractError::Unauthorized);
        }

        let mut project =
            ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;
        let mut record = Self::get_verification(env, project_id).ok_or(ContractError::VerificationNotFound)?;
        if record.status != VerificationStatus::Rejected {
            return Err(ContractError::InvalidStatus);
        }

        let mut appeals = env
            .storage()
            .persistent()
            .get::<_, Vec<VerificationAppeal>>(&ExtensionKey2::VerificationAppeals(project_id))
            .unwrap_or_else(|| Vec::new(env));
        if appeals.is_empty() {
            return Err(ContractError::AppealNotFound);
        }

        let last_index = appeals.len().checked_sub(1).ok_or(ContractError::AppealNotFound)?;
        let mut appeal = appeals.get(last_index).ok_or(ContractError::AppealNotFound)?;
        if appeal.reviewed_by.is_some() || appeal.approved.is_some() {
            return Err(ContractError::AppealAlreadyReviewed);
        }

        let now = env.ledger().timestamp();
        appeal.reviewed_by = Some(admin.clone());
        appeal.approved = Some(approved);
        appeal.reviewed_at = now;
        appeals.set(last_index, appeal.clone());
        env.storage()
            .persistent()
            .set(&ExtensionKey2::VerificationAppeals(project_id), &appeals);

        if approved {
            record.status = VerificationStatus::Verified;
            record.decided_at = now;
            let duration = AdminManager::get_verification_duration(env);
            record.expires_at = now.saturating_add(duration);
            env.storage()
                .persistent()
                .set(&StorageKey::Verification(project_id), &record.request_id);
            env.storage()
                .persistent()
                .set(&StorageKey::VerificationRecord(record.request_id), &record);

            project.verification_status = VerificationStatus::Verified;
            project.current_verification_id = Some(record.request_id);
            project.updated_at = now;
            env.storage()
                .persistent()
                .set(&StorageKey::Project(project_id), &project);

            env.storage()
                .persistent()
                .remove(&ExtensionKey2::VerificationRejection(project_id));
            env.storage()
                .persistent()
                .remove(&ExtensionKey::FeeRefund(project_id));
            publish_verification_approved_event(env, project_id, admin.clone(), now);
            crate::notification_registry::NotificationRegistry::emit_project_notification(
                env,
                project_id,
                crate::types::NotificationKind::VerificationApproved,
            );
            AdminActionLog::record_action(
                env,
                admin,
                AdminActionType::VerificationAppealApproved,
                Some(project_id),
                None,
                None,
            );
        } else {
            AdminActionLog::record_action(
                env,
                admin,
                AdminActionType::VerificationAppealRejected,
                Some(project_id),
                None,
                None,
            );
        }

        publish_verification_appeal_reviewed_event(env, project_id, admin, approved);
        Ok(())
    }

    pub fn get_verification_appeals(env: &Env, project_id: u64) -> Vec<VerificationAppeal> {
        env.storage()
            .persistent()
            .get::<_, Vec<VerificationAppeal>>(&ExtensionKey2::VerificationAppeals(project_id))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn get_verification(env: &Env, project_id: u64) -> Option<VerificationRecord> {
        Self::restore_expired_suspension(env, project_id);
        let request_id = env
            .storage()
            .persistent()
            .get::<_, u64>(&StorageKey::Verification(project_id))?;
        env.storage()
            .persistent()
            .get::<_, VerificationRecord>(&StorageKey::VerificationRecord(request_id))
    }

    fn restore_expired_suspension(env: &Env, project_id: u64) {
        let now = env.ledger().timestamp();
        let request_id = match env
            .storage()
            .persistent()
            .get::<_, u64>(&StorageKey::Verification(project_id))
        {
            Some(id) => id,
            None => return,
        };
        let mut record = match env
            .storage()
            .persistent()
            .get::<_, VerificationRecord>(&StorageKey::VerificationRecord(request_id))
        {
            Some(record) => record,
            None => return,
        };
        if record.status != VerificationStatus::Suspended {
            return;
        }
        let mut timeline = env
            .storage()
            .persistent()
            .get::<_, Vec<VerificationSuspension>>(
                &ExtensionKey::ProjectVerificationSuspensions(project_id),
            )
            .unwrap_or_else(|| Vec::new(env));
        let index = match timeline.len().checked_sub(1) {
            Some(index) => index,
            None => return,
        };
        let mut entry = match timeline.get(index) {
            Some(entry) if entry.restored_at.is_none() && now >= entry.restore_at => entry,
            _ => return,
        };
        record.status = VerificationStatus::Verified;
        entry.restored_at = Some(now);
        env.storage()
            .persistent()
            .set(&StorageKey::VerificationRecord(request_id), &record);
        timeline.set(index, entry);
        env.storage().persistent().set(
            &ExtensionKey::ProjectVerificationSuspensions(project_id),
            &timeline,
        );
        if let Some(mut project) = ProjectRegistry::get_project(env, project_id) {
            project.verification_status = VerificationStatus::Verified;
            project.updated_at = now;
            env.storage()
                .persistent()
                .set(&StorageKey::Project(project_id), &project);
        }
        publish_verification_restored_event(env, project_id, None);
    }

    pub fn suspend_verification(
        env: &Env,
        project_id: u64,
        admin: Address,
        reason: String,
        investigation_ticket: String,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;
        let mut project =
            ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;
        if project.verification_status != VerificationStatus::Verified {
            return Err(ContractError::InvalidStatus);
        }
        let mut record = Self::get_verification(env, project_id)
            .ok_or(ContractError::VerificationNotFound)?;
        let now = env.ledger().timestamp();
        let restore_at = now.saturating_add(crate::constants::MAX_VERIFICATION_SUSPENSION_SECONDS);
        record.status = VerificationStatus::Suspended;
        env.storage()
            .persistent()
            .set(&StorageKey::VerificationRecord(record.request_id), &record);
        let entry = VerificationSuspension {
            project_id,
            request_id: record.request_id,
            admin: admin.clone(),
            reason: reason.clone(),
            investigation_ticket: investigation_ticket.clone(),
            suspended_at: now,
            restore_at,
            restored_at: None,
            restored_by: None,
        };
        let mut timeline = env
            .storage()
            .persistent()
            .get::<_, Vec<VerificationSuspension>>(
                &ExtensionKey::ProjectVerificationSuspensions(project_id),
            )
            .unwrap_or_else(|| Vec::new(env));
        timeline.push_back(entry);
        env.storage().persistent().set(
            &ExtensionKey::ProjectVerificationSuspensions(project_id),
            &timeline,
        );
        project.verification_status = VerificationStatus::Suspended;
        project.updated_at = now;
        env.storage()
            .persistent()
            .set(&StorageKey::Project(project_id), &project);
        publish_verification_suspended_event(
            env,
            project_id,
            admin.clone(),
            reason.clone(),
            investigation_ticket,
            restore_at,
        );
        AdminActionLog::record_action(
            env,
            admin,
            AdminActionType::VerificationSuspended,
            Some(project_id),
            None,
            Some(reason),
        );
        Ok(())
    }

    pub fn restore_verification(
        env: &Env,
        project_id: u64,
        admin: Address,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;
        let mut project =
            ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;
        if project.verification_status != VerificationStatus::Suspended {
            return Err(ContractError::InvalidStatus);
        }
        let mut record = Self::get_verification(env, project_id)
            .ok_or(ContractError::VerificationNotFound)?;
        let now = env.ledger().timestamp();
        record.status = VerificationStatus::Verified;
        env.storage()
            .persistent()
            .set(&StorageKey::VerificationRecord(record.request_id), &record);
        let mut timeline = env
            .storage()
            .persistent()
            .get::<_, Vec<VerificationSuspension>>(
                &ExtensionKey::ProjectVerificationSuspensions(project_id),
            )
            .unwrap_or_else(|| Vec::new(env));
        if let Some(index) = timeline.len().checked_sub(1) {
            if let Some(mut entry) = timeline.get(index) {
                entry.restored_at = Some(now);
                entry.restored_by = Some(admin.clone());
                timeline.set(index, entry);
            }
        }
        env.storage().persistent().set(
            &ExtensionKey::ProjectVerificationSuspensions(project_id),
            &timeline,
        );
        project.verification_status = VerificationStatus::Verified;
        project.updated_at = now;
        env.storage()
            .persistent()
            .set(&StorageKey::Project(project_id), &project);
        publish_verification_restored_event(env, project_id, Some(admin.clone()));
        AdminActionLog::record_action(
            env,
            admin,
            AdminActionType::VerificationRestored,
            Some(project_id),
            None,
            None,
        );
        Ok(())
    }

    pub fn get_verification_suspension_timeline(
        env: &Env,
        project_id: u64,
    ) -> Vec<VerificationSuspension> {
        Self::restore_expired_suspension(env, project_id);
        env.storage()
            .persistent()
            .get(&ExtensionKey::ProjectVerificationSuspensions(project_id))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn get_verification_record(env: &Env, request_id: u64) -> Option<VerificationRecord> {
        env.storage()
            .persistent()
            .get::<_, VerificationRecord>(&StorageKey::VerificationRecord(request_id))
    }

    pub fn get_verification_evidence_versions(
        env: &Env,
        request_id: u64,
    ) -> Vec<VerificationEvidenceVersion> {
        env.storage()
            .persistent()
            .get::<_, Vec<VerificationEvidenceVersion>>(
                &ExtensionKey2::VerificationEvidenceVersions(request_id),
            )
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn get_verification_evidence_version(
        env: &Env,
        request_id: u64,
        version: u32,
    ) -> Option<VerificationEvidenceVersion> {
        let versions = Self::get_verification_evidence_versions(env, request_id);
        if version == 0 || version > versions.len() {
            return None;
        }
        versions.get(version - 1)
    }

    pub fn compare_verification_evidence(
        env: &Env,
        request_id: u64,
        first_version: u32,
        second_version: u32,
    ) -> Option<VerificationEvidenceComparison> {
        let first = Self::get_verification_evidence_version(env, request_id, first_version)?;
        let second = Self::get_verification_evidence_version(env, request_id, second_version)?;
        Some(VerificationEvidenceComparison {
            changed: first.evidence_cid != second.evidence_cid,
            first,
            second,
        })
    }

    pub fn get_pending_verifications(
        env: &Env,
        start: u32,
        limit: u32,
    ) -> Vec<VerificationRecord> {
        let pending_ids = env
            .storage()
            .persistent()
            .get::<_, Vec<u64>>(&ExtensionKey::PendingVerificationRequests)
            .unwrap_or_else(|| Vec::new(env));
        let page_ids = crate::pagination::paginate(env, &pending_ids, start, limit);
        let mut records = Vec::new(env);
        for i in 0..page_ids.len() {
            if let Some(request_id) = page_ids.get(i) {
                if let Some(record) = Self::get_verification_record(env, request_id) {
                    records.push_back(record);
                }
            }
        }
        records
    }

    fn remove_pending_request(env: &Env, request_id: u64) {
        let pending = env
            .storage()
            .persistent()
            .get::<_, Vec<u64>>(&ExtensionKey::PendingVerificationRequests)
            .unwrap_or_else(|| Vec::new(env));
        let updated = Utils::remove_item_from_vec(env, &pending, &request_id);
        if updated.is_empty() {
            env.storage()
                .persistent()
                .remove(&ExtensionKey::PendingVerificationRequests);
        } else {
            env.storage()
                .persistent()
                .set(&ExtensionKey::PendingVerificationRequests, &updated);
        }
    }

    /// Returns `true` if the project has a Verified record that has **not** yet expired.
    ///
    /// A record is considered active when:
    ///   1. `status == Verified`, **and**
    ///   2. `expires_at` is either `None` (legacy records without an expiry) **or**
    ///      `Some(t)` where `t > current_ledger_timestamp`.
    ///
    /// If the record is expired this also emits a `VerificationExpiredEvent` so that
    /// indexers can pick it up without needing a dedicated "check expiry" transaction.
    pub fn is_verification_active(env: &Env, project_id: u64) -> bool {
        // `StorageKey::Verification(project_id)` holds the *request id*, not the
        // record — the record lives under `VerificationRecord(request_id)`.
        // Reading it directly as a `VerificationRecord` raised a
        // `ConversionError` that escalated to a host panic, so this entry point
        // trapped for every project that had ever requested verification.
        // `Self::get_verification` already performs the correct two-hop lookup.
        let record: VerificationRecord = match Self::get_verification(env, project_id) {
            Some(r) => r,
            None => return false,
        };

        if record.status != VerificationStatus::Verified {
            return false;
        }

        if record.expires_at == 0 {
            // legacy / no-expiry record
            return true;
        }

        let now = env.ledger().timestamp();
        if now >= record.expires_at {
            // Emit expiry event so indexers can react
            publish_verification_expired_event(env, project_id, record.expires_at);
            false
        } else {
            true
        }
    }

    /// Batch-fetch verification records for multiple project IDs.
    /// Silently skips IDs with no record. Clamped to 100 entries.
    ///
    /// `StorageKey::Verification(project_id)` holds the *request id*, not the
    /// record, so this performs the same two-hop lookup as
    /// [`Self::get_verification`].
    pub fn get_verifications_batch(env: &Env, ids: Vec<u64>) -> Vec<(u64, VerificationRecord)> {
        const MAX_BATCH: u32 = 100;
        let len = core::cmp::min(ids.len(), MAX_BATCH);
        let mut out = Vec::new(env);
        for i in 0..len {
            if let Some(id) = ids.get(i) {
                if let Some(record) = Self::get_verification(env, id) {
                    out.push_back((id, record));
                }
            }
        }
        out
    }

    /// Batch-fetch verification records by verification request ID.
    /// Silently skips request IDs with no record. Clamped to 100 entries.
    pub fn get_verification_records_batch(
        env: &Env,
        request_ids: Vec<u64>,
    ) -> Vec<(u64, VerificationRecord)> {
        const MAX_BATCH: u32 = 100;
        let len = core::cmp::min(request_ids.len(), MAX_BATCH);
        let mut out = Vec::new(env);
        for i in 0..len {
            if let Some(request_id) = request_ids.get(i) {
                if let Some(record) = Self::get_verification_record(env, request_id) {
                    out.push_back((request_id, record));
                }
            }
        }
        out
    }

    /// Retrieve the complete verification request history for a project.
    pub fn get_verification_history(env: &Env, project_id: u64) -> Vec<VerificationRecord> {
        let mut out = Vec::new(env);
        if let Some(history) = env
            .storage()
            .persistent()
            .get::<_, Vec<u64>>(&StorageKey::ProjectVerificationHistory(project_id))
        {
            for i in 0..history.len() {
                if let Some(req_id) = history.get(i) {
                    if let Some(record) = env
                        .storage()
                        .persistent()
                        .get::<_, VerificationRecord>(&StorageKey::VerificationRecord(req_id))
                    {
                        out.push_back(record);
                    }
                }
            }
        }
        out
    }

    /// Admin: assign a pending verification request to a specific admin for review.
    pub fn assign_verification(
        env: &Env,
        project_id: u64,
        admin: Address,
        assignee: Address,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;

        // Assignee must also be an admin
        if !crate::admin_manager::AdminManager::is_admin(env, &assignee) {
            return Err(ContractError::AdminNotFound);
        }

        let mut record =
            Self::get_verification(env, project_id).ok_or(ContractError::VerificationNotFound)?;
        if record.status != VerificationStatus::Pending {
            return Err(ContractError::InvalidStatus);
        }

        record.assigned_admin = Some(assignee.clone());
        env.storage()
            .persistent()
            .set(&StorageKey::VerificationRecord(record.request_id), &record);

        crate::events::publish_verification_assigned_event(
            env,
            project_id,
            record.request_id,
            assignee.clone(),
            admin.clone(),
        );

        AdminActionLog::record_action(
            env,
            admin,
            AdminActionType::VerificationAssigned,
            Some(project_id),
            None,
            None,
        );

        Ok(())
    }

    /// Get the admin assigned to review a verification request.
    pub fn get_assigned_admin(env: &Env, project_id: u64) -> Option<Address> {
        let record = Self::get_verification(env, project_id)?;
        record.assigned_admin
    }

    /// Returns `true` if a verification record exists for `project_id`.
    ///
    /// This is a low-cost existence check (single storage `has` call) kept
    /// as a convenience for admin tooling and integration tests.  On-chain
    /// callers currently use `get_verification` directly and check `is_some()`,
    /// so this helper has no callers in production paths yet.
    // Dead-code justification: convenience helper for integration tests and
    // off-chain tooling; not yet called from on-chain paths.
    #[allow(dead_code)]
    pub fn verification_exists(env: &Env, project_id: u64) -> bool {
        env.storage()
            .persistent()
            .has(&StorageKey::ProjectVerificationHistory(project_id))
    }

    pub fn revoke_verification(
        env: &Env,
        project_id: u64,
        admin: Address,
        reason: String,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;

        if crate::admin_manager::AdminManager::get_admin_approval_threshold(env) > 1 {
            return Err(ContractError::Unauthorized);
        }

        let mut project =
            ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        if project.verification_status != VerificationStatus::Verified {
            return Err(ContractError::InvalidStatus);
        }

        let mut record =
            Self::get_verification(env, project_id).ok_or(ContractError::VerificationNotFound)?;

        let now = env.ledger().timestamp();

        record.status = VerificationStatus::Unverified;
        record.revoke_reason = Some(reason.clone());
        record.decided_at = now;
        env.storage()
            .persistent()
            .set(&StorageKey::Verification(project_id), &record.request_id);
        env.storage()
            .persistent()
            .set(&StorageKey::VerificationRecord(record.request_id), &record);

        project.verification_status = VerificationStatus::Unverified;
        project.current_verification_id = Some(record.request_id);
        project.updated_at = now;
        env.storage()
            .persistent()
            .set(&StorageKey::Project(project_id), &project);

        publish_verification_revoked_event(env, project_id, admin.clone(), reason.clone());
        crate::notification_registry::NotificationRegistry::emit_project_notification(
            env,
            project_id,
            crate::types::NotificationKind::VerificationRevoked,
        );

        AdminActionLog::record_action(
            env,
            admin,
            AdminActionType::VerificationRevoked,
            Some(project_id),
            None,
            Some(reason),
        );

        Ok(())
    }

    /// Get minimum project age configuration
    pub fn get_min_project_age(env: &Env) -> u64 {
        env.storage()
            .persistent()
            .get(&StorageKey::MinProjectAge)
            .unwrap_or(crate::constants::MIN_PROJECT_AGE_SECONDS)
    }

    /// Set minimum project age (admin only)
    pub fn set_min_project_age(
        env: &Env,
        admin: Address,
        min_age_seconds: u64,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;
        let previous_min_age_seconds = Self::get_min_project_age(env);
        env.storage()
            .persistent()
            .set(&StorageKey::MinProjectAge, &min_age_seconds);

        crate::events::publish_min_project_age_set_event(
            env,
            admin.clone(),
            previous_min_age_seconds,
            min_age_seconds,
        );

        AdminActionLog::record_action(
            env,
            admin,
            AdminActionType::MinProjectAgeSet,
            None,
            None,
            None,
        );

        Ok(())
    }

    /// Get verification validity duration configuration
    pub fn get_verification_duration(env: &Env) -> u64 {
        env.storage()
            .persistent()
            .get(&StorageKey::VerificationDuration)
            .unwrap_or(crate::constants::VERIFICATION_VALIDITY_PERIOD)
    }

    /// Set verification validity duration (admin only).
    ///
    /// This function is implemented but not yet wired to a `lib.rs` entry
    /// point.  It is kept here so that the getter/setter pair is complete
    /// and the setter can be exposed in a follow-up PR without touching this
    /// module again.
    // Dead-code justification: setter is implemented but not yet exposed via
    // lib.rs; will be wired in a follow-up PR adding the admin config endpoint.
    #[allow(dead_code)]
    pub fn set_verification_duration(
        env: &Env,
        admin: Address,
        duration_seconds: u64,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;
        let previous_duration_seconds = Self::get_verification_duration(env);
        env.storage()
            .persistent()
            .set(&StorageKey::VerificationDuration, &duration_seconds);

        crate::events::publish_verification_duration_set_event(
            env,
            admin.clone(),
            previous_duration_seconds,
            duration_seconds,
        );

        AdminActionLog::record_action(
            env,
            admin,
            AdminActionType::VerificationDurationSet,
            None,
            None,
            None,
        );

        Ok(())
    }

    /// Admin-driven direct renewal: reset the expiry of an already-verified project
    /// without requiring a pending renewal request.
    pub fn renew_verification(
        env: &Env,
        project_id: u64,
        admin: Address,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;

        let mut project =
            ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        if project.verification_status != VerificationStatus::Verified {
            return Err(ContractError::InvalidStatus);
        }

        let mut verification =
            Self::get_verification(env, project_id).ok_or(ContractError::VerificationNotFound)?;

        let now = env.ledger().timestamp();
        let duration = Self::get_verification_duration(env);
        let new_expires_at = now.saturating_add(duration);

        verification.expires_at = new_expires_at;
        verification.last_renewed_at = now;
        env.storage().persistent().set(
            &StorageKey::Verification(project_id),
            &verification.request_id,
        );
        env.storage().persistent().set(
            &StorageKey::VerificationRecord(verification.request_id),
            &verification,
        );

        project.updated_at = now;
        env.storage()
            .persistent()
            .set(&StorageKey::Project(project_id), &project);

        publish_verification_renewed_event(env, project_id, admin.clone(), new_expires_at);

        AdminActionLog::record_action(
            env,
            admin,
            AdminActionType::VerificationRenewalApproved,
            Some(project_id),
            None,
            None,
        );

        Ok(())
    }

    pub fn request_renewal(
        env: &Env,
        project_id: u64,
        requester: Address,
        evidence_cid: String,
    ) -> Result<(), ContractError> {
        let project =
            ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        require_owner_auth(&requester, &project.owner)?;
        if project.verification_status != VerificationStatus::Verified {
            return Err(ContractError::InvalidStatus);
        }
        if env
            .storage()
            .persistent()
            .has(&StorageKey::VerificationRenewal(project_id))
        {
            return Err(ContractError::InvalidStatus);
        }

        VerificationValidation::validate_evidence_cid(&evidence_cid)?;

        let fee_amount = match FeeManager::get_fee_config(env) {
            Ok(config) if config.verification_fee > 0 => {
                FeeManager::consume_fee_payment(
                    env,
                    project_id,
                    requester.clone(),
                    config.verification_fee,
                )?;
                config.verification_fee
            }
            Ok(config) => config.verification_fee,
            Err(_) => 0,
        };

        let now = env.ledger().timestamp();
        let renewal = VerificationRenewalRecord {
            project_id,
            requester: requester.clone(),
            status: VerificationStatus::Pending,
            evidence_cid: evidence_cid.clone(),
            timestamp: now,
            fee_amount,
            expires_at: now.saturating_add(Self::get_verification_duration(env)),
        };

        env.storage()
            .persistent()
            .set(&StorageKey::VerificationRenewal(project_id), &renewal);

        publish_verification_renewal_requested_event(
            env,
            project_id,
            requester,
            evidence_cid,
            fee_amount,
        );
        Ok(())
    }

    pub fn approve_renewal(
        env: &Env,
        project_id: u64,
        admin: Address,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;

        let renewal = Self::get_renewal_request(env, project_id)
            .ok_or(ContractError::VerificationNotFound)?;
        let mut verification =
            Self::get_verification(env, project_id).ok_or(ContractError::VerificationNotFound)?;
        let mut project =
            ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        let now = env.ledger().timestamp();
        let expires_at = now.saturating_add(Self::get_verification_duration(env));

        verification.status = VerificationStatus::Verified;
        verification.expires_at = expires_at;
        verification.last_renewed_at = now;
        env.storage().persistent().set(
            &StorageKey::Verification(project_id),
            &verification.request_id,
        );
        env.storage().persistent().set(
            &StorageKey::VerificationRecord(verification.request_id),
            &verification,
        );

        project.updated_at = now;
        project.current_verification_id = Some(verification.request_id);
        env.storage()
            .persistent()
            .set(&StorageKey::Project(project_id), &project);

        let history_index: u32 = env
            .storage()
            .persistent()
            .get(&StorageKey::VerificationRenewalCount(project_id))
            .unwrap_or(0);
        let approved = VerificationRenewalRecord {
            status: VerificationStatus::Verified,
            expires_at,
            ..renewal.clone()
        };
        env.storage().persistent().set(
            &StorageKey::VerificationRenewalHistory(project_id, history_index),
            &approved,
        );
        env.storage().persistent().set(
            &StorageKey::VerificationRenewalCount(project_id),
            &history_index.saturating_add(1),
        );
        env.storage()
            .persistent()
            .remove(&StorageKey::VerificationRenewal(project_id));

        publish_verification_renewal_approved_event(env, project_id, admin.clone(), expires_at);

        AdminActionLog::record_action(
            env,
            admin,
            AdminActionType::VerificationRenewalApproved,
            Some(project_id),
            None,
            None,
        );

        Ok(())
    }

    /// Directly renew an already-Verified verification without going through a
    pub fn reject_renewal(env: &Env, project_id: u64, admin: Address) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;
        let _renewal = Self::get_renewal_request(env, project_id)
            .ok_or(ContractError::VerificationNotFound)?;
        env.storage()
            .persistent()
            .remove(&StorageKey::VerificationRenewal(project_id));
        publish_verification_renewal_rejected_event(env, project_id, admin.clone());

        AdminActionLog::record_action(
            env,
            admin,
            AdminActionType::VerificationRenewalRejected,
            Some(project_id),
            None,
            None,
        );

        Ok(())
    }

    pub fn get_renewal_request(env: &Env, project_id: u64) -> Option<VerificationRenewalRecord> {
        env.storage()
            .persistent()
            .get(&StorageKey::VerificationRenewal(project_id))
    }

    pub fn get_renewal_history(
        env: &Env,
        project_id: u64,
        start_index: u32,
        limit: u32,
    ) -> Vec<VerificationRenewalRecord> {
        let effective_limit = if limit == 0 || limit > MAX_PAGE_LIMIT {
            MAX_PAGE_LIMIT
        } else {
            limit
        };

        let count: u32 = env
            .storage()
            .persistent()
            .get(&StorageKey::VerificationRenewalCount(project_id))
            .unwrap_or(0);

        let mut history = Vec::new(env);
        let end = core::cmp::min(start_index.saturating_add(effective_limit), count);
        for index in start_index..end {
            if let Some(record) = env
                .storage()
                .persistent()
                .get(&StorageKey::VerificationRenewalHistory(project_id, index))
            {
                history.push_back(record);
            }
        }
        history
    }

    pub fn is_verification_expired(env: &Env, project_id: u64) -> Result<bool, ContractError> {
        let verification =
            Self::get_verification(env, project_id).ok_or(ContractError::VerificationNotFound)?;
        Ok(verification.expires_at != 0 && env.ledger().timestamp() >= verification.expires_at)
    }

    /// Explicitly processes verification expiry for a project if its verification period has elapsed.
    ///
    /// If the project is currently `Verified` and `expires_at > 0` and `now >= expires_at`:
    /// - Updates `VerificationRecord.status` to `Unverified`
    /// - Updates `Project.verification_status` to `Unverified`
    /// - Publishes a `VerificationExpired` event
    ///
    /// Returns `Ok(true)` if expiry state transition occurred, or `Ok(false)` if not expired / not verified.
    pub fn process_verification_expiry(env: &Env, project_id: u64) -> Result<bool, ContractError> {
        let mut project =
            ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;
        let mut record =
            Self::get_verification(env, project_id).ok_or(ContractError::VerificationNotFound)?;

        if record.status != VerificationStatus::Verified || record.expires_at == 0 {
            return Ok(false);
        }

        let now = env.ledger().timestamp();
        if now >= record.expires_at {
            record.status = VerificationStatus::Unverified;
            env.storage()
                .persistent()
                .set(&StorageKey::VerificationRecord(record.request_id), &record);

            project.verification_status = VerificationStatus::Unverified;
            project.updated_at = now;
            env.storage()
                .persistent()
                .set(&StorageKey::Project(project_id), &project);

            publish_verification_expired_event(env, project_id, record.expires_at);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn is_verification_expiring_soon(
        env: &Env,
        project_id: u64,
        threshold_seconds: u64,
    ) -> Result<bool, ContractError> {
        let verification =
            Self::get_verification(env, project_id).ok_or(ContractError::VerificationNotFound)?;

        if verification.expires_at == 0 {
            return Ok(false);
        }

        let now = env.ledger().timestamp();
        if now > verification.expires_at {
            return Ok(false);
        }

        Ok(now.saturating_add(threshold_seconds) >= verification.expires_at)
    }

    /// Emit the owner reminder when expiry is within 30 days, or resend it
    /// after seven days when no renewal request has been opened.
    pub fn process_verification_expiry_notification(
        env: &Env,
        project_id: u64,
    ) -> Result<bool, ContractError> {
        let project =
            ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;
        let record = Self::get_verification(env, project_id)
            .ok_or(ContractError::VerificationNotFound)?;
        if record.status != VerificationStatus::Verified || record.expires_at == 0 {
            return Ok(false);
        }
        let now = env.ledger().timestamp();
        if now >= record.expires_at
            || now.saturating_add(crate::constants::VERIFICATION_EXPIRY_REMINDER_SECONDS)
                < record.expires_at
        {
            return Ok(false);
        }

        let key = NotificationKey::VerificationExpiryNotification(project_id);
        let mut notification = env
            .storage()
            .persistent()
            .get::<_, VerificationExpiryNotification>(&key);
        if env
            .storage()
            .persistent()
            .has(&StorageKey::VerificationRenewal(project_id))
        {
            if let Some(mut existing) = notification {
                existing.owner_responded = true;
                env.storage().persistent().set(&key, &existing);
            }
            return Ok(false);
        }

        let resend = if let Some(existing) = notification.as_ref() {
            if now
                < existing
                    .last_sent_at
                    .saturating_add(crate::constants::VERIFICATION_EXPIRY_RESEND_SECONDS)
            {
                return Ok(false);
            }
            true
        } else {
            false
        };
        let resend_count = notification
            .as_ref()
            .map(|existing| existing.resend_count.saturating_add(1))
            .unwrap_or(0);
        let instructions = soroban_sdk::String::from_str(
            env,
            "Renew with request_renewal(project_id, requester, evidence_cid) before expiry.",
        );
        let mut state = notification.take().unwrap_or(VerificationExpiryNotification {
            project_id,
            request_id: record.request_id,
            owner: project.owner.clone(),
            expires_at: record.expires_at,
            renewal_instructions: instructions.clone(),
            first_sent_at: now,
            last_sent_at: now,
            resend_count: 0,
            delivery_status: NotificationDeliveryStatus::Pending,
            owner_responded: false,
        });
        state.last_sent_at = now;
        state.resend_count = resend_count;
        state.delivery_status = NotificationDeliveryStatus::Pending;
        state.owner_responded = false;
        env.storage().persistent().set(&key, &state);
        publish_verification_expiry_notification_event(
            env,
            project_id,
            project.owner.clone(),
            record.expires_at,
            state.renewal_instructions,
            resend,
            resend_count,
        );
        crate::notification_registry::NotificationRegistry::emit_project_notification(
            env,
            project_id,
            crate::types::NotificationKind::VerificationExpiringSoon,
        );
        Ok(true)
    }

    pub fn get_verification_expiry_notification(
        env: &Env,
        project_id: u64,
    ) -> Option<VerificationExpiryNotification> {
        env.storage()
            .persistent()
            .get(&NotificationKey::VerificationExpiryNotification(project_id))
    }

    pub fn record_verification_expiry_notification_delivery(
        env: &Env,
        project_id: u64,
        admin: Address,
        delivered: bool,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;
        let key = NotificationKey::VerificationExpiryNotification(project_id);
        let mut notification = env
            .storage()
            .persistent()
            .get::<_, VerificationExpiryNotification>(&key)
            .ok_or(ContractError::VerificationNotFound)?;
        notification.delivery_status = if delivered {
            NotificationDeliveryStatus::Delivered
        } else {
            NotificationDeliveryStatus::Failed
        };
        env.storage().persistent().set(&key, &notification);
        Ok(())
    }

    /// Admin-only: prune verification history for a project, retaining only the
    /// most recent `keep_count` records. Pass `keep_count = 0` to remove all
    /// historical records (the live `Verification(project_id)` record is never removed).
    ///
    /// This frees storage for projects that have accumulated many verification
    /// requests (e.g. repeated rejection/re-submission cycles).
    pub fn clear_verification_history(
        env: &Env,
        project_id: u64,
        admin: &Address,
        keep_count: u32,
    ) -> Result<u32, ContractError> {
        // Auth: admin only
        if !crate::admin_manager::AdminManager::is_admin(env, admin) {
            return Err(ContractError::AdminOnly);
        }

        // Project must exist
        crate::project_registry::ProjectRegistry::get_project(env, project_id)
            .ok_or(ContractError::ProjectNotFound)?;

        let history_key = StorageKey::ProjectVerificationHistory(project_id);
        let history: Vec<u64> = env
            .storage()
            .persistent()
            .get(&history_key)
            .unwrap_or_else(|| Vec::new(env));

        let total = history.len();
        if total == 0 {
            // Nothing to prune
            return Ok(0);
        }

        // Determine how many to remove from the front (oldest entries)
        let keep = core::cmp::min(keep_count, total);
        let remove_count = total - keep;

        if remove_count == 0 {
            return Ok(0);
        }

        // Remove individual VerificationRecord entries for pruned request IDs
        for i in 0..remove_count {
            if let Some(req_id) = history.get(i) {
                env.storage()
                    .persistent()
                    .remove(&StorageKey::VerificationRecord(req_id));
            }
        }

        // Build the retained history (most recent `keep` entries)
        let mut retained = Vec::new(env);
        for i in remove_count..total {
            if let Some(req_id) = history.get(i) {
                retained.push_back(req_id);
            }
        }

        if retained.is_empty() {
            env.storage().persistent().remove(&history_key);
        } else {
            env.storage().persistent().set(&history_key, &retained);
        }

        crate::events::publish_verification_history_cleared_event(
            env,
            project_id,
            admin.clone(),
            remove_count,
            keep,
        );

        AdminActionLog::record_action(
            env,
            admin.clone(),
            AdminActionType::VerificationHistoryCleared,
            Some(project_id),
            None,
            None,
        );

        Ok(remove_count)
    }

    /// Admin-only: clear the renewal history for a project, freeing storage
    /// accumulated from repeated renewal cycles.
    /// Returns the number of renewal records removed.
    pub fn clear_renewal_history(
        env: &Env,
        project_id: u64,
        admin: &Address,
    ) -> Result<u32, ContractError> {
        // Auth: admin only
        if !crate::admin_manager::AdminManager::is_admin(env, admin) {
            return Err(ContractError::AdminOnly);
        }

        // Project must exist
        crate::project_registry::ProjectRegistry::get_project(env, project_id)
            .ok_or(ContractError::ProjectNotFound)?;

        let count: u32 = env
            .storage()
            .persistent()
            .get(&StorageKey::VerificationRenewalCount(project_id))
            .unwrap_or(0);

        if count == 0 {
            return Ok(0);
        }

        // Remove every individual renewal record
        for index in 0..count {
            env.storage()
                .persistent()
                .remove(&StorageKey::VerificationRenewalHistory(project_id, index));
        }

        // Reset the counter
        env.storage()
            .persistent()
            .remove(&StorageKey::VerificationRenewalCount(project_id));

        crate::events::publish_renewal_history_cleared_event(env, project_id, admin.clone(), count);

        AdminActionLog::record_action(
            env,
            admin.clone(),
            AdminActionType::RenewalHistoryCleared,
            Some(project_id),
            None,
            None,
        );

        Ok(count)
    }
}
