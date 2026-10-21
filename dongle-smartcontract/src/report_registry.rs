//! Project reporting functionality for spam, scams, broken links, or abusive metadata.

use crate::admin_action_log::AdminActionLog;
use crate::errors::ContractError;
use crate::admin_manager::AdminManager;
use crate::events::{
    publish_project_reported_event, publish_report_evidence_attached_event,
    publish_report_routed_event, publish_report_routing_rule_updated_event,
};
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::{ReportModerationKey, StorageKey};
use crate::types::{
    AdminActionType, ProjectReport, ReportAssignment, ReportCategory, ReportEvidenceAttachment,
    ReportRoutingRule,
};
use crate::utils::Utils;
use soroban_sdk::{Address, Env, String, Vec};

pub struct ReportRegistry;

impl ReportRegistry {
    /// Report a project with a reason CID. The report is filed under
    /// `ReportCategory::Other`.
    pub fn report_project(
        env: &Env,
        project_id: u64,
        reporter: Address,
        reason_cid: String,
    ) -> Result<(), ContractError> {
        Self::report_project_with_category(
            env,
            project_id,
            reporter,
            reason_cid,
            ReportCategory::Other,
        )
        .map(|_| ())
    }

    /// Report a project under a specific category and return the new report
    /// ID. If a routing rule exists for the category, the report is
    /// immediately assigned to the next moderator in that rule (#920).
    pub fn report_project_with_category(
        env: &Env,
        project_id: u64,
        reporter: Address,
        reason_cid: String,
        category: ReportCategory,
    ) -> Result<u64, ContractError> {
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
        let report_id = Self::next_report_id(env);
        let report = ProjectReport {
            project_id,
            reporter: reporter.clone(),
            reason_cid: reason_cid.clone(),
            timestamp: now,
            report_id,
            category,
            assigned_moderator: None,
            evidence_cids: Vec::new(env),
        };
        env.storage()
            .persistent()
            .set(&ReportModerationKey::Report(report_id), &report);

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

        // Auto-route when the category has a routing rule; reports in
        // categories without a rule stay unassigned until an admin routes them.
        if Self::get_routing_rule(env, category).is_some() {
            Self::assign_next_moderator(env, report_id, category, None)?;
        }

        Ok(report_id)
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
            return Err(ContractError::InvalidStatus);
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

    // ── Report routing (#920) ────────────────────────────────────────────────

    fn next_report_id(env: &Env) -> u64 {
        let id: u64 = env
            .storage()
            .persistent()
            .get(&ReportModerationKey::NextReportId)
            .unwrap_or(1);
        env.storage()
            .persistent()
            .set(&ReportModerationKey::NextReportId, &(id + 1));
        id
    }

    /// Get a report by its global ID.
    pub fn get_report(env: &Env, report_id: u64) -> Option<ProjectReport> {
        env.storage()
            .persistent()
            .get(&ReportModerationKey::Report(report_id))
    }

    /// Persist a report under its ID and mirror the change into the
    /// project's report list so `get_project_reports` stays consistent.
    pub(crate) fn save_report(env: &Env, report: &ProjectReport) {
        env.storage()
            .persistent()
            .set(&ReportModerationKey::Report(report.report_id), report);

        let key = StorageKey::ProjectReports(report.project_id);
        let reports: Vec<ProjectReport> = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| Vec::new(env));
        for i in 0..reports.len() {
            if let Some(existing) = reports.get(i) {
                if existing.report_id == report.report_id {
                    let mut updated = reports.clone();
                    updated.set(i, report.clone());
                    env.storage().persistent().set(&key, &updated);
                    break;
                }
            }
        }
    }

    fn require_admin(env: &Env, admin: &Address) -> Result<(), ContractError> {
        admin.require_auth();
        if !AdminManager::is_admin(env, admin) {
            return Err(ContractError::AdminOnly);
        }
        Ok(())
    }

    /// Create or replace the routing rule for a category. Every moderator
    /// must be an admin.
    pub fn set_routing_rule(
        env: &Env,
        admin: Address,
        category: ReportCategory,
        moderators: Vec<Address>,
    ) -> Result<(), ContractError> {
        Self::require_admin(env, &admin)?;

        if moderators.is_empty()
            || moderators.len() > crate::constants::MAX_MODERATORS_PER_ROUTING_RULE
        {
            return Err(ContractError::InvalidInput);
        }
        let mut unique: Vec<Address> = Vec::new(env);
        for moderator in moderators.iter() {
            if !AdminManager::is_admin(env, &moderator) {
                return Err(ContractError::AdminOnly);
            }
            if !unique.contains(&moderator) {
                unique.push_back(moderator);
            }
        }

        let rule = ReportRoutingRule {
            category,
            moderators: unique.clone(),
            next_index: 0,
            updated_by: admin.clone(),
            updated_at: env.ledger().timestamp(),
        };
        env.storage()
            .persistent()
            .set(&ReportModerationKey::RoutingRule(category), &rule);

        publish_report_routing_rule_updated_event(env, category, admin, unique.len());
        Ok(())
    }

    /// Remove the routing rule for a category. Existing assignments are kept.
    pub fn remove_routing_rule(
        env: &Env,
        admin: Address,
        category: ReportCategory,
    ) -> Result<(), ContractError> {
        Self::require_admin(env, &admin)?;
        let key = ReportModerationKey::RoutingRule(category);
        if !env.storage().persistent().has(&key) {
            return Err(ContractError::ReportRoutingRuleNotFound);
        }
        env.storage().persistent().remove(&key);
        publish_report_routing_rule_updated_event(env, category, admin, 0);
        Ok(())
    }

    /// Get the routing rule for a category, if configured.
    pub fn get_routing_rule(env: &Env, category: ReportCategory) -> Option<ReportRoutingRule> {
        env.storage()
            .persistent()
            .get(&ReportModerationKey::RoutingRule(category))
    }

    /// Route a report to the next moderator for `category`, re-categorising
    /// the report if needed. Returns the assigned moderator.
    pub fn route_report_to_moderator(
        env: &Env,
        admin: Address,
        report_id: u64,
        category: ReportCategory,
    ) -> Result<Address, ContractError> {
        Self::require_admin(env, &admin)?;
        Self::assign_next_moderator(env, report_id, category, Some(admin))
    }

    /// Pick the next moderator from the category's rule (round-robin) and
    /// assign the report to them.
    fn assign_next_moderator(
        env: &Env,
        report_id: u64,
        category: ReportCategory,
        assigned_by: Option<Address>,
    ) -> Result<Address, ContractError> {
        let mut report = Self::get_report(env, report_id).ok_or(ContractError::ReportNotFound)?;
        let mut rule =
            Self::get_routing_rule(env, category).ok_or(ContractError::ReportRoutingRuleNotFound)?;

        let idx = rule.next_index % rule.moderators.len();
        let moderator = rule
            .moderators
            .get(idx)
            .ok_or(ContractError::ReportRoutingRuleNotFound)?;
        rule.next_index = (idx + 1) % rule.moderators.len();
        env.storage()
            .persistent()
            .set(&ReportModerationKey::RoutingRule(category), &rule);

        report.category = category;
        Self::assign(env, &mut report, moderator.clone(), assigned_by);
        Ok(moderator)
    }

    /// Record `moderator` as the report's assignee, moving it out of the
    /// previous moderator's queue.
    fn assign(
        env: &Env,
        report: &mut ProjectReport,
        moderator: Address,
        assigned_by: Option<Address>,
    ) {
        let previous = report.assigned_moderator.clone();
        if let Some(prev) = &previous {
            let key = ReportModerationKey::ModeratorAssignments(prev.clone());
            let ids: Vec<u64> = env
                .storage()
                .persistent()
                .get(&key)
                .unwrap_or_else(|| Vec::new(env));
            let remaining = Utils::remove_item_from_vec(env, &ids, &report.report_id);
            env.storage().persistent().set(&key, &remaining);
        }

        let key = ReportModerationKey::ModeratorAssignments(moderator.clone());
        let mut ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| Vec::new(env));
        if !ids.contains(report.report_id) {
            ids.push_back(report.report_id);
        }
        env.storage().persistent().set(&key, &ids);

        let assignment = ReportAssignment {
            report_id: report.report_id,
            moderator: moderator.clone(),
            category: report.category,
            assigned_by,
            assigned_at: env.ledger().timestamp(),
        };
        env.storage()
            .persistent()
            .set(&ReportModerationKey::Assignment(report.report_id), &assignment);

        report.assigned_moderator = Some(moderator.clone());
        Self::save_report(env, report);

        publish_report_routed_event(
            env,
            report.report_id,
            report.project_id,
            report.category,
            moderator,
            previous,
        );
    }

    /// Reassign a batch of reports to `new_moderator` (admin-only). Returns
    /// the number of reports whose assignee changed; reports already held by
    /// `new_moderator` are skipped. Fails atomically if any report is missing.
    pub fn bulk_reassign_reports(
        env: &Env,
        admin: Address,
        report_ids: Vec<u64>,
        new_moderator: Address,
    ) -> Result<u32, ContractError> {
        Self::require_admin(env, &admin)?;
        if report_ids.is_empty() || report_ids.len() > crate::constants::MAX_REPORT_REASSIGN_BATCH
        {
            return Err(ContractError::InvalidInput);
        }
        if !AdminManager::is_admin(env, &new_moderator) {
            return Err(ContractError::AdminOnly);
        }

        let mut reassigned: u32 = 0;
        for report_id in report_ids.iter() {
            let mut report =
                Self::get_report(env, report_id).ok_or(ContractError::ReportNotFound)?;
            if report.assigned_moderator.as_ref() == Some(&new_moderator) {
                continue;
            }
            Self::assign(env, &mut report, new_moderator.clone(), Some(admin.clone()));
            reassigned += 1;
        }
        Ok(reassigned)
    }

    /// Current assignment record for a report, if any.
    pub fn get_report_assignment(env: &Env, report_id: u64) -> Option<ReportAssignment> {
        env.storage()
            .persistent()
            .get(&ReportModerationKey::Assignment(report_id))
    }

    /// Report IDs currently assigned to a moderator.
    pub fn get_moderator_assignments(env: &Env, moderator: Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&ReportModerationKey::ModeratorAssignments(moderator))
            .unwrap_or_else(|| Vec::new(env))
    }

    // ── Report evidence (#921) ───────────────────────────────────────────────

    /// Attach an evidence document CID to a report.
    ///
    /// Callable by the original reporter, the assigned moderator, or any
    /// admin. Evidence is append-only: CIDs are content-addressed (and thus
    /// immutable) and attached CIDs can never be replaced or removed. Each
    /// attachment is recorded in the report's evidence audit trail.
    pub fn attach_evidence_to_report(
        env: &Env,
        report_id: u64,
        submitter: Address,
        evidence_cid: String,
    ) -> Result<(), ContractError> {
        submitter.require_auth();
        let mut report = Self::get_report(env, report_id).ok_or(ContractError::ReportNotFound)?;

        let is_assigned_moderator = report.assigned_moderator.as_ref() == Some(&submitter);
        if submitter != report.reporter
            && !is_assigned_moderator
            && !AdminManager::is_admin(env, &submitter)
        {
            return Err(ContractError::Unauthorized);
        }

        Utils::validate_metadata_cid(&evidence_cid)?;

        if report.evidence_cids.len() >= crate::constants::MAX_REPORT_EVIDENCE {
            return Err(ContractError::ReportEvidenceLimitReached);
        }
        if report.evidence_cids.contains(&evidence_cid) {
            return Err(ContractError::ReportEvidenceAlreadyAttached);
        }

        let index = report.evidence_cids.len();
        report.evidence_cids.push_back(evidence_cid.clone());
        Self::save_report(env, &report);

        let audit_key = ReportModerationKey::EvidenceAudit(report_id);
        let mut audit: Vec<ReportEvidenceAttachment> = env
            .storage()
            .persistent()
            .get(&audit_key)
            .unwrap_or_else(|| Vec::new(env));
        audit.push_back(ReportEvidenceAttachment {
            report_id,
            evidence_cid: evidence_cid.clone(),
            index,
            attached_by: submitter.clone(),
            attached_at: env.ledger().timestamp(),
        });
        env.storage().persistent().set(&audit_key, &audit);

        publish_report_evidence_attached_event(
            env,
            report_id,
            report.project_id,
            evidence_cid,
            index,
            submitter,
        );
        Ok(())
    }

    /// Evidence CIDs attached to a report, in attachment order.
    pub fn get_report_evidence(env: &Env, report_id: u64) -> Vec<String> {
        Self::get_report(env, report_id)
            .map(|r| r.evidence_cids)
            .unwrap_or_else(|| Vec::new(env))
    }

    /// Full evidence audit trail for a report.
    pub fn get_report_evidence_audit(env: &Env, report_id: u64) -> Vec<ReportEvidenceAttachment> {
        env.storage()
            .persistent()
            .get(&ReportModerationKey::EvidenceAudit(report_id))
            .unwrap_or_else(|| Vec::new(env))
    }
}
