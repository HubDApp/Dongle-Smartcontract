//! Project changelog registry for publishing update notes and release history.

use crate::constants::MAX_CID_LEN;
use crate::errors::ContractError;
use crate::events::{publish_changelog_added_event, publish_changelog_removed_event};
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::ExtensionKey;
use crate::storage_manager::StorageManager;
use crate::types::ChangelogEntry;
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

        // Verify project exists and caller is owner
        let project =
            ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        if project.owner != owner {
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
            .get(&ExtensionKey::NextChangelogEntryId)
            .unwrap_or(1);

        // Create changelog entry
        let now = env.ledger().timestamp();
        let entry = ChangelogEntry {
            id: changelog_id,
            project_id,
            cid: cid.clone(),
            created_at: now,
            description,
            version,
            changelog_cid,
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
            .set(&ExtensionKey::NextChangelogEntryId, &(changelog_id + 1));

        // Extend TTL
        StorageManager::extend_project_ttl(env, project_id);

        // Publish event
        publish_changelog_added_event(env, changelog_id, project_id, owner, cid);

        Ok(changelog_id)
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

        // Remove from project's changelog list
        let project_changelogs = Self::get_project_changelog_entries(env, entry.project_id);
        let new_changelogs = Utils::remove_item_from_vec(env, &project_changelogs, &changelog_id);
        env.storage().persistent().set(
            &ExtensionKey::ProjectChangelogEntries(entry.project_id),
            &new_changelogs,
        );

        // Remove the changelog entry
        env.storage()
            .persistent()
            .remove(&ExtensionKey::ProjectChangelogEntry(changelog_id));

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

        // Get all changelog entry IDs for the project
        let changelog_ids = Self::get_project_changelog_entries(env, project_id);
        let total = changelog_ids.len();

        if total == 0 || start_index >= total {
            return Vec::new(env);
        }

        // Collect all entries
        let mut entries = Vec::new(env);
        for i in 0..total {
            if let Some(changelog_id) = changelog_ids.get(i) {
                if let Some(entry) = Self::get_changelog_entry(env, changelog_id) {
                    entries.push_back(entry);
                }
            }
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
    /// - `u32` number of changelog entries
    pub fn get_changelog_count(env: &Env, project_id: u64) -> u32 {
        Self::get_project_changelog_entries(env, project_id).len()
    }

    // Internal helper function to get project changelog entry IDs
    fn get_project_changelog_entries(env: &Env, project_id: u64) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&ExtensionKey::ProjectChangelogEntries(project_id))
            .unwrap_or_else(|| Vec::new(env))
    }
}
