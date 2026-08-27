use crate::constants::MAX_ADMIN_ACTION_LOG_PAGE;
use crate::storage_keys::{ExtensionKey, StorageKey};
use crate::types::{AdminActionEntry, AdminActionType};
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
            .get(&ExtensionKey::AdminActionLogByAdmin(admin.clone()))
            .unwrap_or_else(|| Vec::new(env));
        admin_ids.push_back(id);
        env.storage()
            .persistent()
            .set(&ExtensionKey::AdminActionLogByAdmin(admin), &admin_ids);
    }

    pub fn get_log_entry(env: &Env, log_id: u64) -> Option<AdminActionEntry> {
        env.storage()
            .persistent()
            .get(&StorageKey::AdminActionLog(log_id))
    }

    pub fn list_admin_actions(env: &Env, start: u32, limit: u32) -> Vec<AdminActionEntry> {
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

        let start_idx = count.saturating_sub(start as u64);

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
    /// `start` is a zero-based offset into the filtered result set; `limit` caps
    /// the page size at `MAX_ADMIN_ACTION_LOG_PAGE`.
    pub fn get_admin_action_log_by_admin(
        env: &Env,
        admin: Address,
        start: u32,
        limit: u32,
    ) -> Vec<AdminActionEntry> {
        let ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&ExtensionKey::AdminActionLogByAdmin(admin))
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
                if skipped < start {
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
}
