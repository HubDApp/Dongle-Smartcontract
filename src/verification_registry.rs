//! Verification requests with ownership and fee checks, and events.

use crate::constants::MAX_CID_LEN;
use crate::errors::Error;
use crate::events::VerificationApproved;
use crate::events::VerificationRejected;
use crate::events::VerificationRequested;
use crate::storage_keys::StorageKey;
use crate::types::{VerificationRecord, VerificationStatus};
use soroban_sdk::{Address, Env, String as SorobanString};

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
        evidence_cid: SorobanString,
    ) -> Result<(), Error> {
        if project_id == 0 {
            return Err(Error::InvalidProjectId);
        }
        if evidence_cid.is_empty() || evidence_cid.len() > MAX_CID_LEN as u32 {
            return Err(Error::InvalidEvidenceCid);
        }

        let project_key = StorageKey::Project(project_id);
        let project: crate::types::Project = env
            .storage()
            .persistent()
            .get(&project_key)
            .ok_or(Error::ProjectNotFound)?;

        if project.owner != requester {
            return Err(Error::NotProjectOwnerForVerification);
        }

        if !Self::fee_paid_for_project(env, project_id) {
            return Err(Error::FeeNotPaid);
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
    ) -> Result<(), Error> {
        if !Self::is_admin(env, &verifier) {
            return Err(Error::UnauthorizedVerifier);
        }

        let key = StorageKey::Verification(project_id);
        let mut record: VerificationRecord = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(Error::VerificationNotFound)?;

        if record.status != VerificationStatus::Pending {
            // Use Error:: alias — identical to ContractError::, avoids unused-import warning.
            return Err(Error::VerificationNotPending);
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

    pub fn reject_verification(env: &Env, project_id: u64, verifier: Address) -> Result<(), Error> {
        if !Self::is_admin(env, &verifier) {
            return Err(Error::UnauthorizedVerifier);
        }

        let key = StorageKey::Verification(project_id);
        let mut record: VerificationRecord = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(Error::VerificationNotFound)?;

        if record.status != VerificationStatus::Pending {
            return Err(Error::VerificationNotPending);
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

    fn is_admin(env: &Env, addr: &Address) -> bool {
        let admin: Option<Address> = env.storage().persistent().get(&StorageKey::Admin);
        admin.as_ref() == Some(addr)
    }

    pub fn get_verification(env: &Env, project_id: u64) -> Option<VerificationRecord> {
        env.storage()
            .persistent()
            .get(&StorageKey::Verification(project_id))
    }
}
