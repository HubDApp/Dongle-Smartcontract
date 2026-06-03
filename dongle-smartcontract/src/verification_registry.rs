//! Verification requests with ownership and fee checks, events, and state machine.

use crate::auth::{require_admin_auth, require_owner_auth};
use crate::constants::MAX_CID_LEN;
use crate::errors::ContractError;
use crate::events::{
    publish_verification_approved_event, publish_verification_rejected_event,
    publish_verification_requested_event, publish_verification_revoked_event,
};
use crate::fee_manager::FeeManager;
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::StorageKey;
use crate::types::{VerificationRecord, VerificationRenewalRecord, VerificationStatus};
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
            (current, target) if current == target => Err(ContractError::InvalidStatusTransition),

            // All other transitions are invalid
            (_from, _to) => Err(ContractError::InvalidStatusTransition),
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
            return Err(ContractError::InvalidStatusTransition);
        }

        // 4. Validate state transition using centralized state machine
        VerificationStateMachine::validate_transition(
            project.verification_status,
            VerificationStatus::Pending,
        )?;

        // 5. Consume fee payment
        FeeManager::consume_fee_payment(env, project_id)?;

        // 6. Validate evidence
        Self::validate_evidence_cid(&evidence_cid)?;

        // 6. Generate a unique request ID
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
        let config = FeeManager::get_fee_config(env)?;
        let now = env.ledger().timestamp();
        let record = VerificationRecord {
            request_id,
            project_id,
            requester: requester.clone(),
            status: VerificationStatus::Pending,
            evidence_cid: evidence_cid.clone(),
            timestamp: now,
            fee_amount: config.verification_fee,
            revoke_reason: None,
            expires_at: 0,
            last_renewed_at: 0,
        };

        // 8. Save to historical record
        env.storage()
            .persistent()
            .set(&StorageKey::VerificationRecord(request_id), &record);

        // 9. Save to current/latest backward-compatible record
        env.storage()
            .persistent()
            .set(&StorageKey::Verification(project_id), &record);

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
        project.updated_at = now;
        env.storage()
            .persistent()
            .set(&StorageKey::Project(project_id), &project);

        publish_verification_requested_event(env, project_id, requester, evidence_cid);
        Ok(())
    }

    pub fn approve_verification(
        env: &Env,
        project_id: u64,
        admin: Address,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;

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
        env.storage()
            .persistent()
            .set(&StorageKey::Verification(project_id), &record);
        env.storage()
            .persistent()
            .set(&StorageKey::VerificationRecord(record.request_id), &record);

        // Update project
        project.verification_status = VerificationStatus::Verified;
        project.updated_at = now;
        env.storage()
            .persistent()
            .set(&StorageKey::Project(project_id), &project);

        publish_verification_approved_event(env, project_id, admin);
        Ok(())
    }

    pub fn reject_verification(
        env: &Env,
        project_id: u64,
        admin: Address,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;

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
        env.storage()
            .persistent()
            .set(&StorageKey::Verification(project_id), &record);
        env.storage()
            .persistent()
            .set(&StorageKey::VerificationRecord(record.request_id), &record);

        // Update project
        project.verification_status = VerificationStatus::Rejected;
        project.updated_at = now;
        env.storage()
            .persistent()
            .set(&StorageKey::Project(project_id), &project);

        publish_verification_rejected_event(env, project_id, admin);
        Ok(())
    }

    pub fn get_verification(
        env: &Env,
        project_id: u64,
    ) -> Result<VerificationRecord, ContractError> {
        env.storage()
            .persistent()
            .get(&StorageKey::Verification(project_id))
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

        let mut project =
            ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        if project.verification_status != VerificationStatus::Verified {
            return Err(ContractError::VerificationNotRevocable);
        }

        let mut record = Self::get_verification(env, project_id)?;

        let now = env.ledger().timestamp();

        record.status = VerificationStatus::Unverified;
        record.revoke_reason = Some(reason.clone());
        env.storage()
            .persistent()
            .set(&StorageKey::Verification(project_id), &record);
        env.storage()
            .persistent()
            .set(&StorageKey::VerificationRecord(record.request_id), &record);

        project.verification_status = VerificationStatus::Unverified;
        project.updated_at = now;
        env.storage()
            .persistent()
            .set(&StorageKey::Project(project_id), &project);

        publish_verification_revoked_event(env, project_id, admin, reason);
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

        env.storage()
            .persistent()
            .set(&StorageKey::MinProjectAge, &min_age_seconds);

        crate::events::publish_min_project_age_set_event(env, min_age_seconds, admin);
        Ok(())
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
