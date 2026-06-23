//! Project reporting functionality for spam, scams, broken links, or abusive metadata.

use crate::admin_action_log::AdminActionLog;
use crate::errors::ContractError;
use crate::events::publish_project_reported_event;
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::StorageKey;
use crate::types::{AdminActionType, ProjectReport};
use crate::utils::Utils;
use soroban_sdk::{Address, Env, String, Vec};

pub struct ReportRegistry;

impl ReportRegistry {
    /// Report a project with a reason CID
    pub fn report_project(
        env: &Env,
        project_id: u64,
        reporter: Address,
        reason_cid: String,
    ) -> Result<(), ContractError> {
        // Validate project exists
        ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        // Require authentication
        reporter.require_auth();

        // Validate reason CID
        Utils::validate_report_reason_cid(&reason_cid)?;

        // Check for duplicate reports from the same user
        if env
            .storage()
            .persistent()
            .has(&StorageKey::UserReport(project_id, reporter.clone()))
        {
            return Err(ContractError::AlreadyReported);
        }

        let now = env.ledger().timestamp();
        let report = ProjectReport {
            project_id,
            reporter: reporter.clone(),
            reason_cid: reason_cid.clone(),
            timestamp: now,
        };

        // Get existing reports for this project
        let mut reports: Vec<ProjectReport> = env
            .storage()
            .persistent()
            .get(&StorageKey::ProjectReports(project_id))
            .unwrap_or_else(|| Vec::new(env));

        // Add new report
        reports.push_back(report);

        // Store updated reports list
        env.storage()
            .persistent()
            .set(&StorageKey::ProjectReports(project_id), &reports);

        // Track user report to prevent duplicates
        env.storage()
            .persistent()
            .set(&StorageKey::UserReport(project_id, reporter.clone()), &true);

        // Update report count
        let count = reports.len();
        env.storage()
            .persistent()
            .set(&StorageKey::ProjectReportCount(project_id), &count);

        publish_project_reported_event(env, project_id, reporter, reason_cid);
        Ok(())
    }

    /// Get all reports for a project (admin only)
    pub fn get_project_reports(env: &Env, project_id: u64) -> Vec<ProjectReport> {
        env.storage()
            .persistent()
            .get(&StorageKey::ProjectReports(project_id))
            .unwrap_or_else(|| Vec::new(env))
    }

    /// Get report count for a project
    pub fn get_project_report_count(env: &Env, project_id: u64) -> u32 {
        env.storage()
            .persistent()
            .get(&StorageKey::ProjectReportCount(project_id))
            .unwrap_or(0)
    }

    /// Check if a user has already reported a project
    pub fn has_user_reported(env: &Env, project_id: u64, reporter: &Address) -> bool {
        env.storage()
            .persistent()
            .has(&StorageKey::UserReport(project_id, reporter.clone()))
    }

    /// Clear all reports for a project (admin-only).
    ///
    /// Removes the reports list, the report count, and all per-user dedup keys
    /// so reporters can file new reports after the slate is cleaned.
    /// Returns `ReportsAlreadyCleared` if there are no reports to clear.
    pub fn clear_project_reports(
        env: &Env,
        project_id: u64,
        admin: &Address,
    ) -> Result<(), ContractError> {
        // Auth: admin only
        if !crate::admin_manager::AdminManager::is_admin(env, admin) {
            return Err(ContractError::AdminOnly);
        }

        // Project must exist
        crate::project_registry::ProjectRegistry::get_project(env, project_id)
            .ok_or(ContractError::ProjectNotFound)?;

        let count = Self::get_project_report_count(env, project_id);
        if count == 0 {
            return Err(ContractError::ReportsCleared);
        }

        // Gather existing reports to remove individual UserReport dedup keys
        let reports: Vec<crate::types::ProjectReport> = env
            .storage()
            .persistent()
            .get(&StorageKey::ProjectReports(project_id))
            .unwrap_or_else(|| Vec::new(env));

        // Remove per-reporter dedup keys so they can report again
        for i in 0..reports.len() {
            if let Some(report) = reports.get(i) {
                env.storage()
                    .persistent()
                    .remove(&StorageKey::UserReport(project_id, report.reporter.clone()));
            }
        }

        // Remove the reports list and count
        env.storage()
            .persistent()
            .remove(&StorageKey::ProjectReports(project_id));
        env.storage()
            .persistent()
            .remove(&StorageKey::ProjectReportCount(project_id));

        crate::events::publish_project_reports_cleared_event(env, project_id, admin.clone());

        AdminActionLog::record_action(
            env,
            admin.clone(),
            AdminActionType::ProjectReportsCleared,
            Some(project_id),
            None,
            None,
        );

        Ok(())
    }
}
