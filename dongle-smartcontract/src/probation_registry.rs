//! Probationary verification registry and lifecycle management.
//!
//! Acceptance Criteria:
//! 1. 30-day probationary period after approval
//! 2. Enhanced monitoring during probation
//! 3. Auto-promote to full verification after period
//! 4. Can revoke during probation without full review

use crate::admin_action_log::AdminActionLog;
use crate::admin_manager::AdminManager;
use crate::auth::require_admin_auth;
use crate::constants::{DEFAULT_PROBATIONARY_DURATION_SECS, MAX_PAGE_LIMIT};
use crate::errors::ContractError;
use crate::events::{
    publish_probation_auto_promoted_event, publish_probation_incident_event,
    publish_probation_revoked_event, publish_probation_started_event,
    publish_verification_revoked_event,
};
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::{ProbationKey, StorageKey};
use crate::types::{
    AdminActionType, NotificationKind, ProbationIncident, ProbationRecord, VerificationRecord,
    VerificationStatus,
};
use crate::utils::Utils;
use soroban_sdk::{Address, Env, String, Vec};

pub struct ProbationRegistry;

impl ProbationRegistry {
    /// Get the configured duration for the probationary period (default: 30 days).
    pub fn get_probation_duration(env: &Env) -> u64 {
        env.storage()
            .persistent()
            .get(&ProbationKey::ProbationDuration)
            .unwrap_or(DEFAULT_PROBATIONARY_DURATION_SECS)
    }

    /// Admin-only: configure the probationary period duration in seconds.
    pub fn set_probation_duration(
        env: &Env,
        admin: Address,
        duration_secs: u64,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;

        if AdminManager::get_admin_approval_threshold(env) > 1 {
            return Err(ContractError::Unauthorized);
        }

        if duration_secs == 0 {
            return Err(ContractError::InvalidProjectData);
        }

        env.storage()
            .persistent()
            .set(&ProbationKey::ProbationDuration, &duration_secs);

        Ok(())
    }

    /// Initiate the 30-day probationary verification period for an approved project.
    pub fn start_probation(
        env: &Env,
        project_id: u64,
        request_id: u64,
        admin: &Address,
    ) -> Result<ProbationRecord, ContractError> {
        let now = env.ledger().timestamp();
        let duration = Self::get_probation_duration(env);
        let probation_until = now.saturating_add(duration);

        let record = ProbationRecord {
            project_id,
            request_id,
            approved_by: admin.clone(),
            started_at: now,
            probation_until,
            is_promoted: false,
            is_revoked: false,
            enhanced_monitoring: true,
            incident_count: 0,
        };

        // Persist record
        env.storage()
            .persistent()
            .set(&ProbationKey::ProjectProbation(project_id), &record);

        // Add to active probationary projects index
        let mut projects = Self::get_probationary_projects_raw(env);
        if !projects.contains(&project_id) {
            projects.push_back(project_id);
            env.storage()
                .persistent()
                .set(&ProbationKey::ProbationaryProjects, &projects);
        }

        publish_probation_started_event(
            env,
            project_id,
            request_id,
            admin.clone(),
            now,
            probation_until,
        );

        Ok(record)
    }

    /// Check if a project is currently within its 30-day probationary period.
    /// Automatically considers projects older than 30 days as promoted out of probation.
    pub fn is_in_probation(env: &Env, project_id: u64) -> bool {
        let Some(record) = Self::get_probation_record(env, project_id) else {
            return false;
        };

        if record.is_revoked || record.is_promoted {
            return false;
        }

        let now = env.ledger().timestamp();
        now < record.probation_until
    }

    /// Fetch the stored probation record for a project, if any.
    pub fn get_probation_record(env: &Env, project_id: u64) -> Option<ProbationRecord> {
        env.storage()
            .persistent()
            .get(&ProbationKey::ProjectProbation(project_id))
    }

    /// Returns the effective verification status:
    /// - If in active probation: `VerificationStatus::Probationary`
    /// - If 30-day probation passed or fully verified: `VerificationStatus::Verified`
    /// - Otherwise the base status (Unverified, Pending, Rejected, Suspended)
    pub fn get_effective_verification_status(
        env: &Env,
        project_id: u64,
    ) -> Option<VerificationStatus> {
        let project = ProjectRegistry::get_project(env, project_id)?;
        if project.verification_status == VerificationStatus::Verified {
            if Self::is_in_probation(env, project_id) {
                Some(VerificationStatus::Probationary)
            } else {
                Some(VerificationStatus::Verified)
            }
        } else {
            Some(project.verification_status)
        }
    }

    /// Auto-promote a project to full verification after the 30-day probationary period.
    /// Can be called by any caller or during maintenance; safely idempotently returns whether
    /// the project was promoted.
    pub fn check_and_promote_probation(env: &Env, project_id: u64) -> Result<bool, ContractError> {
        let mut record = match Self::get_probation_record(env, project_id) {
            Some(r) => r,
            None => return Ok(false),
        };

        if record.is_revoked || record.is_promoted {
            return Ok(false);
        }

        let now = env.ledger().timestamp();
        if now >= record.probation_until {
            // 30 days have elapsed — auto-promote to full verification
            record.is_promoted = true;
            record.enhanced_monitoring = false;
            env.storage()
                .persistent()
                .set(&ProbationKey::ProjectProbation(project_id), &record);

            // Remove from active probationary projects list
            let projects = Self::get_probationary_projects_raw(env);
            let updated_projects = Utils::remove_item_from_vec(env, &projects, &project_id);
            env.storage()
                .persistent()
                .set(&ProbationKey::ProbationaryProjects, &updated_projects);

            publish_probation_auto_promoted_event(env, project_id, now);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Fast-track revocation during probation without full review.
    /// Admin can immediately revoke verification of a probationary project.
    pub fn revoke_during_probation(
        env: &Env,
        admin: Address,
        project_id: u64,
        reason: String,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;

        if AdminManager::get_admin_approval_threshold(env) > 1 {
            return Err(ContractError::Unauthorized);
        }

        // Must be currently in probation
        if !Self::is_in_probation(env, project_id) {
            return Err(ContractError::InvalidStatus);
        }

        let mut project =
            ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        let mut record = Self::get_probation_record(env, project_id)
            .ok_or(ContractError::VerificationNotFound)?;

        let now = env.ledger().timestamp();

        // Mark probation revoked
        record.is_revoked = true;
        record.enhanced_monitoring = false;
        env.storage()
            .persistent()
            .set(&ProbationKey::ProjectProbation(project_id), &record);

        // Remove from probationary projects
        let projects = Self::get_probationary_projects_raw(env);
        let updated_projects = Utils::remove_item_from_vec(env, &projects, &project_id);
        env.storage()
            .persistent()
            .set(&ProbationKey::ProbationaryProjects, &updated_projects);

        // Update project status to Unverified
        project.verification_status = VerificationStatus::Unverified;
        project.updated_at = now;
        env.storage()
            .persistent()
            .set(&StorageKey::Project(project_id), &project);

        // Update verification record if exists
        let req_id: Option<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::Verification(project_id));
        if let Some(r_id) = req_id {
            if let Some(mut v_record) = env
                .storage()
                .persistent()
                .get::<_, VerificationRecord>(&StorageKey::VerificationRecord(r_id))
            {
                v_record.status = VerificationStatus::Unverified;
                v_record.revoke_reason = Some(reason.clone());
                v_record.decided_at = now;
                env.storage()
                    .persistent()
                    .set(&StorageKey::VerificationRecord(r_id), &v_record);
            }
        }

        publish_probation_revoked_event(env, project_id, admin.clone(), reason.clone(), now);
        publish_verification_revoked_event(env, project_id, admin.clone(), reason);

        crate::notification_registry::NotificationRegistry::emit_project_notification(
            env,
            project_id,
            NotificationKind::VerificationRevoked,
        );

        AdminActionLog::record_action(
            env,
            admin,
            AdminActionType::VerificationRevoked,
            Some(project_id),
            None,
            None,
        );

        Ok(())
    }

    /// Enhanced monitoring: record an incident or discrepancy against a project in probation.
    pub fn record_probation_incident(
        env: &Env,
        reporter: Address,
        project_id: u64,
        details: String,
    ) -> Result<u32, ContractError> {
        if !Self::is_in_probation(env, project_id) {
            return Err(ContractError::InvalidStatus);
        }

        let mut record = Self::get_probation_record(env, project_id)
            .ok_or(ContractError::VerificationNotFound)?;

        let now = env.ledger().timestamp();
        let incident_id = record.incident_count.saturating_add(1);
        record.incident_count = incident_id;

        let incident = ProbationIncident {
            incident_id,
            project_id,
            reporter: reporter.clone(),
            details: details.clone(),
            recorded_at: now,
        };

        // Persist incident and updated record
        env.storage()
            .persistent()
            .set(&ProbationKey::ProbationIncident(project_id, incident_id), &incident);
        env.storage()
            .persistent()
            .set(&ProbationKey::ProjectProbation(project_id), &record);

        publish_probation_incident_event(env, project_id, incident_id, reporter, details, now);

        Ok(incident_id)
    }

    /// Get total count of incidents recorded during a project's probation.
    pub fn get_probation_incident_count(env: &Env, project_id: u64) -> u32 {
        Self::get_probation_record(env, project_id)
            .map(|r| r.incident_count)
            .unwrap_or(0)
    }

    /// List project IDs currently under active probation and enhanced monitoring.
    pub fn list_probationary_projects(env: &Env, start_index: u32, limit: u32) -> Vec<u64> {
        let projects = Self::get_probationary_projects_raw(env);
        let now = env.ledger().timestamp();
        let mut active: Vec<u64> = Vec::new(env);

        for i in 0..projects.len() {
            if let Some(pid) = projects.get(i) {
                if let Some(rec) = Self::get_probation_record(env, pid) {
                    if !rec.is_revoked && !rec.is_promoted && now < rec.probation_until {
                        active.push_back(pid);
                    }
                }
            }
        }

        let effective_limit = if limit == 0 || limit > MAX_PAGE_LIMIT {
            MAX_PAGE_LIMIT
        } else {
            limit
        };

        let mut page = Vec::new(env);
        let total = active.len();
        if start_index >= total {
            return page;
        }

        let end = (start_index.saturating_add(effective_limit)).min(total);
        for idx in start_index..end {
            if let Some(pid) = active.get(idx) {
                page.push_back(pid);
            }
        }

        page
    }

    fn get_probationary_projects_raw(env: &Env) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&ProbationKey::ProbationaryProjects)
            .unwrap_or_else(|| Vec::new(env))
    }
}
