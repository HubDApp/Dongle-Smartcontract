//! Project changelog registry for publishing update notes and release history.
//!
//! # Immutability Contract (#655)
//!
//! Changelog entries are **write-once**. Once an entry is created via
//! [`ChangelogRegistry::add_changelog_entry`] it cannot be modified. The only
//! mutation allowed is deletion via [`ChangelogRegistry::remove_changelog_entry`].
//!
//! If a changelog entry needs to be corrected, the workflow is:
//! 1. Call `remove_changelog_entry` to delete the existing entry.
//! 2. Call `add_changelog_entry` with the corrected CID / metadata.
//!
//! There is deliberately **no** `update_changelog_entry` function. Attempting to
//! call a non-existent update function will produce a compile-time error, which
//! is the strongest possible enforcement of this contract.
//!
//! All changes (add and remove) emit on-chain events so indexers can reconstruct
//! the full audit trail.
//!
//! # Admin approval (#930)
//!
//! Every entry carries a [`ChangelogStatus`]: `Draft` → `Approved` → `Published`.
//! Only `Published` entries appear in [`ChangelogRegistry::get_project_changelog`]
//! and [`ChangelogRegistry::get_changelog_count`]. Status transitions never
//! touch the entry's content, so the write-once contract above still holds.
//!
//! - Owners call [`ChangelogRegistry::submit_changelog_for_approval`], which
//!   stores a `Draft` and queues it for admin review.
//! - Admins review drafts in bulk with
//!   [`ChangelogRegistry::approve_changelog_entries`] (optionally publishing
//!   immediately) and [`ChangelogRegistry::reject_changelog_entries`].
//! - Approved entries are published in bulk by the owner or an admin via
//!   [`ChangelogRegistry::publish_changelog_entries`], emitting
//!   `ChangelogEntryPublished`.
//!
//! When an admin enables approval via
//! [`ChangelogRegistry::set_changelog_approval_required`], the direct
//! `add_changelog_entry` path is closed (`ChangelogApprovalRequired`). It is
//! open by default so existing integrations keep working.

use crate::admin_manager::AdminManager;
use crate::auth::require_admin_auth;
use crate::constants::{MAX_CHANGELOG_APPROVAL_BATCH, MAX_CID_LEN};
use crate::errors::ContractError;
use crate::events::{
    publish_changelog_added_event, publish_changelog_entry_published_event,
    publish_changelog_removed_event, publish_changelog_reviewed_event,
    publish_changelog_submitted_event,
};
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::{ChangelogApprovalKey, ExtensionKey, ExtensionKey2};
use crate::storage_manager::StorageManager;
use crate::types::{ChangelogBatchResult, ChangelogEntry, ChangelogStatus};
use crate::utils::Utils;
use soroban_sdk::{Address, Env, String, Vec};

pub struct ChangelogRegistry;

impl ChangelogRegistry {
    /// Add a new changelog entry for a project.
    ///
    /// # Arguments
    /// - `env`: The Soroban environment
    /// - `project_id`: The project ID to add changelog for
    /// - `owner`: The project owner (must be authenticated)
    /// - `cid`: IPFS CID containing the changelog content
    /// - `description`: Optional description/title for the changelog entry
    /// - `version`: Optional semver string for this release (e.g. "1.2.3")
    /// - `changelog_cid`: Optional secondary IPFS CID for a machine-readable release-notes document
    ///
    /// # Returns
    /// - `Ok(u64)` with the new changelog entry ID on success
    /// - `Err(ContractError)` on failure
    pub fn add_changelog_entry(
        env: &Env,
        project_id: u64,
        owner: Address,
        cid: String,
        description: Option<String>,
        version: Option<String>,
        changelog_cid: Option<String>,
    ) -> Result<u64, ContractError> {
        // Authentication check
        owner.require_auth();

        if Self::is_changelog_approval_required(env) {
            return Err(ContractError::ChangelogApprovalRequired);
        }

        let now = env.ledger().timestamp();
        let entry = Self::create_entry(
            env,
            project_id,
            &owner,
            cid,
            description,
            version,
            changelog_cid,
            ChangelogStatus::Published,
            Some(now),
        )?;

        publish_changelog_added_event(env, entry.id, project_id, owner, entry.cid);

        Ok(entry.id)
    }

    /// Submit a changelog entry for admin approval (project owner only).
    ///
    /// The entry is stored as `Draft`, queued for admin review and hidden from
    /// the public changelog until it is approved and published.
    pub fn submit_changelog_for_approval(
        env: &Env,
        project_id: u64,
        owner: Address,
        cid: String,
        description: Option<String>,
        version: Option<String>,
        changelog_cid: Option<String>,
    ) -> Result<u64, ContractError> {
        owner.require_auth();

        let entry = Self::create_entry(
            env,
            project_id,
            &owner,
            cid,
            description,
            version,
            changelog_cid,
            ChangelogStatus::Draft,
            None,
        )?;

        let mut queue = Self::get_pending_queue(env);
        queue.push_back(entry.id);
        env.storage()
            .persistent()
            .set(&ChangelogApprovalKey::PendingQueue, &queue);

        publish_changelog_submitted_event(env, entry.id, project_id, owner, entry.cid);

        Ok(entry.id)
    }

    /// Validate and store a new entry with the given status.
    #[allow(clippy::too_many_arguments)]
    fn create_entry(
        env: &Env,
        project_id: u64,
        owner: &Address,
        cid: String,
        description: Option<String>,
        version: Option<String>,
        changelog_cid: Option<String>,
        status: ChangelogStatus,
        published_at: Option<u64>,
    ) -> Result<ChangelogEntry, ContractError> {
        // Verify project exists and caller is owner
        let project =
            ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        if project.owner != *owner {
            return Err(ContractError::Unauthorized);
        }

        // Validate primary CID
        if cid.is_empty() {
            return Err(ContractError::InvalidCid);
        }
        if !Utils::is_valid_ipfs_cid(&cid) {
            return Err(ContractError::InvalidCid);
        }
        if cid.len() as usize > MAX_CID_LEN {
            return Err(ContractError::InvalidCid);
        }

        // Validate optional version string (must be non-empty when provided)
        if let Some(ref v) = version {
            if v.is_empty() {
                return Err(ContractError::InvalidProjectData);
            }
        }

        // Validate optional secondary changelog CID when provided
        if let Some(ref ccid) = changelog_cid {
            if ccid.is_empty()
                || !Utils::is_valid_ipfs_cid(ccid)
                || ccid.len() as usize > MAX_CID_LEN
            {
                return Err(ContractError::InvalidCid);
            }
        }

        // Check for duplicate CID in existing changelog entries
        let existing_entry_ids = Self::get_project_changelog_entries(env, project_id);
        for i in 0..existing_entry_ids.len() {
            if let Some(changelog_id) = existing_entry_ids.get(i) {
                if let Some(entry) = Self::get_changelog_entry(env, changelog_id) {
                    if entry.cid == cid {
                        return Err(ContractError::DuplicateReview);
                    }
                }
            }
        }

        // Get next changelog entry ID
        let changelog_id: u64 = env
            .storage()
            .persistent()
            .get(&ExtensionKey2::NextChangelogEntryId)
            .unwrap_or(1);

        // Create changelog entry
        let now = env.ledger().timestamp();
        let entry = ChangelogEntry {
            id: changelog_id,
            project_id,
            cid,
            created_at: now,
            description,
            version,
            changelog_cid,
            status,
            approved_by: None,
            published_at,
        };

        // Store changelog entry
        env.storage()
            .persistent()
            .set(&ExtensionKey::ProjectChangelogEntry(changelog_id), &entry);

        // Add to project's changelog list
        let mut project_changelogs = Self::get_project_changelog_entries(env, project_id);
        project_changelogs.push_back(changelog_id);
        env.storage().persistent().set(
            &ExtensionKey::ProjectChangelogEntries(project_id),
            &project_changelogs,
        );

        // Increment next ID
        env.storage()
            .persistent()
            .set(&ExtensionKey2::NextChangelogEntryId, &(changelog_id + 1));

        // Extend TTL
        StorageManager::extend_project_ttl(env, project_id);

        Ok(entry)
    }

    /// Admin bulk approval of `Draft` entries.
    ///
    /// With `publish = true` approved entries are published immediately.
    /// IDs that do not exist or are not `Draft` are reported in `failed`
    /// instead of aborting the batch.
    pub fn approve_changelog_entries(
        env: &Env,
        admin: Address,
        changelog_ids: Vec<u64>,
        publish: bool,
    ) -> Result<ChangelogBatchResult, ContractError> {
        require_admin_auth(env, &admin)?;
        Self::validate_batch(&changelog_ids)?;

        let mut result = Self::empty_batch_result(env);
        let mut queue = Self::get_pending_queue(env);
        for id in changelog_ids.iter() {
            let Some(mut entry) = Self::get_changelog_entry(env, id) else {
                result.failed.push_back(id);
                continue;
            };
            if entry.status != ChangelogStatus::Draft {
                result.failed.push_back(id);
                continue;
            }

            entry.status = ChangelogStatus::Approved;
            entry.approved_by = Some(admin.clone());
            publish_changelog_reviewed_event(env, id, entry.project_id, admin.clone(), true);
            if publish {
                Self::mark_published(env, &mut entry, admin.clone());
            }
            env.storage()
                .persistent()
                .set(&ExtensionKey::ProjectChangelogEntry(id), &entry);

            queue = Utils::remove_item_from_vec(env, &queue, &id);
            result.succeeded.push_back(id);
        }
        env.storage()
            .persistent()
            .set(&ChangelogApprovalKey::PendingQueue, &queue);

        Ok(result)
    }

    /// Admin bulk rejection of `Draft` entries. Rejected drafts are deleted.
    pub fn reject_changelog_entries(
        env: &Env,
        admin: Address,
        changelog_ids: Vec<u64>,
    ) -> Result<ChangelogBatchResult, ContractError> {
        require_admin_auth(env, &admin)?;
        Self::validate_batch(&changelog_ids)?;

        let mut result = Self::empty_batch_result(env);
        let mut queue = Self::get_pending_queue(env);
        for id in changelog_ids.iter() {
            let Some(entry) = Self::get_changelog_entry(env, id) else {
                result.failed.push_back(id);
                continue;
            };
            if entry.status != ChangelogStatus::Draft {
                result.failed.push_back(id);
                continue;
            }

            Self::delete_entry(env, &entry);
            queue = Utils::remove_item_from_vec(env, &queue, &id);
            publish_changelog_reviewed_event(env, id, entry.project_id, admin.clone(), false);
            result.succeeded.push_back(id);
        }
        env.storage()
            .persistent()
            .set(&ChangelogApprovalKey::PendingQueue, &queue);

        Ok(result)
    }

    /// Bulk-publish `Approved` entries. Callable by an admin, or by the owner
    /// of each entry's project (entries of other projects are reported as
    /// failed).
    pub fn publish_changelog_entries(
        env: &Env,
        caller: Address,
        changelog_ids: Vec<u64>,
    ) -> Result<ChangelogBatchResult, ContractError> {
        caller.require_auth();
        Self::validate_batch(&changelog_ids)?;
        let is_admin = AdminManager::is_admin(env, &caller);

        let mut result = Self::empty_batch_result(env);
        for id in changelog_ids.iter() {
            let Some(mut entry) = Self::get_changelog_entry(env, id) else {
                result.failed.push_back(id);
                continue;
            };
            let is_owner = ProjectRegistry::get_project(env, entry.project_id)
                .map(|p| p.owner == caller)
                .unwrap_or(false);
            if entry.status != ChangelogStatus::Approved || !(is_admin || is_owner) {
                result.failed.push_back(id);
                continue;
            }

            Self::mark_published(env, &mut entry, caller.clone());
            env.storage()
                .persistent()
                .set(&ExtensionKey::ProjectChangelogEntry(id), &entry);
            result.succeeded.push_back(id);
        }

        Ok(result)
    }

    /// Require (or stop requiring) admin approval for new changelog entries.
    pub fn set_changelog_approval_required(
        env: &Env,
        admin: Address,
        required: bool,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;
        env.storage()
            .persistent()
            .set(&ChangelogApprovalKey::ApprovalRequired, &required);
        Ok(())
    }

    pub fn is_changelog_approval_required(env: &Env) -> bool {
        env.storage()
            .persistent()
            .get(&ChangelogApprovalKey::ApprovalRequired)
            .unwrap_or(false)
    }

    /// Draft entries awaiting admin review, oldest first.
    pub fn get_pending_changelog_entries(
        env: &Env,
        start_index: u32,
        limit: u32,
    ) -> Vec<ChangelogEntry> {
        let queue = Self::get_pending_queue(env);
        let page = crate::pagination::paginate(env, &queue, start_index, limit);
        let mut result = Vec::new(env);
        for id in page.iter() {
            if let Some(entry) = Self::get_changelog_entry(env, id) {
                result.push_back(entry);
            }
        }
        result
    }

    fn mark_published(env: &Env, entry: &mut ChangelogEntry, published_by: Address) {
        entry.status = ChangelogStatus::Published;
        entry.published_at = Some(env.ledger().timestamp());
        publish_changelog_entry_published_event(env, entry, published_by);
    }

    fn validate_batch(changelog_ids: &Vec<u64>) -> Result<(), ContractError> {
        if changelog_ids.is_empty() || changelog_ids.len() > MAX_CHANGELOG_APPROVAL_BATCH {
            return Err(ContractError::InvalidChangelogBatch);
        }
        Ok(())
    }

    fn empty_batch_result(env: &Env) -> ChangelogBatchResult {
        ChangelogBatchResult {
            succeeded: Vec::new(env),
            failed: Vec::new(env),
        }
    }

    fn get_pending_queue(env: &Env) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&ChangelogApprovalKey::PendingQueue)
            .unwrap_or_else(|| Vec::new(env))
    }

    /// Remove an entry from storage and from its project's entry list.
    fn delete_entry(env: &Env, entry: &ChangelogEntry) {
        let project_changelogs = Self::get_project_changelog_entries(env, entry.project_id);
        let new_changelogs = Utils::remove_item_from_vec(env, &project_changelogs, &entry.id);
        env.storage().persistent().set(
            &ExtensionKey::ProjectChangelogEntries(entry.project_id),
            &new_changelogs,
        );
        env.storage()
            .persistent()
            .remove(&ExtensionKey::ProjectChangelogEntry(entry.id));
    }

    /// Remove a changelog entry (project owner only).
    ///
    /// # Arguments
    /// - `env`: The Soroban environment
    /// - `changelog_id`: The changelog entry ID to remove
    /// - `owner`: The project owner (must be authenticated)
    ///
    /// # Returns
    /// - `Ok(())` on success
    /// - `Err(ContractError)` on failure
    pub fn remove_changelog_entry(
        env: &Env,
        changelog_id: u64,
        owner: Address,
    ) -> Result<(), ContractError> {
        // Authentication check
        owner.require_auth();

        // Get changelog entry
        let entry: ChangelogEntry = env
            .storage()
            .persistent()
            .get(&ExtensionKey::ProjectChangelogEntry(changelog_id))
            .ok_or(ContractError::ReviewNotFound)?;

        // Verify caller is project owner
        let project = ProjectRegistry::get_project(env, entry.project_id)
            .ok_or(ContractError::ProjectNotFound)?;

        if project.owner != owner {
            return Err(ContractError::Unauthorized);
        }

        Self::delete_entry(env, &entry);

        // A withdrawn draft no longer needs admin review
        if entry.status == ChangelogStatus::Draft {
            let queue = Utils::remove_item_from_vec(env, &Self::get_pending_queue(env), &changelog_id);
            env.storage()
                .persistent()
                .set(&ChangelogApprovalKey::PendingQueue, &queue);
        }

        // Extend TTL
        StorageManager::extend_project_ttl(env, entry.project_id);

        // Publish event
        publish_changelog_removed_event(env, changelog_id, entry.project_id, owner);

        Ok(())
    }

    /// Get paginated changelog entries for a project.
    ///
    /// # Arguments
    /// - `env`: The Soroban environment
    /// - `project_id`: The project ID to get changelog for
    /// - `start_index`: Starting index for pagination
    /// - `limit`: Maximum number of entries to return (capped at MAX_PAGE_LIMIT)
    /// - `sort_mode`: Sort order (Newest or Oldest)
    ///
    /// # Returns
    /// - `Vec<ChangelogEntry>` paginated and sorted changelog entries
    pub fn get_project_changelog(
        env: &Env,
        project_id: u64,
        start_index: u32,
        limit: u32,
        sort_mode: crate::types::ChangelogSortMode,
    ) -> Vec<ChangelogEntry> {
        use crate::constants::MAX_PAGE_LIMIT;

        // Validate project exists
        if ProjectRegistry::get_project(env, project_id).is_none() {
            return Vec::new(env);
        }

        let effective_limit = if limit == 0 || limit > MAX_PAGE_LIMIT {
            MAX_PAGE_LIMIT
        } else {
            limit
        };

        // Collect published entries only; drafts and approved-but-unpublished
        // entries are not part of the public changelog (#930).
        let changelog_ids = Self::get_project_changelog_entries(env, project_id);
        let mut entries = Vec::new(env);
        for changelog_id in changelog_ids.iter() {
            if let Some(entry) = Self::get_changelog_entry(env, changelog_id) {
                if entry.status == ChangelogStatus::Published {
                    entries.push_back(entry);
                }
            }
        }

        let total = entries.len();
        if total == 0 || start_index >= total {
            return Vec::new(env);
        }

        // Sort entries based on sort_mode
        match sort_mode {
            crate::types::ChangelogSortMode::Newest => {
                // For newest first (descending), swap when a.created_at < b.created_at
                Utils::bubble_sort_by(&mut entries, |a, b| a.created_at < b.created_at);
            }
            crate::types::ChangelogSortMode::Oldest => {
                // For oldest first (ascending), swap when a.created_at > b.created_at
                Utils::bubble_sort_by(&mut entries, |a, b| a.created_at > b.created_at);
            }
        }

        // Apply pagination
        let end = (start_index + effective_limit).min(total);
        let mut paginated = Vec::new(env);

        for i in start_index..end {
            if let Some(entry) = entries.get(i) {
                paginated.push_back(entry.clone());
            }
        }

        paginated
    }

    /// Get a single changelog entry by ID.
    ///
    /// # Arguments
    /// - `env`: The Soroban environment
    /// - `changelog_id`: The changelog entry ID
    ///
    /// # Returns
    /// - `Option<ChangelogEntry>` the changelog entry if found
    pub fn get_changelog_entry(env: &Env, changelog_id: u64) -> Option<ChangelogEntry> {
        env.storage()
            .persistent()
            .get(&ExtensionKey::ProjectChangelogEntry(changelog_id))
    }

    /// Get changelog entry count for a project.
    ///
    /// # Arguments
    /// - `env`: The Soroban environment
    /// - `project_id`: The project ID
    ///
    /// # Returns
    /// - `u32` number of published changelog entries
    pub fn get_changelog_count(env: &Env, project_id: u64) -> u32 {
        let mut count = 0;
        for changelog_id in Self::get_project_changelog_entries(env, project_id).iter() {
            if let Some(entry) = Self::get_changelog_entry(env, changelog_id) {
                if entry.status == ChangelogStatus::Published {
                    count += 1;
                }
            }
        }
        count
    }

    /// Explicitly reject any attempt to update a changelog entry in-place.
    ///
    /// # Immutability enforcement (#655)
    ///
    /// Changelog entries are write-once. This function exists purely to provide
    /// a clear, discoverable API surface that documents the "no update" contract
    /// and returns a typed error when called, rather than silently not existing.
    /// Intentionally unimplemented update function — changelog entries are
    /// write-once (immutable after creation).
    ///
    /// To change a changelog entry:
    /// 1. Call [`remove_changelog_entry`] to delete it.
    /// 2. Call [`add_changelog_entry`] with the corrected data.
    ///
    /// # Returns
    /// Always returns `Err(ContractError::Unauthorized)` — changelog entries
    /// cannot be updated; they can only be removed and recreated.
    // Dead-code justification: intentional stub that enforces the immutability
    // invariant.  It is kept (rather than deleted) so that call-sites that
    // accidentally invoke "update" get a clear compile-time or runtime error
    // instead of silently missing the function.
    #[allow(dead_code)]
    pub fn update_changelog_entry(
        _env: &Env,
        _changelog_id: u64,
        _owner: Address,
    ) -> Result<(), ContractError> {
        // Changelog entries are write-once (immutable after creation).
        // Use remove_changelog_entry + add_changelog_entry instead.
        Err(ContractError::Unauthorized)
    }

    // Internal helper function to get project changelog entry IDs
    pub(crate) fn get_project_changelog_entries(env: &Env, project_id: u64) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&ExtensionKey::ProjectChangelogEntries(project_id))
            .unwrap_or_else(|| Vec::new(env))
    }
}
