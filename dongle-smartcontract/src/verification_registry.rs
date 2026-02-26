//! Verification requests with ownership and fee checks, and events.

use crate::admin_manager::AdminManager;
use crate::errors::ContractError;
use crate::events::{publish_verification_approved_event, publish_verification_rejected_event};
use crate::types::{DataKey, VerificationRecord, VerificationStatus};
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
        evidence_cid: String,
    ) -> Result<(), ContractError> {
        requester.require_auth();

        // 1. Validate project ownership
        let project = crate::project_registry::ProjectRegistry::get_project(env, project_id)
            .ok_or(ContractError::ProjectNotFound)?;
        if project.owner != requester {
            return Err(ContractError::Unauthorized);
        }

        // 2. Consume fee payment
        crate::fee_manager::FeeManager::consume_fee_payment(env, project_id)?;

        // 3. Validate evidence
        Self::validate_evidence_cid(&evidence_cid)?;

        // 4. Create record
        let config = crate::fee_manager::FeeManager::get_fee_config(env)?;
        let record = VerificationRecord {
            project_id,
            requester: requester.clone(),
            status: VerificationStatus::Pending,
            evidence_cid: evidence_cid.clone(),
            timestamp: env.ledger().timestamp(),
            fee_amount: config.verification_fee,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Verification(project_id), &record);

        // 5. Update project status to Pending
        let mut mut_project = project;
        mut_project.verification_status = VerificationStatus::Pending;
        env.storage()
            .persistent()
            .set(&DataKey::Project(project_id), &mut_project);

        publish_verification_requested_event(env, project_id, requester, evidence_cid);
        Ok(())
    }

    pub fn approve_verification(
        env: &Env,
        project_id: u64,
        admin: Address,
    ) -> Result<(), ContractError> {
        admin.require_auth();

        // Verify admin privileges
        AdminManager::require_admin(env, &admin)?;

        // Get verification record
        let mut record: VerificationRecord = env
            .storage()
            .persistent()
            .get(&DataKey::Verification(project_id))
            .ok_or(ContractError::VerificationNotFound)?;

        if record.status != VerificationStatus::Pending {
            return Err(ContractError::InvalidStatusTransition);
        }

        record.status = VerificationStatus::Verified;
        env.storage()
            .persistent()
            .set(&DataKey::Verification(project_id), &record);

// 1. SECURITY: Authorize the admin (From Incoming)
let stored_admin: Address = env
    .storage()
    .persistent()
    .get(&StorageKey::Admin)
    .ok_or(ContractError::Unauthorized)?;

admin.require_auth(); 

// 2. LOGIC: Update the project (Hybrid)
let mut project = crate::project_registry::ProjectRegistry::get_project(env, project_id)
    .ok_or(ContractError::ProjectNotFound)?;

project.verification_status = VerificationStatus::Verified;
project.updated_at = env.ledger().timestamp(); // Keeps your data current

env.storage()
    .persistent()
    .set(&DataKey::Project(project_id), &project);

// 3. STATE: Create the separate verification record (From Incoming)
let record = VerificationRecord {
    status: VerificationStatus::Verified,
};
env.storage()
    .persistent()
    .set(&StorageKey::Verification(project_id), &record);

// 4. EVENT: Notify the network
publish_verification_approved_event(env, project_id, admin);
        Ok(())
    }

    pub fn reject_verification(
        env: &Env,
        project_id: u64,
        admin: Address,
    ) -> Result<(), ContractError> {
        admin.require_auth();

// Verify admin privileges
AdminManager::require_admin(env, &admin)?;

// Get verification record
let mut record: VerificationRecord = env
    .storage()
    .persistent()
    .get(&StorageKey::Verification(project_id)) // Ensure this matches your key type
    .ok_or(ContractError::RecordNotFound)?;
            .storage()
            .persistent()
            .get(&DataKey::Verification(project_id))
            .ok_or(ContractError::VerificationNotFound)?;

// Check if already processed
if record.status != VerificationStatus::Pending {
    return Err(ContractError::InvalidStatusTransition);
}

// Update status
record.status = VerificationStatus::Rejected;ected;
        env.storage()
            .persistent()
            .set(&DataKey::Verification(project_id), &record);

// 1. Authorize Admin (From Incoming)
let stored_admin: Address = env
    .storage()
    .persistent()
    .get(&StorageKey::Admin)
    .ok_or(ContractError::Unauthorized)?;

if admin != stored_admin {
    return Err(ContractError::Unauthorized);
}
admin.require_auth(); // This is the most critical security line

// 2. Update Project Status to Rejected
let mut project = crate::project_registry::ProjectRegistry::get_project(env, project_id)
    .ok_or(ContractError::ProjectNotFound)?;

project.verification_status = VerificationStatus::Rejected;
project.updated_at = env.ledger().timestamp(); // Keeps your data history accurate

env.storage()
    .persistent()
    .set(&DataKey::Project(project_id), &project);

// 3. Update the Verification Record (New required state)
let record = VerificationRecord {
    status: VerificationStatus::Rejected,
};
env.storage()
    .persistent()
    .set(&StorageKey::Verification(project_id), &record);

// 4. Emit the Event exactly once
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
.ok_or(ContractError::VerificationNotFound)
    }

    #[allow(dead_code)]
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

    #[allow(dead_code)]
    pub fn verification_exists(_env: &Env, _project_id: u64) -> bool {
        false
    pub fn verification_exists(env: &Env, project_id: u64) -> bool {
        env.storage()
            .persistent()
            .has(&StorageKey::Verification(project_id))
    }

    #[allow(dead_code)]
    pub fn get_verification_status(
        env: &Env,
        project_id: u64,
    ) -> Result<VerificationStatus, ContractError> {
        let record = Self::get_verification(env, project_id)?;
        Ok(record.status)
    }

    #[allow(dead_code)]
    pub fn update_verification_evidence(
        _env: &Env,
        _project_id: u64,
        _requester: Address,
        _new_evidence_cid: String,
    ) -> Result<(), ContractError> {
        todo!("Verification evidence update logic not implemented")
    }

    #[allow(dead_code)]
    pub fn validate_evidence_cid(evidence_cid: &String) -> Result<(), ContractError> {
        if evidence_cid.is_empty() {
            return Err(ContractError::InvalidProjectData);
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_verification_stats(_env: &Env) -> (u32, u32, u32) {
        (0, 0, 0)
    }
}
