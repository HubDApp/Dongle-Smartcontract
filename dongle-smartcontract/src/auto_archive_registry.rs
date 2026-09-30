//! Keeper-driven automatic archival for inactive projects (issue #753).
//!
//! Soroban contracts cannot execute on a timer, so any authenticated keeper
//! may create owner notices and run bounded archival batches. A project cannot
//! be auto-archived until a notice has existed for the full 30-day period and
//! no project update has invalidated it.

use crate::auth::require_admin_auth;
use crate::auto_archive_types::{
    ArchivedProjectSearchResult, AutoArchiveBatchResult, AutoArchiveConfig, AutoArchiveNotice,
    AutoArchiveNoticeStatus, AutoArchiveRecord,
};
use crate::constants::{
    AUTO_ARCHIVE_NOTICE_SECS, DEFAULT_AUTO_ARCHIVE_THRESHOLD_SECS, MAX_ARCHIVED_SEARCH_SCAN,
    MAX_AUTO_ARCHIVE_BATCH_SIZE, MAX_AUTO_ARCHIVE_THRESHOLD_SECS, MAX_PAGE_LIMIT,
    MIN_AUTO_ARCHIVE_THRESHOLD_SECS,
};
use crate::errors::ContractError;
use crate::events::{
    publish_auto_archive_config_event, publish_auto_archive_notice_delivery_event,
    publish_auto_archive_notice_event, publish_project_auto_archived_event,
};
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::{AutoArchiveKey as AK, StorageKey};
use crate::storage_manager::StorageManager;
use crate::types::Project;
use crate::utils::Utils;
use soroban_sdk::{Address, Env, String, Vec};

pub struct AutoArchiveRegistry;

impl AutoArchiveRegistry {
    fn default_config() -> AutoArchiveConfig {
        AutoArchiveConfig {
            enabled: true,
            inactivity_threshold_secs: DEFAULT_AUTO_ARCHIVE_THRESHOLD_SECS,
        }
    }

    pub fn get_config(env: &Env) -> AutoArchiveConfig {
        let config = env
            .storage()
            .persistent()
            .get(&AK::Config)
            .unwrap_or_else(Self::default_config);
        StorageManager::extend_auto_archive_config_ttl(env);
        config
    }

    /// Admin-only configuration. The threshold must leave room for the fixed
    /// 30-day notice window and is bounded to one through ten years.
    pub fn set_config(
        env: &Env,
        admin: Address,
        config: AutoArchiveConfig,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;
        if config.inactivity_threshold_secs < MIN_AUTO_ARCHIVE_THRESHOLD_SECS
            || config.inactivity_threshold_secs > MAX_AUTO_ARCHIVE_THRESHOLD_SECS
        {
            return Err(ContractError::AutoArchiveThresholdInvalid);
        }
        env.storage().persistent().set(&AK::Config, &config);
        StorageManager::extend_auto_archive_config_ttl(env);
        publish_auto_archive_config_event(env, &config, admin);
        Ok(())
    }

    pub fn get_notice(env: &Env, project_id: u64) -> Option<AutoArchiveNotice> {
        let notice = env.storage().persistent().get(&AK::Notice(project_id));
        if notice.is_some() {
            StorageManager::extend_auto_archive_project_ttl(env, project_id);
        }
        notice
    }

    fn notice_start(last_activity_at: u64, threshold_secs: u64) -> u64 {
        last_activity_at
            .saturating_add(threshold_secs)
            .saturating_sub(AUTO_ARCHIVE_NOTICE_SECS)
    }

    /// Create the owner-facing pre-archival notice. Returns false when a valid
    /// notice for the same activity timestamp already exists.
    pub fn notify_project(
        env: &Env,
        keeper: Address,
        project_id: u64,
    ) -> Result<bool, ContractError> {
        keeper.require_auth();
        let config = Self::get_config(env);
        if !config.enabled {
            return Err(ContractError::AutoArchiveDisabled);
        }
        let project =
            ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;
        if project.archived {
            return Err(ContractError::AlreadyArchived);
        }
        let now = env.ledger().timestamp();
        if now < Self::notice_start(project.updated_at, config.inactivity_threshold_secs) {
            return Err(ContractError::AutoArchiveNotDue);
        }
        if let Some(existing) = Self::get_notice(env, project_id) {
            if existing.last_activity_at == project.updated_at
                && existing.status != AutoArchiveNoticeStatus::Failed
            {
                return Ok(false);
            }
        }

        let notice = AutoArchiveNotice {
            project_id,
            owner: project.owner,
            last_activity_at: project.updated_at,
            notified_at: now,
            archive_eligible_at: project
                .updated_at
                .saturating_add(config.inactivity_threshold_secs),
            status: AutoArchiveNoticeStatus::Pending,
        };
        env.storage()
            .persistent()
            .set(&AK::Notice(project_id), &notice);
        StorageManager::extend_auto_archive_project_ttl(env, project_id);
        publish_auto_archive_notice_event(env, &notice);
        Ok(true)
    }

    /// Admin/off-chain delivery service records whether the owner notice was
    /// delivered. Delivery state is auditable but does not let a keeper bypass
    /// the on-chain 30-day waiting period.
    pub fn record_notice_delivery(
        env: &Env,
        admin: Address,
        project_id: u64,
        delivered: bool,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;
        let mut notice =
            Self::get_notice(env, project_id).ok_or(ContractError::AutoArchiveNoticeNotFound)?;
        notice.status = if delivered {
            AutoArchiveNoticeStatus::Delivered
        } else {
            AutoArchiveNoticeStatus::Failed
        };
        env.storage()
            .persistent()
            .set(&AK::Notice(project_id), &notice);
        StorageManager::extend_auto_archive_project_ttl(env, project_id);
        publish_auto_archive_notice_delivery_event(env, project_id, admin, notice.status);
        Ok(())
    }

    fn eligible_for_archive(
        env: &Env,
        project: &Project,
        config: &AutoArchiveConfig,
    ) -> Option<AutoArchiveNotice> {
        if !config.enabled || project.archived {
            return None;
        }
        let now = env.ledger().timestamp();
        if now
            < project
                .updated_at
                .saturating_add(config.inactivity_threshold_secs)
        {
            return None;
        }
        let notice = Self::get_notice(env, project.id)?;
        if notice.last_activity_at != project.updated_at
            || now < notice.notified_at.saturating_add(AUTO_ARCHIVE_NOTICE_SECS)
        {
            return None;
        }
        Some(notice)
    }

    fn store_record(env: &Env, record: &AutoArchiveRecord) {
        env.storage()
            .persistent()
            .set(&AK::Record(record.project_id), record);
        StorageManager::extend_auto_archive_project_ttl(env, record.project_id);
        publish_project_auto_archived_event(env, record);
    }

    /// Process a bounded project-ID range. The caller is an authenticated
    /// keeper; only projects with a valid, fully elapsed notice are archived.
    pub fn archive_inactive_batch(
        env: &Env,
        keeper: Address,
        start_project_id: u64,
        limit: u32,
    ) -> Result<AutoArchiveBatchResult, ContractError> {
        keeper.require_auth();
        let config = Self::get_config(env);
        if !config.enabled {
            return Err(ContractError::AutoArchiveDisabled);
        }
        let count: u64 = env
            .storage()
            .persistent()
            .get(&StorageKey::ProjectCount)
            .unwrap_or(0);
        if count == 0 {
            return Ok(AutoArchiveBatchResult {
                scanned_projects: 0,
                archived_count: 0,
                archived_project_ids: Vec::new(env),
                next_project_id: None,
                has_more: false,
            });
        }
        let start = if start_project_id == 0 {
            1
        } else {
            start_project_id
        };
        if start > count {
            return Ok(AutoArchiveBatchResult {
                scanned_projects: 0,
                archived_count: 0,
                archived_project_ids: Vec::new(env),
                next_project_id: None,
                has_more: false,
            });
        }
        let effective_limit = if limit == 0 || limit > MAX_AUTO_ARCHIVE_BATCH_SIZE {
            MAX_AUTO_ARCHIVE_BATCH_SIZE
        } else {
            limit
        };
        let scan_end = core::cmp::min(
            start.saturating_add(MAX_AUTO_ARCHIVE_BATCH_SIZE as u64),
            count.saturating_add(1),
        );
        let mut archived_ids = Vec::new(env);
        let mut scanned = 0u32;
        let mut cursor = start;

        while cursor < scan_end && archived_ids.len() < effective_limit {
            if let Some(project) = ProjectRegistry::get_project(env, cursor) {
                scanned = scanned.saturating_add(1);
                if let Some(notice) = Self::eligible_for_archive(env, &project, &config) {
                    let archived_project =
                        ProjectRegistry::auto_archive_project(env, cursor, keeper.clone())?;
                    let record = AutoArchiveRecord {
                        project_id: cursor,
                        owner: archived_project.owner,
                        last_activity_at: project.updated_at,
                        notified_at: notice.notified_at,
                        archived_at: env.ledger().timestamp(),
                        archived_by: keeper.clone(),
                        restored_at: None,
                    };
                    Self::store_record(env, &record);
                    archived_ids.push_back(cursor);
                }
            }
            cursor = cursor.saturating_add(1);
        }

        let has_more = cursor <= count;
        let next_project_id = if has_more { Some(cursor) } else { None };
        Ok(AutoArchiveBatchResult {
            scanned_projects: scanned,
            archived_count: archived_ids.len(),
            archived_project_ids: archived_ids,
            next_project_id,
            has_more,
        })
    }

    pub fn get_record(env: &Env, project_id: u64) -> Option<AutoArchiveRecord> {
        let record = env.storage().persistent().get(&AK::Record(project_id));
        if record.is_some() {
            StorageManager::extend_auto_archive_project_ttl(env, project_id);
        }
        record
    }

    /// Called by ProjectRegistry after either automatic or manual restoration.
    pub fn on_project_restored(env: &Env, project_id: u64) {
        env.storage().persistent().remove(&AK::Notice(project_id));
        if let Some(mut record) = env
            .storage()
            .persistent()
            .get::<AK, AutoArchiveRecord>(&AK::Record(project_id))
        {
            record.restored_at = Some(env.ledger().timestamp());
            env.storage()
                .persistent()
                .set(&AK::Record(project_id), &record);
            StorageManager::extend_auto_archive_project_ttl(env, project_id);
        }
    }

    /// Owner/admin restoration entry point dedicated to the auto-archive flow.
    pub fn restore_archived_project(
        env: &Env,
        project_id: u64,
        caller: Address,
    ) -> Result<Project, ContractError> {
        ProjectRegistry::reactivate_project(env, project_id, caller)?;
        ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)
    }

    fn contains_ignore_case(haystack: &String, needle: &String, env: &Env) -> bool {
        if needle.is_empty() {
            return true;
        }
        let haystack = Utils::to_lowercase(env, haystack);
        let needle = Utils::to_lowercase(env, needle);
        let haystack_len = haystack.len() as usize;
        let needle_len = needle.len() as usize;
        if needle_len == 0 || haystack_len < needle_len || haystack_len > 256 {
            return false;
        }
        let mut haystack_bytes = [0u8; 256];
        let mut needle_bytes = [0u8; 256];
        haystack.copy_into_slice(&mut haystack_bytes[..haystack_len]);
        needle.copy_into_slice(&mut needle_bytes[..needle_len]);
        for start in 0..=(haystack_len - needle_len) {
            let mut matches = true;
            for offset in 0..needle_len {
                if haystack_bytes[start + offset] != needle_bytes[offset] {
                    matches = false;
                    break;
                }
            }
            if matches {
                return true;
            }
        }
        false
    }

    fn project_matches(project: &Project, query: &String, env: &Env) -> bool {
        Self::contains_ignore_case(&project.name, query, env)
            || Self::contains_ignore_case(&project.slug, query, env)
            || Self::contains_ignore_case(&project.category, query, env)
            || Self::contains_ignore_case(&project.description, query, env)
    }

    /// Cursor-based substring search over archived projects. The scan is bounded
    /// to avoid unbounded ledger reads; clients continue with `next_cursor`.
    pub fn search_archived_projects(
        env: &Env,
        query: String,
        start_cursor: u64,
        limit: u32,
    ) -> Result<ArchivedProjectSearchResult, ContractError> {
        if query.len() as usize > 256 {
            return Err(ContractError::InvalidInput);
        }
        let count: u64 = env
            .storage()
            .persistent()
            .get(&StorageKey::ProjectCount)
            .unwrap_or(0);
        let start = if start_cursor == 0 { 1 } else { start_cursor };
        if count == 0 || start > count {
            return Ok(ArchivedProjectSearchResult {
                projects: Vec::new(env),
                next_cursor: None,
                has_more: false,
                scanned_projects: 0,
            });
        }
        let effective_limit = if limit == 0 || limit > MAX_PAGE_LIMIT {
            MAX_PAGE_LIMIT
        } else {
            limit
        };
        let end = core::cmp::min(
            start.saturating_add(MAX_ARCHIVED_SEARCH_SCAN as u64),
            count.saturating_add(1),
        );
        let mut projects = Vec::new(env);
        let mut scanned = 0u32;
        let mut cursor = start;
        while cursor < end && projects.len() < effective_limit {
            if let Some(project) = ProjectRegistry::get_project(env, cursor) {
                scanned = scanned.saturating_add(1);
                if project.archived && Self::project_matches(&project, &query, env) {
                    projects.push_back(project);
                }
            }
            cursor = cursor.saturating_add(1);
        }
        let has_more = cursor <= count;
        Ok(ArchivedProjectSearchResult {
            projects,
            next_cursor: if has_more { Some(cursor) } else { None },
            has_more,
            scanned_projects: scanned,
        })
    }
}
