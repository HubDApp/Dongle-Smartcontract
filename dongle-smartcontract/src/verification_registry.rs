//! Verification requests with ownership and fee checks, and events.

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

#[allow(dead_code)]
impl VerificationRegistry {
    pub fn request_verification(
        env: &Env,
        project_id: u64,
        requester: Address,
        _evidence_cid: String,
    ) -> Result<(), ContractError> {
        requester.require_auth();

        // 1. Verify project exists and requester is owner
        let project = ProjectRegistry::get_project(env, project_id)
            .ok_or(ContractError::InvalidProjectData)?;
        if project.owner != requester {
            return Err(ContractError::Unauthorized);
        }

        // 2. Verify fee is paid
        if !FeeManager::is_fee_paid(env, project_id) {
            return Err(ContractError::InvalidProjectData); // Or a specific FeeNotPaid error if we had one
        }

        // 3. Store VerificationRecord with Pending
        let record = VerificationRecord {
            status: VerificationStatus::Pending,
        };
        env.storage()
            .persistent()
            .set(&StorageKey::Verification(project_id), &record);

        // 4. Update project status
        let mut updated_project = project;
        updated_project.verification_status = VerificationStatus::Pending;
        updated_project.updated_at = env.ledger().timestamp();
        env.storage()
            .persistent()
            .set(&StorageKey::Project(project_id), &updated_project);

        publish_verification_requested_event(env, project_id, requester);
        Ok(())
    }

    pub fn approve_verification(
        env: &Env,
        project_id: u64,
        admin: Address,
    ) -> Result<(), ContractError> {
        // 1. Authorize admin
        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&StorageKey::Admin)
            .ok_or(ContractError::Unauthorized)?;
        if admin != stored_admin {
            return Err(ContractError::Unauthorized);
        }
        admin.require_auth();

        // 2. Update status to Verified
        let mut project = ProjectRegistry::get_project(env, project_id)
            .ok_or(ContractError::InvalidProjectData)?;
        project.verification_status = VerificationStatus::Verified;
        project.updated_at = env.ledger().timestamp();
        env.storage()
            .persistent()
            .set(&StorageKey::Project(project_id), &project);

        let record = VerificationRecord {
            status: VerificationStatus::Verified,
        };
        env.storage()
            .persistent()
            .set(&StorageKey::Verification(project_id), &record);

        publish_verification_approved_event(env, project_id);
        Ok(())
    }

    pub fn reject_verification(
        env: &Env,
        project_id: u64,
        admin: Address,
    ) -> Result<(), ContractError> {
        // 1. Authorize admin
        let stored_admin: Address = env
            .storage()
            .persistent()
            .get(&StorageKey::Admin)
            .ok_or(ContractError::Unauthorized)?;
        if admin != stored_admin {
            return Err(ContractError::Unauthorized);
        }
        admin.require_auth();

        // 2. Update status to Rejected
        let mut project = ProjectRegistry::get_project(env, project_id)
            .ok_or(ContractError::InvalidProjectData)?;
        project.verification_status = VerificationStatus::Rejected;
        project.updated_at = env.ledger().timestamp();
        env.storage()
            .persistent()
            .set(&StorageKey::Project(project_id), &project);

        let record = VerificationRecord {
            status: VerificationStatus::Rejected,
        };
        env.storage()
            .persistent()
            .set(&StorageKey::Verification(project_id), &record);

        publish_verification_rejected_event(env, project_id);
        Ok(())
    }

    pub fn get_verification(
        env: &Env,
        project_id: u64,
    ) -> Result<VerificationRecord, ContractError> {
        env.storage()
            .persistent()
            .get(&StorageKey::Verification(project_id))
            .ok_or(ContractError::InvalidProjectData)
    }

    pub fn list_pending_verifications(
        env: &Env,
        _admin: Address,
        start_project_id: u64,
        limit: u32,
    ) -> Result<Vec<VerificationRecord>, ContractError> {
        // Simple implementation for now: iterate projects and collect pending
        let count: u64 = env
            .storage()
            .persistent()
            .get(&StorageKey::ProjectCount)
            .unwrap_or(0);
        let mut pending = Vec::new(env);
        let mut checked = 0;
        let mut current_id = start_project_id;

        while checked < limit && current_id <= count {
            if let Some(record) = env
                .storage()
                .persistent()
                .get::<_, VerificationRecord>(&StorageKey::Verification(current_id))
            {
                if record.status == VerificationStatus::Pending {
                    pending.push_back(record);
                    checked += 1;
                }
            }
            current_id += 1;
        }

        Ok(pending)
    }

    pub fn verification_exists(env: &Env, project_id: u64) -> bool {
        env.storage()
            .persistent()
            .has(&StorageKey::Verification(project_id))
    }

    pub fn get_verification_status(
        env: &Env,
        project_id: u64,
    ) -> Result<VerificationStatus, ContractError> {
        let record = Self::get_verification(env, project_id)?;
        Ok(record.status)
    }

    pub fn update_verification_evidence(
        _env: &Env,
        _project_id: u64,
        _requester: Address,
        _new_evidence_cid: String,
    ) -> Result<(), ContractError> {
        todo!("Verification evidence update logic not implemented")
    }

    pub fn validate_evidence_cid(evidence_cid: &String) -> Result<(), ContractError> {
        if evidence_cid.is_empty() {
            return Err(ContractError::InvalidProjectData);
        }
        Ok(())
    }

    pub fn get_verification_stats(_env: &Env) -> (u32, u32, u32) {
        (0, 0, 0)
    }
}
