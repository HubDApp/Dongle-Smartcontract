extern crate alloc;
use alloc::format;
use crate::constants::MAX_ADMIN_ACTION_LOG_PAGE;
use crate::storage_keys::{ExtensionKey, ExtensionKey2, StorageKey};
use crate::types::{AdminActionCategory, AdminActionEntry, AdminActionType, AdminActivityReport};
use soroban_sdk::{Address, Env, String, Vec};

pub struct AdminActionLog;

impl AdminActionLog {
    pub fn record_action(
        env: &Env,
        admin: Address,
        action_type: AdminActionType,
        target_id: Option<u64>,
        target_address: Option<Address>,
        reason_cid: Option<String>,
    ) {
        let id = Self::get_next_id(env);
        let entry = AdminActionEntry {
            id,
            admin: admin.clone(),
            action_type,
            target_id,
            target_address,
            timestamp: env.ledger().timestamp(),
            reason_cid,
        };
        env.storage()
            .persistent()
            .set(&StorageKey::AdminActionLog(id), &entry);
        env.storage()
            .persistent()
            .set(&StorageKey::AdminActionLogCount, &id);

        // Maintain per-admin index so `get_admin_action_log_by_admin` can filter
        // without scanning every entry.
        let mut admin_ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&ExtensionKey2::AdminActionLogByAdmin(admin.clone()))
            .unwrap_or_else(|| Vec::new(env));
        admin_ids.push_back(id);
        env.storage()
            .persistent()
            .set(&ExtensionKey2::AdminActionLogByAdmin(admin), &admin_ids);
    }

    pub fn get_log_entry(env: &Env, log_id: u64) -> Option<AdminActionEntry> {
        env.storage()
            .persistent()
            .get(&StorageKey::AdminActionLog(log_id))
    }

    /// List admin action log entries, **most recent first**.
    ///
    /// # Pagination convention (reverse / most-recent-first offset)
    ///
    /// Unlike the ID-cursor endpoints (`list_projects`, `list_projects_by_status`,
    /// which take a `start_id` and walk forward) and the plain index-offset
    /// endpoints (`list_featured_projects`, `list_reviews`, … which take a
    /// zero-based `start_index` into an ascending list via [`crate::pagination::paginate`]),
    /// this endpoint paginates **backward** from the newest entry:
    ///
    /// ```text
    /// start_idx = count.saturating_sub(start_index)
    /// page      = entries[start_idx], entries[start_idx - 1], … (descending IDs)
    /// ```
    ///
    /// So `start_index = 0` returns the newest `limit` entries, `start_index = limit`
    /// returns the page before that, and so on.
    ///
    /// This divergence is intentional and load-bearing:
    ///
    /// * The action log is an **append-only audit trail**. Operators and
    ///   incident responders almost always want the *latest* actions, so the
    ///   default page (offset 0) must be the newest entries, not the oldest.
    /// * Entry IDs are dense and monotonic (`1..=count`), so a reverse offset
    ///   needs no cursor bookkeeping — `count - start_index` is an O(1) seek.
    /// * A forward offset would make the *first* page the genesis actions and
    ///   force callers to compute `count` themselves and offset from the end on
    ///   every call; every new action would also shift every page boundary.
    ///
    /// The sibling filtered endpoint [`Self::get_admin_action_log_by_admin`] uses
    /// the same most-recent-first offset semantics over the per-admin index for
    /// consistency within this module.
    pub fn list_admin_actions(env: &Env, start_index: u32, limit: u32) -> Vec<AdminActionEntry> {
        let count: u64 = env
            .storage()
            .persistent()
            .get(&StorageKey::AdminActionLogCount)
            .unwrap_or(0);

        if count == 0 {
            return Vec::new(env);
        }

        let effective_limit = if limit == 0 || limit > MAX_ADMIN_ACTION_LOG_PAGE {
            MAX_ADMIN_ACTION_LOG_PAGE
        } else {
            limit
        };

        let start_idx = count.saturating_sub(start_index as u64);

        let mut entries = Vec::new(env);
        let mut i = 0u32;
        while i < effective_limit && start_idx > i as u64 {
            let id = start_idx - i as u64;
            if let Some(entry) = env
                .storage()
                .persistent()
                .get::<_, AdminActionEntry>(&StorageKey::AdminActionLog(id))
            {
                entries.push_back(entry);
            }
            i += 1;
        }
        entries
    }

    /// Return paginated action log entries filtered to a specific admin address.
    ///
    /// Results are returned in reverse-insertion order (most recent first).
    /// `start_index` is a zero-based offset into the filtered result set; `limit` caps
    /// the page size at `MAX_ADMIN_ACTION_LOG_PAGE`.
    pub fn get_admin_action_log_by_admin(
        env: &Env,
        admin: Address,
        start_index: u32,
        limit: u32,
    ) -> Vec<AdminActionEntry> {
        let ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&ExtensionKey2::AdminActionLogByAdmin(admin))
            .unwrap_or_else(|| Vec::new(env));

        let total = ids.len();
        if total == 0 {
            return Vec::new(env);
        }

        let effective_limit = if limit == 0 || limit > MAX_ADMIN_ACTION_LOG_PAGE {
            MAX_ADMIN_ACTION_LOG_PAGE
        } else {
            limit
        };

        // The index is in insertion (ascending) order; iterate from the end for
        // most-recent-first ordering.
        let mut entries = Vec::new(env);
        let mut skipped = 0u32;
        let mut collected = 0u32;

        // Walk backwards through the per-admin ID list.
        let mut pos = total;
        while pos > 0 {
            pos -= 1;
            if let Some(log_id) = ids.get(pos) {
                if skipped < start_index {
                    skipped += 1;
                    continue;
                }
                if collected >= effective_limit {
                    break;
                }
                if let Some(entry) = env
                    .storage()
                    .persistent()
                    .get::<_, AdminActionEntry>(&StorageKey::AdminActionLog(log_id))
                {
                    entries.push_back(entry);
                    collected += 1;
                }
            }
        }
        entries
    }

    pub fn get_action_log_count(env: &Env) -> u64 {
        env.storage()
            .persistent()
            .get(&StorageKey::AdminActionLogCount)
            .unwrap_or(0)
    }

    fn get_next_id(env: &Env) -> u64 {
        env.storage()
            .persistent()
            .get(&StorageKey::AdminActionLogCount)
            .unwrap_or(0)
            + 1
    }

    /// Map an admin action type to its high-level compliance audit category.
    pub fn categorize_action(action_type: &AdminActionType) -> AdminActionCategory {
        match action_type {
            AdminActionType::VerificationApproved
            | AdminActionType::VerificationRejected
            | AdminActionType::VerificationAppealSubmitted
            | AdminActionType::VerificationAppealApproved
            | AdminActionType::VerificationAppealRejected
            | AdminActionType::VerificationRevoked
            | AdminActionType::VerificationSuspended
            | AdminActionType::VerificationRestored
            | AdminActionType::VerificationRiskModelUpdated
            | AdminActionType::VerificationRiskOverridden
            | AdminActionType::VerificationRenewalApproved
            | AdminActionType::VerificationRenewalRejected
            | AdminActionType::VerificationDurationSet
            | AdminActionType::VerificationAssigned
            | AdminActionType::VerificationAssignmentAccepted
            | AdminActionType::VerificationAssignmentDeclined
            | AdminActionType::VerificationAssignmentEscalated
            | AdminActionType::VerificationSlaSet
            | AdminActionType::MinProjectAgeSet
            | AdminActionType::VerificationHistoryCleared
            | AdminActionType::RenewalHistoryCleared => AdminActionCategory::Verification,

            AdminActionType::CollectionCreated
            | AdminActionType::CollectionUpdated
            | AdminActionType::CollectionDeleted
            | AdminActionType::ProjectAddedToCollection
            | AdminActionType::ProjectRemovedFromCollection => AdminActionCategory::Collection,

            AdminActionType::ContractPaused
            | AdminActionType::ContractResumed => AdminActionCategory::Pause,

            AdminActionType::FeeChanged
            | AdminActionType::ThresholdChanged
            | AdminActionType::MaxReviewsPerProjectSet
            | AdminActionType::AdminAdded
            | AdminActionType::AdminRemoved
            | AdminActionType::AdminExpertiseSet
            | AdminActionType::AdminRoleSet
            | AdminActionType::EmergencyBypass
            | AdminActionType::ReservedNameAdded
            | AdminActionType::ReservedNameRemoved
            | AdminActionType::FeeRefunded => AdminActionCategory::Config,

            _ => AdminActionCategory::Config,
        }
    }

    /// Retrieve an audit report of admin actions within a timestamp window [start, end].
    ///
    /// Includes category breakdowns, action details, and suspicious activity detection
    /// (e.g. bulk operations in rapid succession, or actions at unusual times).
    pub fn get_admin_activity_report(
        env: &Env,
        admin: Address,
        start: u64,
        end: u64,
    ) -> AdminActivityReport {
        Self::get_admin_activity_report_filtered(env, admin, start, end, AdminActionCategory::All)
    }

    /// Retrieve an audit report filtered by a specific action category.
    pub fn get_admin_activity_report_filtered(
        env: &Env,
        admin: Address,
        start: u64,
        end: u64,
        category: AdminActionCategory,
    ) -> AdminActivityReport {
        let ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&ExtensionKey2::AdminActionLogByAdmin(admin.clone()))
            .or_else(|| {
                env.storage()
                    .persistent()
                    .get(&ExtensionKey::AdminActionLogByAdmin(admin.clone()))
            })
            .unwrap_or_else(|| Vec::new(env));

        let mut actions = Vec::new(env);
        let mut verification_count = 0u32;
        let mut collection_count = 0u32;
        let mut pause_count = 0u32;
        let mut config_count = 0u32;
        let mut other_count = 0u32;
        let mut total_actions = 0u32;

        let total_ids = ids.len();
        for i in 0..total_ids {
            if let Some(log_id) = ids.get(i) {
                if let Some(entry) = Self::get_log_entry(env, log_id) {
                    if entry.timestamp >= start && entry.timestamp <= end {
                        total_actions += 1;
                        let cat = Self::categorize_action(&entry.action_type);
                        match cat {
                            AdminActionCategory::Verification => verification_count += 1,
                            AdminActionCategory::Collection => collection_count += 1,
                            AdminActionCategory::Pause => pause_count += 1,
                            AdminActionCategory::Config => config_count += 1,
                            _ => other_count += 1,
                        }

                        if category == AdminActionCategory::All || cat == category {
                            actions.push_back(entry);
                        }
                    }
                }
            }
        }

        // Suspicious activity detection:
        // 1. Bulk operations: >= 5 actions within a 60-second window
        // 2. Unusual times: actions executed between 00:00 and 04:00 UTC (timestamp % 86400 < 4 * 3600)
        let mut has_suspicious_activity = false;
        let mut suspicious_reasons = Vec::new(env);

        let mut bulk_detected = false;
        let mut unusual_time_detected = false;

        let action_count = actions.len();
        if action_count >= 5 {
            for i in 0..action_count.saturating_sub(4) {
                if let (Some(first), Some(fifth)) = (actions.get(i), actions.get(i + 4)) {
                    if fifth.timestamp.saturating_sub(first.timestamp) <= 60 {
                        bulk_detected = true;
                        break;
                    }
                }
            }
        }

        for i in 0..action_count {
            if let Some(entry) = actions.get(i) {
                if (entry.timestamp % 86400) < (4 * 3600) {
                    unusual_time_detected = true;
                    break;
                }
            }
        }

        if bulk_detected {
            has_suspicious_activity = true;
            suspicious_reasons.push_back(String::from_str(
                env,
                "BULK_OPERATIONS_DETECTED: 5+ actions in rapid succession (<= 60s)",
            ));
        }

        if unusual_time_detected {
            has_suspicious_activity = true;
            suspicious_reasons.push_back(String::from_str(
                env,
                "UNUSUAL_TIME_OPERATION: action detected during off-hours (00:00-04:00 UTC)",
            ));
        }

        AdminActivityReport {
            admin,
            start_time: start,
            end_time: end,
            total_actions,
            verification_count,
            collection_count,
            pause_count,
            config_count,
            other_count,
            actions,
            has_suspicious_activity,
            suspicious_reasons,
        }
    }

    /// Generate an audit report in standard JSON format.
    pub fn generate_admin_audit_report_json(
        env: &Env,
        admin: Address,
        start: u64,
        end: u64,
    ) -> String {
        let report = Self::get_admin_activity_report(env, admin, start, end);
        let json_str = format!(
            r#"{{"admin":"{}","start_time":{},"end_time":{},"total_actions":{},"verification":{},"collection":{},"pause":{},"config":{},"other":{},"has_suspicious_activity":{}}}"#,
            report.admin,
            report.start_time,
            report.end_time,
            report.total_actions,
            report.verification_count,
            report.collection_count,
            report.pause_count,
            report.config_count,
            report.other_count,
            report.has_suspicious_activity,
        );
        String::from_str(env, &json_str)
    }

    /// Generate an audit report in standard CSV format.
    pub fn generate_admin_audit_report_csv(
        env: &Env,
        admin: Address,
        start: u64,
        end: u64,
    ) -> String {
        let report = Self::get_admin_activity_report(env, admin, start, end);
        let mut csv = format!("id,admin,timestamp,target_id\n");
        let count = report.actions.len();
        for i in 0..count {
            if let Some(entry) = report.actions.get(i) {
                let tid = match entry.target_id {
                    Some(id) => format!("{}", id),
                    None => format!(""),
                };
                csv.push_str(&format!(
                    "{},{},{},{}\n",
                    entry.id, entry.admin, entry.timestamp, tid
                ));
            }
        }
        String::from_str(env, &csv)
    }
}
