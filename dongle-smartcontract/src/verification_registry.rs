//! Verification requests with ownership and fee checks, and events.

use crate::admin_manager::AdminManager;
use crate::errors::ContractError;
use crate::events::{
    publish_verification_approved_event, publish_verification_rejected_event,
    publish_verification_requested_event,
};
use crate::fee_manager::FeeManager;
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::StorageKey;
use crate::types::{VerificationRecord, VerificationStatus};
use soroban_sdk::{Address, Env, String, Vec};

pub struct VerificationRegistry;

impl VerificationRegistry {
    pub fn request_verification(
        env: &Env,
        project_id: u64,
        requester: Address,
        evidence_cid: String,
    ) -> Result<(), ContractError> {
        requester.require_auth();

        // 1. Validate project existence and ownership
        let mut project = ProjectRegistry::get_project(env, project_id)
            .ok_or(ContractError::ProjectNotFound)?;
        
        if project.owner != requester {
            return Err(ContractError::Unauthorized);
        }

        // 2. Check if already verified or pending
        if project.verification_status != VerificationStatus::Unverified && project.verification_status != VerificationStatus::Rejected {
             return Err(ContractError::InvalidStatusTransition);
        }

        // 3. Consume fee payment
        FeeManager::consume_fee_payment(env, project_id)?;

        // 4. Validate evidence
        Self::validate_evidence_cid(&evidence_cid)?;

        // 5. Create record
        let config = FeeManager::get_fee_config(env)?;
        let now = env.ledger().timestamp();
        let record = VerificationRecord {
            project_id,
            requester: requester.clone(),
            status: VerificationStatus::Pending,
            evidence_cid: evidence_cid.clone(),
            timestamp: now,
            fee_amount: config.verification_fee,
        };

        env.storage()
            .persistent()
            .set(&StorageKey::Verification(project_id), &record);

        // 6. Update project status to Pending
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
        admin.require_auth();
        AdminManager::require_admin(env, &admin)?;

        // Get project
        let mut project = ProjectRegistry::get_project(env, project_id)
            .ok_or(ContractError::ProjectNotFound)?;

        // Get verification record
        let mut record = Self::get_verification(env, project_id)?;

        if record.status != VerificationStatus::Pending {
            return Err(ContractError::InvalidStatusTransition);
        }

        let now = env.ledger().timestamp();

        // Update record
        record.status = VerificationStatus::Verified;
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
        admin.require_auth();
        AdminManager::require_admin(env, &admin)?;

        // Get project
        let mut project = ProjectRegistry::get_project(env, project_id)
            .ok_or(ContractError::ProjectNotFound)?;

        // Get verification record
        let mut record = Self::get_verification(env, project_id)?;

        if record.status != VerificationStatus::Pending {
            return Err(ContractError::InvalidStatusTransition);
        }

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

    pub fn validate_evidence_cid(evidence_cid: &String) -> Result<(), ContractError> {
        if evidence_cid.is_empty() {
            return Err(ContractError::InvalidProjectData);
        }
        Ok(())
    }

    pub fn verification_exists(env: &Env, project_id: u64) -> bool {
        env.storage()
            .persistent()
            .has(&StorageKey::Verification(project_id))
    }
}
