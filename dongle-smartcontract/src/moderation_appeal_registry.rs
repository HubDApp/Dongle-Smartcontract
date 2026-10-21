//! Report decisions and appeals against them (#922).
//!
//! A moderator (the report's assignee or any admin) records a
//! `ReportDecision` on a report. The reporter or the reported project's owner
//! may then appeal that decision once. Appeals are routed round-robin to the
//! admin-managed senior moderator list, skipping the moderator who made the
//! original decision, and per-moderator / global appeal statistics are kept
//! so moderators' decisions can be reviewed.

use crate::admin_manager::AdminManager;
use crate::errors::ContractError;
use crate::events::{
    publish_moderation_appeal_filed_event, publish_moderation_appeal_resolved_event,
    publish_report_resolved_event,
};
use crate::project_registry::ProjectRegistry;
use crate::report_registry::ReportRegistry;
use crate::storage_keys::ModerationAppealKey;
use crate::types::{
    AppealRecord, AppealStats, AppealStatus, ReportDecision, ReportDecisionRecord,
};
use crate::utils::Utils;
use soroban_sdk::{Address, Env, String, Vec};

pub struct ModerationAppealRegistry;

impl ModerationAppealRegistry {
    fn require_admin(env: &Env, admin: &Address) -> Result<(), ContractError> {
        admin.require_auth();
        if !AdminManager::is_admin(env, admin) {
            return Err(ContractError::AdminOnly);
        }
        Ok(())
    }

    fn validate_text(text: &String) -> Result<(), ContractError> {
        crate::validation::validate_utf8_string(text)?;
        if text.is_empty() || text.len() > crate::constants::MAX_APPEAL_TEXT_LEN {
            return Err(ContractError::InvalidInput);
        }
        Ok(())
    }

    // ── Report decisions ─────────────────────────────────────────────────────

    /// Record a moderation decision on a report. Callable by the report's
    /// assigned moderator or any admin; a report can be decided only once.
    pub fn resolve_report(
        env: &Env,
        moderator: Address,
        report_id: u64,
        decision: ReportDecision,
        notes_cid: Option<String>,
    ) -> Result<(), ContractError> {
        moderator.require_auth();
        let report =
            ReportRegistry::get_report(env, report_id).ok_or(ContractError::ReportNotFound)?;

        let is_assigned = report.assigned_moderator.as_ref() == Some(&moderator);
        if !is_assigned && !AdminManager::is_admin(env, &moderator) {
            return Err(ContractError::Unauthorized);
        }
        if let Some(cid) = &notes_cid {
            Utils::validate_metadata_cid(cid)?;
        }

        let key = ModerationAppealKey::Decision(report_id);
        if env.storage().persistent().has(&key) {
            return Err(ContractError::ReportAlreadyResolved);
        }

        let record = ReportDecisionRecord {
            report_id,
            decision,
            moderator: moderator.clone(),
            notes_cid,
            decided_at: env.ledger().timestamp(),
        };
        env.storage().persistent().set(&key, &record);

        publish_report_resolved_event(env, report_id, report.project_id, decision, moderator);
        Ok(())
    }

    /// Decision recorded for a report, if any.
    pub fn get_report_decision(env: &Env, report_id: u64) -> Option<ReportDecisionRecord> {
        env.storage()
            .persistent()
            .get(&ModerationAppealKey::Decision(report_id))
    }

    // ── Senior moderators ────────────────────────────────────────────────────

    /// Admin: designate an admin as a senior moderator.
    pub fn add_senior_moderator(
        env: &Env,
        admin: Address,
        moderator: Address,
    ) -> Result<(), ContractError> {
        Self::require_admin(env, &admin)?;
        if !AdminManager::is_admin(env, &moderator) {
            return Err(ContractError::AdminOnly);
        }
        let mut seniors = Self::get_senior_moderators(env);
        if seniors.contains(&moderator) {
            return Err(ContractError::InvalidInput);
        }
        if seniors.len() >= crate::constants::MAX_SENIOR_MODERATORS {
            return Err(ContractError::InvalidInput);
        }
        seniors.push_back(moderator);
        env.storage()
            .persistent()
            .set(&ModerationAppealKey::SeniorModerators, &seniors);
        Ok(())
    }

    /// Admin: remove a senior moderator. Appeals already assigned to them
    /// stay assigned.
    pub fn remove_senior_moderator(
        env: &Env,
        admin: Address,
        moderator: Address,
    ) -> Result<(), ContractError> {
        Self::require_admin(env, &admin)?;
        let seniors = Self::get_senior_moderators(env);
        if !seniors.contains(&moderator) {
            return Err(ContractError::AdminNotFound);
        }
        let remaining = Utils::remove_item_from_vec(env, &seniors, &moderator);
        env.storage()
            .persistent()
            .set(&ModerationAppealKey::SeniorModerators, &remaining);
        Ok(())
    }

    pub fn get_senior_moderators(env: &Env) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&ModerationAppealKey::SeniorModerators)
            .unwrap_or_else(|| Vec::new(env))
    }

    /// Pick the next senior moderator round-robin, skipping `exclude` (the
    /// moderator whose decision is under appeal).
    fn route_to_senior(env: &Env, exclude: &Address) -> Result<Address, ContractError> {
        let seniors = Self::get_senior_moderators(env);
        let count = seniors.len();
        if count == 0 {
            return Err(ContractError::NoSeniorModerators);
        }
        let cursor: u32 = env
            .storage()
            .persistent()
            .get(&ModerationAppealKey::SeniorCursor)
            .unwrap_or(0);
        for offset in 0..count {
            let idx = (cursor + offset) % count;
            if let Some(candidate) = seniors.get(idx) {
                if &candidate != exclude {
                    env.storage()
                        .persistent()
                        .set(&ModerationAppealKey::SeniorCursor, &((idx + 1) % count));
                    return Ok(candidate);
                }
            }
        }
        Err(ContractError::NoSeniorModerators)
    }

    // ── Appeals ──────────────────────────────────────────────────────────────

    /// Appeal the moderation decision on a report. Only the reporter or the
    /// reported project's owner may appeal, and only once per report.
    /// Returns the new appeal ID.
    pub fn appeal_moderation_decision(
        env: &Env,
        appellant: Address,
        original_report_id: u64,
        appeal_reason: String,
    ) -> Result<u64, ContractError> {
        appellant.require_auth();
        let report = ReportRegistry::get_report(env, original_report_id)
            .ok_or(ContractError::ReportNotFound)?;
        let decision = Self::get_report_decision(env, original_report_id)
            .ok_or(ContractError::ReportNotResolved)?;

        let is_owner = ProjectRegistry::get_project(env, report.project_id)
            .map(|p| p.owner == appellant)
            .unwrap_or(false);
        if appellant != report.reporter && !is_owner {
            return Err(ContractError::Unauthorized);
        }

        Self::validate_text(&appeal_reason)?;

        let report_appeal_key = ModerationAppealKey::ReportAppeal(original_report_id);
        if env.storage().persistent().has(&report_appeal_key) {
            return Err(ContractError::ModerationAppealAlreadyFiled);
        }

        let senior = Self::route_to_senior(env, &decision.moderator)?;

        let appeal_id: u64 = env
            .storage()
            .persistent()
            .get(&ModerationAppealKey::NextAppealId)
            .unwrap_or(1);
        env.storage()
            .persistent()
            .set(&ModerationAppealKey::NextAppealId, &(appeal_id + 1));

        let appeal = AppealRecord {
            appeal_id,
            report_id: original_report_id,
            project_id: report.project_id,
            appellant: appellant.clone(),
            original_decision: decision.decision,
            original_moderator: decision.moderator.clone(),
            appeal_reason,
            assigned_senior_moderator: senior.clone(),
            status: AppealStatus::Pending,
            resolution: None,
            resolved_by: None,
            filed_at: env.ledger().timestamp(),
            resolved_at: None,
        };
        env.storage()
            .persistent()
            .set(&ModerationAppealKey::Appeal(appeal_id), &appeal);
        env.storage().persistent().set(&report_appeal_key, &appeal_id);

        let queue_key = ModerationAppealKey::SeniorQueue(senior.clone());
        let mut queue: Vec<u64> = env
            .storage()
            .persistent()
            .get(&queue_key)
            .unwrap_or_else(|| Vec::new(env));
        queue.push_back(appeal_id);
        env.storage().persistent().set(&queue_key, &queue);

        Self::update_stats(env, &decision.moderator, |s| {
            s.total += 1;
            s.pending += 1;
        });

        publish_moderation_appeal_filed_event(
            env,
            appeal_id,
            original_report_id,
            report.project_id,
            appellant,
            decision.decision,
            senior,
        );
        Ok(appeal_id)
    }

    /// Resolve a pending appeal. Only the senior moderator the appeal was
    /// routed to may resolve it.
    pub fn resolve_moderation_appeal(
        env: &Env,
        senior_moderator: Address,
        appeal_id: u64,
        overturn: bool,
        resolution: String,
    ) -> Result<(), ContractError> {
        senior_moderator.require_auth();
        let key = ModerationAppealKey::Appeal(appeal_id);
        let mut appeal: AppealRecord = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(ContractError::ModerationAppealNotFound)?;

        if appeal.assigned_senior_moderator != senior_moderator {
            return Err(ContractError::Unauthorized);
        }
        if appeal.status != AppealStatus::Pending {
            return Err(ContractError::ModerationAppealNotPending);
        }
        Self::validate_text(&resolution)?;

        let status = if overturn {
            AppealStatus::Overturned
        } else {
            AppealStatus::Upheld
        };
        appeal.status = status;
        appeal.resolution = Some(resolution);
        appeal.resolved_by = Some(senior_moderator.clone());
        appeal.resolved_at = Some(env.ledger().timestamp());
        env.storage().persistent().set(&key, &appeal);

        let queue_key = ModerationAppealKey::SeniorQueue(senior_moderator.clone());
        let queue: Vec<u64> = env
            .storage()
            .persistent()
            .get(&queue_key)
            .unwrap_or_else(|| Vec::new(env));
        let remaining = Utils::remove_item_from_vec(env, &queue, &appeal_id);
        env.storage().persistent().set(&queue_key, &remaining);

        Self::update_stats(env, &appeal.original_moderator, |s| {
            s.pending = s.pending.saturating_sub(1);
            if overturn {
                s.overturned += 1;
            } else {
                s.upheld += 1;
            }
        });

        publish_moderation_appeal_resolved_event(
            env,
            appeal_id,
            appeal.report_id,
            status,
            senior_moderator,
        );
        Ok(())
    }

    /// Apply `f` to both the moderator's and the global appeal statistics.
    fn update_stats<F: Fn(&mut AppealStats)>(env: &Env, moderator: &Address, f: F) {
        let mod_key = ModerationAppealKey::ModeratorStats(moderator.clone());
        let mut mod_stats = Self::get_moderator_appeal_stats(env, moderator.clone());
        f(&mut mod_stats);
        env.storage().persistent().set(&mod_key, &mod_stats);

        let mut global = Self::get_global_appeal_stats(env);
        f(&mut global);
        env.storage()
            .persistent()
            .set(&ModerationAppealKey::GlobalStats, &global);
    }

    pub fn get_appeal(env: &Env, appeal_id: u64) -> Option<AppealRecord> {
        env.storage()
            .persistent()
            .get(&ModerationAppealKey::Appeal(appeal_id))
    }

    /// Appeal ID filed against a report, if any.
    pub fn get_report_appeal_id(env: &Env, report_id: u64) -> Option<u64> {
        env.storage()
            .persistent()
            .get(&ModerationAppealKey::ReportAppeal(report_id))
    }

    /// Pending appeal IDs assigned to a senior moderator.
    pub fn get_senior_appeal_queue(env: &Env, senior_moderator: Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&ModerationAppealKey::SeniorQueue(senior_moderator))
            .unwrap_or_else(|| Vec::new(env))
    }

    /// Appeal statistics for decisions made by `moderator`.
    pub fn get_moderator_appeal_stats(env: &Env, moderator: Address) -> AppealStats {
        env.storage()
            .persistent()
            .get(&ModerationAppealKey::ModeratorStats(moderator))
            .unwrap_or_default()
    }

    /// Appeal statistics across all moderators.
    pub fn get_global_appeal_stats(env: &Env) -> AppealStats {
        env.storage()
            .persistent()
            .get(&ModerationAppealKey::GlobalStats)
            .unwrap_or_default()
    }
}
