//! Verification requests with ownership and fee checks, and events.

use crate::errors::ContractError;
use crate::events::{
    publish_verification_approved_event, publish_verification_rejected_event,
    publish_verification_requested_event,
};
use crate::types::{DataKey, VerificationRecord, VerificationStatus};
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

        // Check admin
        if !env.storage().persistent().has(&DataKey::Admin(admin.clone())) {
            return Err(ContractError::AdminOnly);
        }

        let mut record = env
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

        // Update project
        let mut project = crate::project_registry::ProjectRegistry::get_project(env, project_id)
            .ok_or(ContractError::ProjectNotFound)?;
        project.verification_status = VerificationStatus::Verified;
        env.storage()
            .persistent()
            .set(&DataKey::Project(project_id), &project);

        publish_verification_approved_event(env, project_id, admin);
        Ok(())
    }

    pub fn reject_verification(
        env: &Env,
        project_id: u64,
        admin: Address,
    ) -> Result<(), ContractError> {
        admin.require_auth();

        // Check admin
        if !env.storage().persistent().has(&DataKey::Admin(admin.clone())) {
            return Err(ContractError::AdminOnly);
        }

        let mut record = env
            .storage()
            .persistent()
            .get(&DataKey::Verification(project_id))
            .ok_or(ContractError::VerificationNotFound)?;

        if record.status != VerificationStatus::Pending {
            return Err(ContractError::InvalidStatusTransition);
        }

        record.status = VerificationStatus::Rejected;
        env.storage()
            .persistent()
            .set(&DataKey::Verification(project_id), &record);

        // Update project
        let mut project = crate::project_registry::ProjectRegistry::get_project(env, project_id)
            .ok_or(ContractError::ProjectNotFound)?;
        project.verification_status = VerificationStatus::Rejected;
        env.storage()
            .persistent()
            .set(&DataKey::Project(project_id), &project);

        publish_verification_rejected_event(env, project_id, admin);
        Ok(())
    }

    pub fn get_verification(
        env: &Env,
        project_id: u64,
    ) -> Result<VerificationRecord, ContractError> {
        env.storage()
            .persistent()
            .get(&DataKey::Verification(project_id))
            .ok_or(ContractError::VerificationNotFound)
    }

    pub fn list_pending_verifications(
        _env: &Env,
        _admin: Address,
        _start_project_id: u64,
        _limit: u32,
    ) -> Result<Vec<VerificationRecord>, ContractError> {
        todo!("Pending verification listing logic not implemented")
    }

    pub fn verification_exists(_env: &Env, _project_id: u64) -> bool {
        false
    }

    pub fn get_verification_status(
        _env: &Env,
        _project_id: u64,
    ) -> Result<VerificationStatus, ContractError> {
        todo!("Verification status retrieval not implemented")
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
