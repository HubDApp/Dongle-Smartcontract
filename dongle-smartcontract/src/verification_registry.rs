//! Verification requests with ownership and fee checks, and events.
use crate::constants::MAX_CID_LEN;
use crate::errors::ContractError;
use crate::events::VerificationApproved;
use crate::events::VerificationRejected;
use crate::events::VerificationRequested;
use crate::storage_keys::StorageKey;
use crate::types::{VerificationRecord, VerificationStatus};
use soroban_sdk::{Address, Env, String};

pub struct VerificationRegistry;

impl VerificationRegistry {
    /// Marks that the verification fee has been paid for a project (called by FeeManager).
    pub fn set_fee_paid(env: &Env, project_id: u64) {
        env.storage()
            .persistent()
            .set(&StorageKey::FeePaidForProject(project_id), &true);
    }

    fn fee_paid_for_project(env: &Env, project_id: u64) -> bool {
        env.storage()
            .persistent()
            .get(&StorageKey::FeePaidForProject(project_id))
            .unwrap_or(false)
    }

    pub fn request_verification(
        env: &Env,
        project_id: u64,
        requester: Address,
        evidence_cid: String,
    ) -> Result<(), ContractError> {
        requester.require_auth();

        if project_id == 0 {
            return Err(ContractError::InvalidProjectId);
        }

        // Soroban String has no trim()/is_empty() — use len() == 0
        if evidence_cid.len() == 0 || evidence_cid.len() as usize > MAX_CID_LEN {
            return Err(ContractError::InvalidEvidenceCid);
        }

        let project: crate::types::Project = env
            .storage()
            .persistent()
            .get(&StorageKey::Project(project_id))
            .ok_or(ContractError::ProjectNotFound)?;

        if project.owner != requester {
            return Err(ContractError::NotProjectOwnerForVerification);
        }

        if !Self::fee_paid_for_project(env, project_id) {
            return Err(ContractError::FeeNotPaid);
        }

        let ledger_timestamp = env.ledger().timestamp();

        let record = VerificationRecord {
            project_id,
            requester: requester.clone(),
            evidence_cid: evidence_cid.clone(),
            status: VerificationStatus::Pending,
            requested_at: ledger_timestamp,
            decided_at: None,
        };

        env.storage()
            .persistent()
            .set(&StorageKey::Verification(project_id), &record);

        VerificationRequested {
            project_id,
            requester: requester.clone(),
            evidence_cid,
        }
        .publish(env);

        Ok(())
    }

    pub fn approve_verification(
        env: &Env,
        project_id: u64,
        verifier: Address,
    ) -> Result<(), ContractError> {
        verifier.require_auth();

        if !Self::is_admin(env, &verifier) {
            return Err(ContractError::UnauthorizedVerifier);
        }

        let key = StorageKey::Verification(project_id);
        let mut record: VerificationRecord = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(ContractError::VerificationNotFound)?;

        if record.status != VerificationStatus::Pending {
            return Err(ContractError::VerificationNotPending);
        }

        let ledger_timestamp = env.ledger().timestamp();
        record.status = VerificationStatus::Verified;
        record.decided_at = Some(ledger_timestamp);

        env.storage().persistent().set(&key, &record);

        VerificationApproved {
            project_id,
            verifier,
        }
        .publish(env);

        Ok(())
    }

    pub fn reject_verification(
        env: &Env,
        project_id: u64,
        verifier: Address,
    ) -> Result<(), ContractError> {
        verifier.require_auth();

        if !Self::is_admin(env, &verifier) {
            return Err(ContractError::UnauthorizedVerifier);
        }

        let key = StorageKey::Verification(project_id);
        let mut record: VerificationRecord = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(ContractError::VerificationNotFound)?;

        if record.status != VerificationStatus::Pending {
            return Err(ContractError::VerificationNotPending);
        }

        let ledger_timestamp = env.ledger().timestamp();
        record.status = VerificationStatus::Rejected;
        record.decided_at = Some(ledger_timestamp);

        env.storage().persistent().set(&key, &record);

        VerificationRejected {
            project_id,
            verifier,
        }
        .publish(env);

        Ok(())
    }

    pub fn get_verification_stats(_env: &Env) -> (u32, u32, u32) {
        (0, 0, 0)
    }
}
