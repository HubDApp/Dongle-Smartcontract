use crate::errors::ContractError;
use crate::events::{
    publish_verification_approved_event, publish_verification_rejected_event,
    publish_verification_requested_event,
};
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::StorageKey;
use crate::types::{VerificationRecord, VerificationStatus};
use soroban_sdk::{Address, Env, String};
use crate::types::{VerificationRecord, VerificationStatus, DataKey};
use soroban_sdk::{Address, Env, String, Vec};

pub struct VerificationRegistry;

impl VerificationRegistry {
    pub fn request_verification(
        env: &Env,
        project_id: u64,
        requester: Address,
        evidence_cid: String,
    ) {
        let record = VerificationRecord {
            project_id,
            status: VerificationStatus::Pending,
            requester: requester.clone(),
            evidence_cid,
            timestamp: env.ledger().timestamp(),
        };

        env.storage()
            .persistent()
            .set(&StorageKey::Verification(project_id), &record);

        publish_verification_requested_event(env, project_id, requester);
    }

    pub fn approve_verification(
        env: &Env,
        project_id: u64,
        _admin: Address,
    ) -> Result<(), ContractError> {
        let mut record = Self::get_verification(env, project_id)?;
        
        if record.status != VerificationStatus::Pending {
            return Err(ContractError::VerificationAlreadyProcessed);
        }

        record.status = VerificationStatus::Verified;
        env.storage()
            .persistent()
            .set(&StorageKey::Verification(project_id), &record);

        publish_verification_approved_event(env, project_id);
        
        // Also update project status to keep records in sync
        ProjectRegistry::update_verification_status(env, project_id, VerificationStatus::Verified)?;
        
        Ok(())
    }

    pub fn reject_verification(
        env: &Env,
        project_id: u64,
        _admin: Address,
    ) -> Result<(), ContractError> {
        let mut record = Self::get_verification(env, project_id)?;
        
        if record.status != VerificationStatus::Pending {
            return Err(ContractError::VerificationAlreadyProcessed);
        }

        record.status = VerificationStatus::Rejected;
        env.storage()
            .persistent()
            .set(&StorageKey::Verification(project_id), &record);

        publish_verification_rejected_event(env, project_id);
        
        // Also update project status to keep records in sync
        ProjectRegistry::update_verification_status(env, project_id, VerificationStatus::Rejected)?;
        
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

    pub fn verification_exists(env: &Env, project_id: u64) -> bool {
        env.storage()
            .persistent()
            .has(&StorageKey::Verification(project_id))
    }
}
