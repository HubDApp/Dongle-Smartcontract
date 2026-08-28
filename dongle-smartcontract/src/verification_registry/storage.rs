//! Verification registry storage mutations: request/approve/reject, renewal, and assignment.

use crate::admin_action_log::AdminActionLog;
use crate::admin_manager::AdminManager;
use crate::auth::{require_admin_auth, require_owner_auth};
use crate::constants::MAX_PAGE_LIMIT;
use crate::errors::ContractError;
use crate::events::{
    publish_verification_approved_event, publish_verification_evidence_updated_event,
    publish_verification_expired_event, publish_verification_rejected_event,
    publish_verification_renewal_approved_event, publish_verification_renewal_rejected_event,
    publish_verification_renewal_requested_event, publish_verification_renewed_event,
    publish_verification_requested_event, publish_verification_revoked_event,
};
use crate::fee_manager::FeeManager;
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::{ExtensionKey, StorageKey};
use crate::types::{
    AdminActionType, VerificationRecord, VerificationRenewalRecord, VerificationStatus,
};
use crate::utils::Utils;
use crate::verification_registry::state_machine::VerificationStateMachine;
use crate::verification_registry::validation::VerificationValidation;
use soroban_sdk::{Address, Env, String, Vec};

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

        let mut pending = env
            .storage()
            .persistent()
            .get::<_, Vec<u64>>(&ExtensionKey::PendingVerificationRequests)
            .unwrap_or_else(|| Vec::new(env));
        Utils::add_unique_to_vec(&mut pending, &request_id);
        env.storage()
            .persistent()
            .set(&ExtensionKey::PendingVerificationRequests, &pending);

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
        let mut record =
            Self::get_verification(env, project_id).ok_or(ContractError::VerificationNotFound)?;

        // 3. Reject if not Pending
        if record.status != VerificationStatus::Pending {
            return Err(ContractError::InvalidStatus);
        }

        // 4. Validate CID before state mutation
        VerificationValidation::validate_evidence_cid(&new_evidence_cid)?;

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
        let mut record =
            Self::get_verification(env, project_id).ok_or(ContractError::VerificationNotFound)?;

        // Verify integrity hash: ensure project metadata (name, slug, category,
        // description) has not changed since the hash was last written by
        // register_project or update_project.  Recompute using the same
        // pipe-separated SHA-256 scheme and compare byte-for-byte.
        if let Some(stored_hash) = ProjectRegistry::get_project_integrity_hash(env, project_id) {
            let recomputed = ProjectRegistry::compute_integrity_hash(
                env,
                &project.name,
                &project.slug,
                &project.category,
                &project.description,
            );
            if recomputed != stored_hash {
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

    pub fn get_verification(env: &Env, project_id: u64) -> Option<VerificationRecord> {
        let request_id = env
            .storage()
            .persistent()
            .get::<_, u64>(&StorageKey::Verification(project_id))?;
        env.storage()
            .persistent()
            .get::<_, VerificationRecord>(&StorageKey::VerificationRecord(request_id))
    }

    pub fn get_verification_record(env: &Env, request_id: u64) -> Option<VerificationRecord> {
        env.storage()
            .persistent()
            .get::<_, VerificationRecord>(&StorageKey::VerificationRecord(request_id))
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
    /// pending renewal request. Extends `expires_at` by the configured
    /// verification duration and records the renewal timestamp.
    pub fn renew_verification(
        env: &Env,
        project_id: u64,
        admin: Address,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;

        // Project must exist
        ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        // Record must exist
        let mut record =
            Self::get_verification(env, project_id).ok_or(ContractError::VerificationNotFound)?;

        // Can only renew an already-Verified record (not Pending / Rejected / Unverified)
        if record.status != VerificationStatus::Verified {
            return Err(ContractError::InvalidStatusTransition);
        }

        let now = env.ledger().timestamp();
        let duration = AdminManager::get_verification_duration(env);
        let new_expires_at = now.saturating_add(duration);

        record.expires_at = new_expires_at;
        record.last_renewed_at = now;
        env.storage()
            .persistent()
            .set(&StorageKey::Verification(project_id), &record.request_id);
        env.storage()
            .persistent()
            .set(&StorageKey::VerificationRecord(record.request_id), &record);

        publish_verification_renewed_event(env, project_id, admin, new_expires_at);
        Ok(())
    }

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
        Ok(verification.expires_at != 0 && env.ledger().timestamp() > verification.expires_at)
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

    /// Batch-fetch verification records by request ID.
    /// Silently skips IDs with no record. Clamped to 100 entries.
    pub fn get_verification_records_batch(env: &Env, request_ids: Vec<u64>) -> Vec<(u64, VerificationRecord)> {
        const MAX_BATCH: u32 = 100;
        let len = core::cmp::min(request_ids.len(), MAX_BATCH);
        let mut out = Vec::new(env);
        for i in 0..len {
            if let Some(id) = request_ids.get(i) {
                if let Some(record) = env
                    .storage()
                    .persistent()
                    .get::<_, VerificationRecord>(&StorageKey::VerificationRecord(id))
                {
                    out.push_back((id, record));
                }
            }
        }
        out
    }
}
