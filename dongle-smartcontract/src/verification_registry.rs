//! Verification requests with ownership and fee checks, events, and state machine.

use crate::auth::{require_admin_auth, require_owner_auth};
use crate::constants::{DEFAULT_VERIFICATION_DURATION_SECS, MAX_CID_LEN};
use crate::errors::ContractError;
use crate::events::{
    publish_verification_approved_event, publish_verification_rejected_event,
    publish_verification_requested_event, publish_verification_revoked_event,
    publish_verification_renewed_event,
};
use crate::fee_manager::FeeManager;
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::StorageKey;
use crate::types::{VerificationConfig, VerificationRecord, VerificationStatus};
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

        // 2. Check if project can request verification using state machine
        if !VerificationStateMachine::can_request_verification(project.verification_status) {
            return Err(ContractError::InvalidStatusTransition);
        }

        // 3. Validate state transition using centralized state machine
        VerificationStateMachine::validate_transition(
            project.verification_status,
            VerificationStatus::Pending,
        )?;

        // 4. Consume fee payment
        FeeManager::consume_fee_payment(env, project_id)?;

        // 5. Validate evidence
        Self::validate_evidence_cid(&evidence_cid)?;

        // 6. Create record
        let config = FeeManager::get_fee_config(env)?;
        let now = env.ledger().timestamp();
        let record = VerificationRecord {
            project_id,
            requester: requester.clone(),
            status: VerificationStatus::Pending,
            evidence_cid: evidence_cid.clone(),
            timestamp: now,
            fee_amount: config.verification_fee,
            revoke_reason: None,
            expires_at: None,
        };

        env.storage()
            .persistent()
            .set(&StorageKey::Verification(project_id), &record);

        // 7. Update project status to Pending
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

        // Compute expiry from configured duration (default 365 days)
        let duration = Self::get_verification_duration(env);
        let expires_at = duration.map(|d| now + d);

        // Update record
        record.status = VerificationStatus::Verified;
        record.expires_at = expires_at;
        env.storage()
            .persistent()
            .set(&StorageKey::Verification(project_id), &record);

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

    /// Returns true if the verification record exists, is Verified, and has not expired.
    pub fn is_verification_active(env: &Env, project_id: u64) -> bool {
        let record: VerificationRecord = match env
            .storage()
            .persistent()
            .get(&StorageKey::Verification(project_id))
        {
            Some(r) => r,
            None => return false,
        };
        if record.status != VerificationStatus::Verified {
            return false;
        }
        match record.expires_at {
            None => true,
            Some(exp) => env.ledger().timestamp() < exp,
        }
    }

    /// Admin: set the verification duration (seconds). Pass `None` to disable expiry.
    pub fn set_verification_duration(
        env: &Env,
        admin: Address,
        duration_secs: Option<u64>,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;
        let config = VerificationConfig { duration_secs };
        env.storage()
            .persistent()
            .set(&StorageKey::VerificationConfig, &config);
        Ok(())
    }

    /// Owner: renew an active (or expired) verification, extending `expires_at` from now.
    /// The project must currently be Verified (status). Requires a new fee payment.
    pub fn renew_verification(
        env: &Env,
        project_id: u64,
        caller: Address,
    ) -> Result<(), ContractError> {
        let project =
            ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;
        require_owner_auth(&caller, &project.owner)?;

        let mut record = Self::get_verification(env, project_id)?;
        if record.status != VerificationStatus::Verified {
            return Err(ContractError::InvalidStatusTransition);
        }

        FeeManager::consume_fee_payment(env, project_id)?;

        let now = env.ledger().timestamp();
        let duration = Self::get_verification_duration(env);
        record.expires_at = duration.map(|d| now + d);
        env.storage()
            .persistent()
            .set(&StorageKey::Verification(project_id), &record);

        publish_verification_renewed_event(env, project_id, caller, record.expires_at);
        Ok(())
    }

    /// Returns the configured duration in seconds, falling back to the default.
    pub fn get_verification_duration(env: &Env) -> Option<u64> {
        let config: Option<VerificationConfig> = env
            .storage()
            .persistent()
            .get(&StorageKey::VerificationConfig);
        match config {
            Some(c) => c.duration_secs,
            None => Some(DEFAULT_VERIFICATION_DURATION_SECS),
        }
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
            .has(&StorageKey::Verification(project_id))
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

        project.verification_status = VerificationStatus::Unverified;
        project.updated_at = now;
        env.storage()
            .persistent()
            .set(&StorageKey::Project(project_id), &project);

        publish_verification_revoked_event(env, project_id, admin, reason);
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
