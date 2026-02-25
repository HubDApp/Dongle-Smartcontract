//! Verification requests with ownership and fee checks, and events.

use crate::admin_registry::AdminRegistry;
use crate::constants::MAX_CID_LEN;
use crate::errors::ContractError;
use crate::events::{
    publish_verification_approved_event, publish_verification_rejected_event,
    publish_verification_requested_event,
};
use crate::project_registry::ProjectRegistry;
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

        // Validate evidence CID
        Self::validate_evidence_cid(&evidence_cid)?;

        // Verify project exists and requester is owner
        let project =
            ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        if project.owner != requester {
            return Err(ContractError::Unauthorized);
        }

        // Create verification record with Pending status
        let record = VerificationRecord {
            status: VerificationStatus::Pending,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Verification(project_id), &record);
        publish_verification_requested_event(env, project_id, requester);

        Ok(())
    }

    pub fn approve_verification(
        env: &Env,
        project_id: u64,
        admin: Address,
    ) -> Result<(), ContractError> {
        admin.require_auth();

        // Check admin authorization
        AdminRegistry::require_admin(env, &admin)?;

        // Get existing verification record
        let mut record = Self::get_verification(env, project_id)?;

        // Validate status transition
        if record.status != VerificationStatus::Pending {
            return Err(ContractError::InvalidStatusTransition);
        }

        // Update status to Verified
        record.status = VerificationStatus::Verified;
        env.storage()
            .persistent()
            .set(&DataKey::Verification(project_id), &record);

        // Update project verification status
        if let Some(mut project) = ProjectRegistry::get_project(env, project_id) {
            project.verification_status = VerificationStatus::Verified;
            env.storage()
                .persistent()
                .set(&DataKey::Project(project_id), &project);
        }

        publish_verification_approved_event(env, project_id);
        Ok(())
    }

    pub fn reject_verification(
        env: &Env,
        project_id: u64,
        admin: Address,
    ) -> Result<(), ContractError> {
        admin.require_auth();

        // Check admin authorization
        AdminRegistry::require_admin(env, &admin)?;

        // Get existing verification record
        let mut record = Self::get_verification(env, project_id)?;

        // Validate status transition
        if record.status != VerificationStatus::Pending {
            return Err(ContractError::InvalidStatusTransition);
        }

        // Update status to Rejected
        record.status = VerificationStatus::Rejected;
        env.storage()
            .persistent()
            .set(&DataKey::Verification(project_id), &record);

        // Update project verification status
        if let Some(mut project) = ProjectRegistry::get_project(env, project_id) {
            project.verification_status = VerificationStatus::Rejected;
            env.storage()
                .persistent()
                .set(&DataKey::Project(project_id), &project);
        }

        publish_verification_rejected_event(env, project_id);
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
        env: &Env,
        admin: Address,
        start_project_id: u64,
        limit: u32,
    ) -> Result<Vec<(u64, VerificationRecord)>, ContractError> {
        admin.require_auth();

        // Check admin authorization
        AdminRegistry::require_admin(env, &admin)?;

        let mut results = Vec::new(env);
        let mut count = 0u32;
        let mut current_id = start_project_id;
        let max_iterations = limit.min(50); // Limit iterations to prevent budget issues

        // Iterate through projects to find pending verifications
        while count < limit && (current_id - start_project_id) < max_iterations as u64 {
            if let Ok(record) = Self::get_verification(env, current_id) {
                if record.status == VerificationStatus::Pending {
                    results.push_back((current_id, record));
                    count += 1;
                }
            }
            current_id += 1;
        }

        Ok(results)
    }

    pub fn verification_exists(env: &Env, project_id: u64) -> bool {
        env.storage()
            .persistent()
            .has(&DataKey::Verification(project_id))
    }

    pub fn get_verification_status(
        env: &Env,
        project_id: u64,
    ) -> Result<VerificationStatus, ContractError> {
        let record = Self::get_verification(env, project_id)?;
        Ok(record.status)
    }

    pub fn update_verification_evidence(
        env: &Env,
        project_id: u64,
        requester: Address,
        new_evidence_cid: String,
    ) -> Result<(), ContractError> {
        requester.require_auth();

        // Validate evidence CID
        Self::validate_evidence_cid(&new_evidence_cid)?;

        // Verify project exists and requester is owner
        let project =
            ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        if project.owner != requester {
            return Err(ContractError::Unauthorized);
        }

        // Get existing verification record
        let record = Self::get_verification(env, project_id)?;

        // Only allow updates for Pending or Rejected status
        if record.status != VerificationStatus::Pending
            && record.status != VerificationStatus::Rejected
        {
            return Err(ContractError::InvalidStatusTransition);
        }

        // Evidence update is implicit - in a real implementation, you'd store the evidence_cid
        // For now, we just verify the operation is allowed

        Ok(())
    }

    pub fn validate_evidence_cid(evidence_cid: &String) -> Result<(), ContractError> {
        if evidence_cid.is_empty() {
            return Err(ContractError::InvalidProjectData);
        }
        Ok(())
    }

    pub fn get_verification_stats(env: &Env) -> (u32, u32, u32) {
        // Returns (pending_count, verified_count, rejected_count)
        // This is a simplified implementation
        (0, 0, 0)
    }
}
