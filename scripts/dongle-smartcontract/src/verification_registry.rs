//! Verification requests with ownership and fee checks, events, and state machine.

use crate::admin_action_log::AdminActionLog;
use crate::auth::{require_admin_auth, require_owner_auth};
use crate::constants::{MAX_CID_LEN, MAX_PAGE_LIMIT};
use crate::errors::ContractError;
use crate::events::{
    publish_verification_approved_event, publish_verification_evidence_updated_event,
    publish_verification_rejected_event, publish_verification_renewal_approved_event,
    publish_verification_renewal_rejected_event, publish_verification_renewal_requested_event,
    publish_verification_requested_event, publish_verification_revoked_event,
};
use crate::fee_manager::FeeManager;
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::{ExtensionKey, StorageKey};
use crate::types::{
    AdminActionType, VerificationRecord, VerificationRenewalRecord, VerificationStatus,
};
use crate::utils::Utils;
use soroban_sdk::{Address, Env, String, Vec};

/// Centralized verification state machine
pub struct VerificationStateMachine;

impl VerificationStateMachine {
    /// Validates if a state transition is allowed
    ///
    /// # Arguments
    /// * `current_status` - The current verification status
    /// * `target_status` - The desired verification status
    ///
    /// # Returns
    /// * `Ok(())` if the transition is valid
    /// * `Err(ContractError)` if the transition is invalid
    pub fn validate_transition(
        current_status: VerificationStatus,
        target_status: VerificationStatus,
    ) -> Result<(), ContractError> {
        match (current_status, target_status) {
            // Unverified -> Pending (verification request)
            (VerificationStatus::Unverified, VerificationStatus::Pending) => Ok(()),

            // Rejected -> Pending (re-request verification after rejection)
            (VerificationStatus::Rejected, VerificationStatus::Pending) => Ok(()),

            // Pending -> Verified (admin approval)
            (VerificationStatus::Pending, VerificationStatus::Verified) => Ok(()),

            // Pending -> Rejected (admin rejection)
            (VerificationStatus::Pending, VerificationStatus::Rejected) => Ok(()),

            // Verified -> Unverified (admin revocation)
            (VerificationStatus::Verified, VerificationStatus::Unverified) => Ok(()),

            // Same state (no change) - this should fail as it's not a valid transition
            (current, target) if current == target => Err(ContractError::InvalidStatus),

            // All other transitions are invalid
            (_from, _to) => Err(ContractError::InvalidStatus),
        }
    }

    /// Gets a descriptive error message for invalid transitions
    #[allow(dead_code)]
    fn get_transition_error_message(
        from: VerificationStatus,
        to: VerificationStatus,
    ) -> &'static str {
        match (from, to) {
            (VerificationStatus::Unverified, VerificationStatus::Verified) => {
                "Cannot verify directly from Unverified status. Must request verification first."
            }
            (VerificationStatus::Unverified, VerificationStatus::Rejected) => {
                "Cannot reject from Unverified status. Must request verification first."
            }
            (VerificationStatus::Pending, VerificationStatus::Unverified) => {
                "Cannot return to Unverified from Pending status."
            }
            (VerificationStatus::Verified, VerificationStatus::Pending) => {
                "Cannot request verification for already verified project."
            }
            (VerificationStatus::Verified, VerificationStatus::Rejected) => {
                "Cannot reject already verified project."
            }
            (VerificationStatus::Verified, VerificationStatus::Unverified) => {
                "Cannot unverify already verified project."
            }
            (VerificationStatus::Rejected, VerificationStatus::Verified) => {
                "Cannot verify directly from Rejected status. Must request verification again."
            }
            (VerificationStatus::Rejected, VerificationStatus::Unverified) => {
                "Cannot return to Unverified from Rejected status."
            }
            _ => "Invalid verification status transition.",
        }
    }

    /// Checks if a project can request verification based on its current status
    pub fn can_request_verification(status: VerificationStatus) -> bool {
        matches!(
            status,
            VerificationStatus::Unverified | VerificationStatus::Rejected
        )
    }

    /// Checks if a project can be approved based on its current status
    #[allow(dead_code)]
    pub fn can_be_approved(status: VerificationStatus) -> bool {
        matches!(status, VerificationStatus::Pending)
    }

    /// Checks if a project can be rejected based on its current status
    #[allow(dead_code)]
    pub fn can_be_rejected(status: VerificationStatus) -> bool {
        matches!(status, VerificationStatus::Pending)
    }

    /// Gets all possible next states from the current state
    #[allow(dead_code)]
    pub fn get_possible_next_states(
        env: &Env,
        status: VerificationStatus,
    ) -> Vec<VerificationStatus> {
        match status {
            VerificationStatus::Unverified => {
                let mut v = Vec::new(env);
                v.push_back(VerificationStatus::Pending);
                v
            }
            VerificationStatus::Pending => {
                let mut v = Vec::new(env);
                v.push_back(VerificationStatus::Verified);
                v.push_back(VerificationStatus::Rejected);
                v
            }
            VerificationStatus::Rejected => {
                let mut v = Vec::new(env);
                v.push_back(VerificationStatus::Pending);
                v
            }
            VerificationStatus::Verified => {
                let mut v = Vec::new(env);
                v.push_back(VerificationStatus::Unverified); // revocable by admin
                v
            }
        }
    }
}

pub struct VerificationRegistry;

impl VerificationRegistry {
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
        Self::validate_evidence_cid(&evidence_cid)?;

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

        // 8. Save to historical record
        env.storage()
            .persistent()
            .set(&StorageKey::VerificationRecord(request_id), &record);

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

        // 11. Update project status to Pending
        project.verification_status = VerificationStatus::Pending;
        project.current_verification_id = Some(request_id);
        project.updated_at = now;
        env.storage()
            .persistent()
            .set(&StorageKey::Project(project_id), &project);

        publish_verification_requested_event(env, project_id, requester, evidence_cid);
        Ok(())
    }

    /// Updates the verification evidence CID for a pending verification request.
    ///
    /// This can only be called by the project owner when the request is in the
    /// Pending status. The supplied CID is validated using the standard CID validation rules.
    /// Once updated successfully, it persists the new CID and publishes a
    /// `VerificationEvidenceUpdated` event.
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
        let mut record = Self::get_verification(env, project_id)?;

        // 3. Reject if not Pending
        if record.status != VerificationStatus::Pending {
            return Err(ContractError::InvalidStatus);
        }

        // 4. Validate CID before state mutation
        Self::validate_evidence_cid(&new_evidence_cid)?;

        // 5. Update CID and persist
        let old_evidence_cid = record.evidence_cid;
        record.evidence_cid = new_evidence_cid.clone();

        env.storage()
            .persistent()
            .set(&StorageKey::VerificationRecord(record.request_id), &record);

        // 6. Emit event
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
        let mut record = Self::get_verification(env, project_id)?;

        // Then validate state transition
        VerificationStateMachine::validate_transition(
            project.verification_status,
            VerificationStatus::Verified,
        )?;

        let now = env.ledger().timestamp();

        // Update record
        record.status = VerificationStatus::Verified;
        record.expires_at = now.saturating_add(Self::get_verification_duration(env));
        record.decided_at = now;
        env.storage()
            .persistent()
            .set(&StorageKey::Verification(project_id), &record.request_id);
        env.storage()
            .persistent()
            .set(&StorageKey::VerificationRecord(record.request_id), &record);

        // Update project
        project.verification_status = VerificationStatus::Verified;
        project.current_verification_id = Some(record.request_id);
        project.updated_at = now;
        env.storage()
            .persistent()
            .set(&StorageKey::Project(project_id), &project);

        publish_verification_approved_event(env, project_id, admin.clone(), now);

        AdminActionLog::record_action(
            env,
            admin,
            AdminActionType::VerificationApproved,
            Some(project_id),
            None,
            None,
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
        let mut record = Self::get_verification(env, project_id)?;

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

        publish_verification_rejected_event(env, project_id, admin.clone(), now);

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

    pub fn get_verification(
        env: &Env,
        project_id: u64,
    ) -> Result<VerificationRecord, ContractError> {
        let request_id = env
            .storage()
            .persistent()
            .get::<_, u64>(&StorageKey::Verification(project_id))
            .ok_or(ContractError::VerificationNotFound)?;
        env.storage()
            .persistent()
            .get::<_, VerificationRecord>(&StorageKey::VerificationRecord(request_id))
            .ok_or(ContractError::VerificationNotFound)
    }

    pub fn get_verification_record(
        env: &Env,
        request_id: u64,
    ) -> Result<VerificationRecord, ContractError> {
        env.storage()
            .persistent()
            .get::<_, VerificationRecord>(&StorageKey::VerificationRecord(request_id))
            .ok_or(ContractError::VerificationNotFound)
    }

    /// Batch-fetch verification records for multiple project IDs.
    /// Silently skips IDs with no record. Clamped to 100 entries.
    pub fn get_verifications_batch(env: &Env, ids: Vec<u64>) -> Vec<(u64, VerificationRecord)> {
        const MAX_PAGE_LIMIT: u32 = 100;
        let len = core::cmp::min(ids.len(), MAX_PAGE_LIMIT);
        let mut out = Vec::new(env);
        for i in 0..len {
            if let Some(id) = ids.get(i) {
                if let Some(record) = env
                    .storage()
                    .persistent()
                    .get(&StorageKey::Verification(id))
                {
                    out.push_back((id, record));
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

        let mut record = Self::get_verification(env, project_id)?;
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
    pub fn get_assigned_admin(
        env: &Env,
        project_id: u64,
    ) -> Result<Option<Address>, ContractError> {
        let record = Self::get_verification(env, project_id)?;
        Ok(record.assigned_admin)
    }

    pub fn validate_evidence_cid(evidence_cid: &String) -> Result<(), ContractError> {
        if evidence_cid.is_empty() {
            return Err(ContractError::InvalidProjectData);
        }
        if !Utils::is_valid_ipfs_cid(evidence_cid) || evidence_cid.len() as usize > MAX_CID_LEN {
            return Err(ContractError::InvalidProjectData);
        }
        Ok(())
    }

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

        let mut record = Self::get_verification(env, project_id)?;

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
            .get(&ExtensionKey::VerificationDuration)
            .unwrap_or(crate::constants::VERIFICATION_VALIDITY_PERIOD)
    }

    /// Set verification validity duration (admin only)
    pub fn set_verification_duration(
        env: &Env,
        admin: Address,
        duration_seconds: u64,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;
        let previous_duration_seconds = Self::get_verification_duration(env);
        env.storage()
            .persistent()
            .set(&ExtensionKey::VerificationDuration, &duration_seconds);

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

        Self::validate_evidence_cid(&evidence_cid)?;

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

        let renewal = Self::get_renewal_request(env, project_id)?;
        let mut verification = Self::get_verification(env, project_id)?;
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

    pub fn reject_renewal(env: &Env, project_id: u64, admin: Address) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;
        let _renewal = Self::get_renewal_request(env, project_id)?;
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

    pub fn get_renewal_request(
        env: &Env,
        project_id: u64,
    ) -> Result<VerificationRenewalRecord, ContractError> {
        env.storage()
            .persistent()
            .get(&StorageKey::VerificationRenewal(project_id))
            .ok_or(ContractError::VerificationNotFound)
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
        let verification = Self::get_verification(env, project_id)?;
        Ok(verification.expires_at != 0 && env.ledger().timestamp() > verification.expires_at)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_transitions() {
        // Unverified -> Pending
        assert!(VerificationStateMachine::validate_transition(
            VerificationStatus::Unverified,
            VerificationStatus::Pending
        )
        .is_ok());

        // Rejected -> Pending
        assert!(VerificationStateMachine::validate_transition(
            VerificationStatus::Rejected,
            VerificationStatus::Pending
        )
        .is_ok());

        // Pending -> Verified
        assert!(VerificationStateMachine::validate_transition(
            VerificationStatus::Pending,
            VerificationStatus::Verified
        )
        .is_ok());

        // Pending -> Rejected
        assert!(VerificationStateMachine::validate_transition(
            VerificationStatus::Pending,
            VerificationStatus::Rejected
        )
        .is_ok());
    }

    #[test]
    fn test_invalid_transitions() {
        // Unverified -> Verified
        assert!(VerificationStateMachine::validate_transition(
            VerificationStatus::Unverified,
            VerificationStatus::Verified
        )
        .is_err());

        // Unverified -> Rejected
        assert!(VerificationStateMachine::validate_transition(
            VerificationStatus::Unverified,
            VerificationStatus::Rejected
        )
        .is_err());

        // Verified -> Pending
        assert!(VerificationStateMachine::validate_transition(
            VerificationStatus::Verified,
            VerificationStatus::Pending
        )
        .is_err());

        // Verified -> Rejected
        assert!(VerificationStateMachine::validate_transition(
            VerificationStatus::Verified,
            VerificationStatus::Rejected
        )
        .is_err());
    }

    #[test]
    fn test_can_request_verification() {
        assert!(VerificationStateMachine::can_request_verification(
            VerificationStatus::Unverified
        ));
        assert!(VerificationStateMachine::can_request_verification(
            VerificationStatus::Rejected
        ));
        assert!(!VerificationStateMachine::can_request_verification(
            VerificationStatus::Pending
        ));
        assert!(!VerificationStateMachine::can_request_verification(
            VerificationStatus::Verified
        ));
    }

    #[test]
    fn test_can_be_approved() {
        assert!(VerificationStateMachine::can_be_approved(
            VerificationStatus::Pending
        ));
        assert!(!VerificationStateMachine::can_be_approved(
            VerificationStatus::Unverified
        ));
        assert!(!VerificationStateMachine::can_be_approved(
            VerificationStatus::Rejected
        ));
        assert!(!VerificationStateMachine::can_be_approved(
            VerificationStatus::Verified
        ));
    }

    #[test]
    fn test_can_be_rejected() {
        assert!(VerificationStateMachine::can_be_rejected(
            VerificationStatus::Pending
        ));
        assert!(!VerificationStateMachine::can_be_rejected(
            VerificationStatus::Unverified
        ));
        assert!(!VerificationStateMachine::can_be_rejected(
            VerificationStatus::Rejected
        ));
        assert!(!VerificationStateMachine::can_be_rejected(
            VerificationStatus::Verified
        ));
    }

    #[test]
    fn test_get_possible_next_states() {
        let env = Env::default();

        let unverified_states = VerificationStateMachine::get_possible_next_states(
            &env,
            VerificationStatus::Unverified,
        );
        assert_eq!(unverified_states.len(), 1);
        assert_eq!(
            unverified_states.get(0).unwrap(),
            VerificationStatus::Pending
        );

        let pending_states =
            VerificationStateMachine::get_possible_next_states(&env, VerificationStatus::Pending);
        assert_eq!(pending_states.len(), 2);
        assert_eq!(pending_states.get(0).unwrap(), VerificationStatus::Verified);
        assert_eq!(pending_states.get(1).unwrap(), VerificationStatus::Rejected);

        let rejected_states =
            VerificationStateMachine::get_possible_next_states(&env, VerificationStatus::Rejected);
        assert_eq!(rejected_states.len(), 1);
        assert_eq!(rejected_states.get(0).unwrap(), VerificationStatus::Pending);

        let verified_states =
            VerificationStateMachine::get_possible_next_states(&env, VerificationStatus::Verified);
        assert_eq!(verified_states.len(), 1);
        assert_eq!(
            verified_states.get(0).unwrap(),
            VerificationStatus::Unverified
        );
    }
}
