//! Bookmark registry — flat bookmark list plus folder organisation (#815).
//!
//! ## Design overview
//!
//! **Flat list** (`UserBookmarks` under `ExtensionKey`) remains the source of
//! truth for *which* projects a user has bookmarked.  Folders are an
//! organisational layer on top: they hold a subset of the user's bookmarked
//! project IDs.
//!
//! **Nested folders** are represented by a `parent_id` field on
//! `BookmarkFolder`.  Depth is validated on creation by walking up the
//! parent chain (capped at `MAX_FOLDER_DEPTH`).
//!
//! **Smart folders** store a `SmartFolderFilter` and resolve dynamically —
//! every `get_smart_folder_bookmarks` call scans the user's full bookmark list
//! and filters it in-contract.
//!
//! **`move_bookmark`** moves a project ID from one folder to another (or to
//! the root / no-folder state).  The project must already be bookmarked in the
//! flat list.  A per-user index (`BookmarkFolderIndex(owner, project_id)`)
//! tracks the current folder so callers do not need to search every folder.

use crate::constants::{
    MAX_BOOKMARKS_PER_FOLDER, MAX_BOOKMARK_FOLDERS_PER_USER, MAX_FOLDER_DEPTH,
    MAX_FOLDER_NAME_LEN, MAX_PAGE_LIMIT, MAX_SMART_FOLDERS_PER_USER,
};
use crate::errors::ContractError;
use crate::events::{
    publish_bookmark_moved_to_folder_event, publish_bookmark_removed_from_folder_event,
    publish_folder_created_event, publish_folder_deleted_event, publish_folder_renamed_event,
    publish_project_bookmarked_event, publish_project_unbookmarked_event,
    publish_smart_folder_created_event, publish_smart_folder_deleted_event,
};
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::{BookmarkKey, ExtensionKey};
use crate::storage_manager::StorageManager;
use crate::types::{BookmarkFolder, SmartFolder, SmartFolderFilter, VerificationStatusFilter};
use crate::utils::Utils;
use soroban_sdk::{Address, Env, Vec};

pub struct BookmarkRegistry;

impl BookmarkRegistry {
    // ── Flat bookmark helpers (existing API, unchanged) ───────────────────

    pub fn bookmark_project(
        env: &Env,
        project_id: u64,
        user: Address,
    ) -> Result<(), ContractError> {
        user.require_auth();

        if ProjectRegistry::get_project(env, project_id).is_none() {
            return Err(ContractError::ProjectNotFound);
        }

        if Self::is_bookmarked(env, project_id, &user) {
            return Err(ContractError::AlreadyBookmarked);
        }

        let mut bookmarks: Vec<u64> = env
            .storage()
            .persistent()
            .get(&ExtensionKey::UserBookmarks(user.clone()))
            .unwrap_or_else(|| Vec::new(env));
        let _ = Utils::add_unique_to_vec(&mut bookmarks, &project_id);
        env.storage()
            .persistent()
            .set(&ExtensionKey::UserBookmarks(user.clone()), &bookmarks);

        StorageManager::extend_project_ttl(env, project_id);
        StorageManager::extend_user_bookmarks_ttl(env, &user);

        publish_project_bookmarked_event(env, project_id, user);
        Ok(())
    }

    pub fn unbookmark_project(
        env: &Env,
        project_id: u64,
        user: Address,
    ) -> Result<(), ContractError> {
        user.require_auth();

        if ProjectRegistry::get_project(env, project_id).is_none() {
            return Err(ContractError::ProjectNotFound);
        }

        if !Self::is_bookmarked(env, project_id, &user) {
            return Err(ContractError::NotBookmarked);
        }

        // Remove from any folder it's currently in.
        if let Some(folder_id) = Self::get_bookmark_folder(env, project_id, &user) {
            let _ = Self::remove_from_folder_list(env, &user, folder_id, project_id);
            env.storage()
                .persistent()
                .remove(&BookmarkKey::BookmarkFolderIndex(user.clone(), project_id));
        }

        let bookmarks: Vec<u64> = env
            .storage()
            .persistent()
            .get(&ExtensionKey::UserBookmarks(user.clone()))
            .unwrap_or_else(|| Vec::new(env));

        let new_bookmarks = Utils::remove_item_from_vec(env, &bookmarks, &project_id);
        env.storage()
            .persistent()
            .set(&ExtensionKey::UserBookmarks(user.clone()), &new_bookmarks);

        StorageManager::extend_project_ttl(env, project_id);
        StorageManager::extend_user_bookmarks_ttl(env, &user);

        publish_project_unbookmarked_event(env, project_id, user);
        Ok(())
    }

    pub fn is_bookmarked(env: &Env, project_id: u64, user: &Address) -> bool {
        let bookmarks: Vec<u64> = env
            .storage()
            .persistent()
            .get(&ExtensionKey::UserBookmarks(user.clone()))
            .unwrap_or_else(|| Vec::new(env));
        bookmarks.contains(project_id)
    }

    pub fn get_user_bookmarks(env: &Env, user: Address, start_index: u32, limit: u32) -> Vec<u64> {
        let effective_limit = if limit == 0 || limit > MAX_PAGE_LIMIT {
            MAX_PAGE_LIMIT
        } else {
            limit
        };

        let bookmarks: Vec<u64> = env
            .storage()
            .persistent()
            .get(&ExtensionKey::UserBookmarks(user))
            .unwrap_or_else(|| Vec::new(env));

        let len = bookmarks.len();
        if start_index >= len {
            return Vec::new(env);
        }

        let end = core::cmp::min(start_index.saturating_add(effective_limit), len);
        let mut page = Vec::new(env);
        for i in start_index..end {
            if let Some(pid) = bookmarks.get(i) {
                page.push_back(pid);
            }
        }
        page
    }

    // ── Folder CRUD ───────────────────────────────────────────────────────

    /// Create a new bookmark folder for `user`.  Returns the new folder ID.
    ///
    /// # Arguments
    /// * `user`      – owner of the folder (caller; auth required).
    /// * `name`      – folder name (max `MAX_FOLDER_NAME_LEN` bytes).
    /// * `parent_id` – optional parent folder ID for nested folders.
    pub fn create_folder(
        env: &Env,
        user: Address,
        name: soroban_sdk::String,
        parent_id: Option<u64>,
    ) -> Result<u64, ContractError> {
        user.require_auth();

        Self::validate_folder_name(&name)?;

        // Enforce per-user folder limit.
        let folder_ids = Self::get_user_folder_ids(env, &user);
        if folder_ids.len() >= MAX_BOOKMARK_FOLDERS_PER_USER {
            return Err(ContractError::MaxFoldersExceeded);
        }

        // Validate parent + compute depth.
        let depth = if let Some(pid) = parent_id {
            let parent = Self::require_folder(env, &user, pid)?;
            Self::compute_folder_depth(env, &user, parent.parent_id, 1)?
        } else {
            0u32
        };

        if depth >= MAX_FOLDER_DEPTH {
            return Err(ContractError::FolderDepthExceeded);
        }

        let id = Self::next_folder_id(env, &user);
        let ts = env.ledger().timestamp();
        let folder = BookmarkFolder {
            id,
            owner: user.clone(),
            name: name.clone(),
            parent_id,
            created_at: ts,
            updated_at: ts,
        };

        env.storage()
            .persistent()
            .set(&BookmarkKey::BookmarkFolder(user.clone(), id), &folder);
        env.storage()
            .persistent()
            .set(
                &BookmarkKey::FolderBookmarks(user.clone(), id),
                &Vec::<u64>::new(env),
            );

        // Append to user's folder-ID list.
        let mut ids = folder_ids;
        ids.push_back(id);
        env.storage()
            .persistent()
            .set(&BookmarkKey::UserFolderIds(user.clone()), &ids);

        StorageManager::extend_user_folder_ids_ttl(env, &user);
        StorageManager::extend_folder_ttl(env, &user, id);

        publish_folder_created_event(env, id, user, name, parent_id);
        Ok(id)
    }

    /// Delete a bookmark folder.  Bookmarks inside the folder are NOT
    /// deleted — they remain in the flat list.  The folder index entries for
    /// those projects are cleared.
    pub fn delete_folder(env: &Env, user: Address, folder_id: u64) -> Result<(), ContractError> {
        user.require_auth();
        Self::require_folder(env, &user, folder_id)?;

        // Clear folder index for every project in the folder.
        let bookmarks: Vec<u64> = env
            .storage()
            .persistent()
            .get(&BookmarkKey::FolderBookmarks(user.clone(), folder_id))
            .unwrap_or_else(|| Vec::new(env));
        for i in 0..bookmarks.len() {
            if let Some(pid) = bookmarks.get(i) {
                env.storage()
                    .persistent()
                    .remove(&BookmarkKey::BookmarkFolderIndex(user.clone(), pid));
            }
        }

        // Remove folder record and bookmark list.
        env.storage()
            .persistent()
            .remove(&BookmarkKey::BookmarkFolder(user.clone(), folder_id));
        env.storage()
            .persistent()
            .remove(&BookmarkKey::FolderBookmarks(user.clone(), folder_id));

        // Remove folder from user's ID list.
        let ids = Self::get_user_folder_ids(env, &user);
        let updated = Utils::remove_item_from_vec(env, &ids, &folder_id);
        env.storage()
            .persistent()
            .set(&BookmarkKey::UserFolderIds(user.clone()), &updated);

        StorageManager::extend_user_folder_ids_ttl(env, &user);

        publish_folder_deleted_event(env, folder_id, user);
        Ok(())
    }

    /// Rename a bookmark folder.
    pub fn rename_folder(
        env: &Env,
        user: Address,
        folder_id: u64,
        new_name: soroban_sdk::String,
    ) -> Result<(), ContractError> {
        user.require_auth();
        Self::validate_folder_name(&new_name)?;

        let mut folder = Self::require_folder(env, &user, folder_id)?;
        folder.name = new_name.clone();
        folder.updated_at = env.ledger().timestamp();

        env.storage()
            .persistent()
            .set(&BookmarkKey::BookmarkFolder(user.clone(), folder_id), &folder);

        StorageManager::extend_folder_ttl(env, &user, folder_id);

        publish_folder_renamed_event(env, folder_id, user, new_name);
        Ok(())
    }

    /// Get a single folder, or `None` if it does not exist.
    pub fn get_folder(env: &Env, user: Address, folder_id: u64) -> Option<BookmarkFolder> {
        env.storage()
            .persistent()
            .get(&BookmarkKey::BookmarkFolder(user, folder_id))
    }

    /// List all folders owned by `user`.
    pub fn list_folders(env: &Env, user: Address) -> Vec<BookmarkFolder> {
        let ids = Self::get_user_folder_ids(env, &user);
        let mut result = Vec::new(env);
        for i in 0..ids.len() {
            if let Some(fid) = ids.get(i) {
                if let Some(f) = env
                    .storage()
                    .persistent()
                    .get::<_, BookmarkFolder>(&BookmarkKey::BookmarkFolder(user.clone(), fid))
                {
                    result.push_back(f);
                }
            }
        }
        result
    }

    /// List direct children (sub-folders) of a given parent folder.
    pub fn list_child_folders(
        env: &Env,
        user: Address,
        parent_id: u64,
    ) -> Vec<BookmarkFolder> {
        let ids = Self::get_user_folder_ids(env, &user);
        let mut result = Vec::new(env);
        for i in 0..ids.len() {
            if let Some(fid) = ids.get(i) {
                if let Some(f) = env
                    .storage()
                    .persistent()
                    .get::<_, BookmarkFolder>(&BookmarkKey::BookmarkFolder(user.clone(), fid))
                {
                    if f.parent_id == Some(parent_id) {
                        result.push_back(f);
                    }
                }
            }
        }
        result
    }

    // ── Move bookmark between folders ─────────────────────────────────────

    /// Move a bookmarked project into a folder.
    ///
    /// The project must already be in the user's flat bookmark list.
    /// `folder_id` must be owned by `user`.  Passing `folder_id = 0` is
    /// **not** a valid folder ID (IDs start at 1); callers wanting to remove a
    /// project from its folder without assigning a new one should call
    /// `remove_bookmark_from_folder`.
    pub fn move_bookmark_to_folder(
        env: &Env,
        user: Address,
        project_id: u64,
        folder_id: u64,
    ) -> Result<(), ContractError> {
        user.require_auth();

        if !Self::is_bookmarked(env, project_id, &user) {
            return Err(ContractError::NotBookmarked);
        }

        Self::require_folder(env, &user, folder_id)?;

        // Remove from old folder if in one.
        if let Some(old_folder_id) = Self::get_bookmark_folder(env, project_id, &user) {
            if old_folder_id == folder_id {
                // Already in the target folder — no-op with success.
                return Ok(());
            }
            let _ = Self::remove_from_folder_list(env, &user, old_folder_id, project_id);
        }

        // Add to new folder's bookmark list.
        let mut folder_bm: Vec<u64> = env
            .storage()
            .persistent()
            .get(&BookmarkKey::FolderBookmarks(user.clone(), folder_id))
            .unwrap_or_else(|| Vec::new(env));

        if folder_bm.len() >= MAX_BOOKMARKS_PER_FOLDER {
            return Err(ContractError::CollectionFull);
        }

        folder_bm.push_back(project_id);
        env.storage().persistent().set(
            &BookmarkKey::FolderBookmarks(user.clone(), folder_id),
            &folder_bm,
        );

        // Update the per-user folder index.
        env.storage().persistent().set(
            &BookmarkKey::BookmarkFolderIndex(user.clone(), project_id),
            &folder_id,
        );

        StorageManager::extend_folder_ttl(env, &user, folder_id);
        StorageManager::extend_bookmark_folder_index_ttl(env, &user, project_id);

        publish_bookmark_moved_to_folder_event(env, project_id, user, folder_id);
        Ok(())
    }

    /// Remove a bookmark from its current folder (leaving it in the flat list,
    /// just not assigned to any folder).
    pub fn remove_bookmark_from_folder(
        env: &Env,
        user: Address,
        project_id: u64,
    ) -> Result<(), ContractError> {
        user.require_auth();

        let folder_id = Self::get_bookmark_folder(env, project_id, &user)
            .ok_or(ContractError::FolderNotFound)?;

        Self::remove_from_folder_list(env, &user, folder_id, project_id)?;

        env.storage()
            .persistent()
            .remove(&BookmarkKey::BookmarkFolderIndex(user.clone(), project_id));

        StorageManager::extend_folder_ttl(env, &user, folder_id);

        publish_bookmark_removed_from_folder_event(env, project_id, user, folder_id);
        Ok(())
    }

    /// Get the bookmarks in a specific folder, paginated.
    pub fn get_folder_bookmarks(
        env: &Env,
        user: Address,
        folder_id: u64,
        start_index: u32,
        limit: u32,
    ) -> Vec<u64> {
        let effective_limit = if limit == 0 || limit > MAX_PAGE_LIMIT {
            MAX_PAGE_LIMIT
        } else {
            limit
        };

        let bookmarks: Vec<u64> = env
            .storage()
            .persistent()
            .get(&BookmarkKey::FolderBookmarks(user, folder_id))
            .unwrap_or_else(|| Vec::new(env));

        let len = bookmarks.len();
        if start_index >= len {
            return Vec::new(env);
        }
        let end = core::cmp::min(start_index.saturating_add(effective_limit), len);
        let mut page = Vec::new(env);
        for i in start_index..end {
            if let Some(pid) = bookmarks.get(i) {
                page.push_back(pid);
            }
        }
        page
    }

    /// Get the current folder (if any) that contains a given bookmarked project.
    pub fn get_bookmark_folder(env: &Env, project_id: u64, user: &Address) -> Option<u64> {
        env.storage()
            .persistent()
            .get(&BookmarkKey::BookmarkFolderIndex(user.clone(), project_id))
    }

    // ── Smart Folders ─────────────────────────────────────────────────────

    /// Create a smart folder.  Returns the new smart-folder ID.
    pub fn create_smart_folder(
        env: &Env,
        user: Address,
        name: soroban_sdk::String,
        filter: SmartFolderFilter,
    ) -> Result<u64, ContractError> {
        user.require_auth();
        Self::validate_folder_name(&name)?;

        let ids = Self::get_user_smart_folder_ids(env, &user);
        if ids.len() >= MAX_SMART_FOLDERS_PER_USER {
            return Err(ContractError::MaxSmartFoldersExceeded);
        }

        let id = Self::next_smart_folder_id(env, &user);
        let ts = env.ledger().timestamp();
        let sf = SmartFolder {
            id,
            owner: user.clone(),
            name: name.clone(),
            filter,
            created_at: ts,
            updated_at: ts,
        };

        env.storage()
            .persistent()
            .set(&BookmarkKey::SmartFolder(user.clone(), id), &sf);

        let mut ids_updated = ids;
        ids_updated.push_back(id);
        env.storage()
            .persistent()
            .set(&BookmarkKey::UserSmartFolderIds(user.clone()), &ids_updated);

        StorageManager::extend_user_smart_folder_ids_ttl(env, &user);
        StorageManager::extend_smart_folder_ttl(env, &user, id);

        publish_smart_folder_created_event(env, id, user, name);
        Ok(id)
    }

    /// Delete a smart folder.
    pub fn delete_smart_folder(
        env: &Env,
        user: Address,
        smart_folder_id: u64,
    ) -> Result<(), ContractError> {
        user.require_auth();
        Self::require_smart_folder(env, &user, smart_folder_id)?;

        env.storage()
            .persistent()
            .remove(&BookmarkKey::SmartFolder(user.clone(), smart_folder_id));

        let ids = Self::get_user_smart_folder_ids(env, &user);
        let updated = Utils::remove_item_from_vec(env, &ids, &smart_folder_id);
        env.storage()
            .persistent()
            .set(&BookmarkKey::UserSmartFolderIds(user.clone()), &updated);

        StorageManager::extend_user_smart_folder_ids_ttl(env, &user);

        publish_smart_folder_deleted_event(env, smart_folder_id, user);
        Ok(())
    }

    /// Get a smart folder record.
    pub fn get_smart_folder(env: &Env, user: Address, smart_folder_id: u64) -> Option<SmartFolder> {
        env.storage()
            .persistent()
            .get(&BookmarkKey::SmartFolder(user, smart_folder_id))
    }

    /// List all smart folders owned by `user`.
    pub fn list_smart_folders(env: &Env, user: Address) -> Vec<SmartFolder> {
        let ids = Self::get_user_smart_folder_ids(env, &user);
        let mut result = Vec::new(env);
        for i in 0..ids.len() {
            if let Some(sfid) = ids.get(i) {
                if let Some(sf) = env
                    .storage()
                    .persistent()
                    .get::<_, SmartFolder>(&BookmarkKey::SmartFolder(user.clone(), sfid))
                {
                    result.push_back(sf);
                }
            }
        }
        result
    }

    /// Resolve the bookmark list of a smart folder by applying its filter to
    /// the user's full bookmark list.
    ///
    /// Returns a paginated subset of matching project IDs.
    pub fn get_smart_folder_bookmarks(
        env: &Env,
        user: Address,
        smart_folder_id: u64,
        start_index: u32,
        limit: u32,
    ) -> Result<Vec<u64>, ContractError> {
        let sf = Self::require_smart_folder(env, &user, smart_folder_id)?;

        let all_bookmarks: Vec<u64> = env
            .storage()
            .persistent()
            .get(&ExtensionKey::UserBookmarks(user.clone()))
            .unwrap_or_else(|| Vec::new(env));

        let effective_limit = if limit == 0 || limit > MAX_PAGE_LIMIT {
            MAX_PAGE_LIMIT
        } else {
            limit
        };

        // Collect matching project IDs.
        let mut matched: Vec<u64> = Vec::new(env);
        for i in 0..all_bookmarks.len() {
            if let Some(project_id) = all_bookmarks.get(i) {
                if Self::project_matches_filter(env, project_id, &sf.filter) {
                    matched.push_back(project_id);
                }
            }
        }

        // Paginate the matched list.
        let len = matched.len();
        if start_index >= len {
            return Ok(Vec::new(env));
        }
        let end = core::cmp::min(start_index.saturating_add(effective_limit), len);
        let mut page = Vec::new(env);
        for i in start_index..end {
            if let Some(pid) = matched.get(i) {
                page.push_back(pid);
            }
        }
        Ok(page)
    }

    // ── Internal helpers ──────────────────────────────────────────────────

    fn validate_folder_name(name: &soroban_sdk::String) -> Result<(), ContractError> {
        let len = name.len();
        if len == 0 {
            return Err(ContractError::InvalidProjectData);
        }
        if len as usize > MAX_FOLDER_NAME_LEN {
            return Err(ContractError::InvalidProjectName);
        }
        Ok(())
    }

    fn require_folder(
        env: &Env,
        user: &Address,
        folder_id: u64,
    ) -> Result<BookmarkFolder, ContractError> {
        env.storage()
            .persistent()
            .get(&BookmarkKey::BookmarkFolder(user.clone(), folder_id))
            .ok_or(ContractError::FolderNotFound)
    }

    fn require_smart_folder(
        env: &Env,
        user: &Address,
        smart_folder_id: u64,
    ) -> Result<SmartFolder, ContractError> {
        env.storage()
            .persistent()
            .get(&BookmarkKey::SmartFolder(user.clone(), smart_folder_id))
            .ok_or(ContractError::SmartFolderNotFound)
    }

    fn get_user_folder_ids(env: &Env, user: &Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&BookmarkKey::UserFolderIds(user.clone()))
            .unwrap_or_else(|| Vec::new(env))
    }

    fn get_user_smart_folder_ids(env: &Env, user: &Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&BookmarkKey::UserSmartFolderIds(user.clone()))
            .unwrap_or_else(|| Vec::new(env))
    }

    fn next_folder_id(env: &Env, user: &Address) -> u64 {
        let id: u64 = env
            .storage()
            .persistent()
            .get(&BookmarkKey::NextFolderIdForUser(user.clone()))
            .unwrap_or(1u64);
        env.storage()
            .persistent()
            .set(&BookmarkKey::NextFolderIdForUser(user.clone()), &(id + 1));
        id
    }

    fn next_smart_folder_id(env: &Env, user: &Address) -> u64 {
        let id: u64 = env
            .storage()
            .persistent()
            .get(&BookmarkKey::NextSmartFolderIdForUser(user.clone()))
            .unwrap_or(1u64);
        env.storage()
            .persistent()
            .set(
                &BookmarkKey::NextSmartFolderIdForUser(user.clone()),
                &(id + 1),
            );
        id
    }

    /// Compute the depth that would result from a new folder with the given
    /// `parent_id`.  `current_depth` starts at 1 on the first call.
    ///
    /// Returns `current_depth` when `parent_id` is `None` (root).
    /// Returns `FolderDepthExceeded` if the chain would exceed `MAX_FOLDER_DEPTH`.
    fn compute_folder_depth(
        env: &Env,
        user: &Address,
        parent_id: Option<u64>,
        current_depth: u32,
    ) -> Result<u32, ContractError> {
        if current_depth > MAX_FOLDER_DEPTH {
            return Err(ContractError::FolderDepthExceeded);
        }
        match parent_id {
            None => Ok(current_depth),
            Some(pid) => {
                let parent = Self::require_folder(env, user, pid)?;
                Self::compute_folder_depth(env, user, parent.parent_id, current_depth + 1)
            }
        }
    }

    /// Remove `project_id` from the `FolderBookmarks` list for `folder_id`.
    fn remove_from_folder_list(
        env: &Env,
        user: &Address,
        folder_id: u64,
        project_id: u64,
    ) -> Result<(), ContractError> {
        let list: Vec<u64> = env
            .storage()
            .persistent()
            .get(&BookmarkKey::FolderBookmarks(user.clone(), folder_id))
            .unwrap_or_else(|| Vec::new(env));
        let updated = Utils::remove_item_from_vec(env, &list, &project_id);
        env.storage()
            .persistent()
            .set(
                &BookmarkKey::FolderBookmarks(user.clone(), folder_id),
                &updated,
            );
        Ok(())
    }

    /// Return `true` if the project satisfies every `Some` field in `filter`.
    /// Fields that are `None` are treated as "match anything".
    fn project_matches_filter(env: &Env, project_id: u64, filter: &SmartFolderFilter) -> bool {
        // If no criteria at all, match every bookmarked project.
        let has_criteria = filter.category.is_some()
            || filter.tag.is_some()
            || matches!(filter.verification_status, VerificationStatusFilter::Is(_));

        if !has_criteria {
            return true;
        }

        let project = match ProjectRegistry::get_project(env, project_id) {
            Some(p) => p,
            None => return false,
        };

        // Category filter.
        if let Some(ref cat) = filter.category {
            if project.category != *cat {
                return false;
            }
        }

        // Verification status filter.
        if let VerificationStatusFilter::Is(ref vs) = filter.verification_status {
            if project.verification_status != *vs {
                return false;
            }
        }

        // Tag filter — project must carry the tag.
        if let Some(ref tag) = filter.tag {
            if let Some(ref tags) = project.tags {
                let mut found = false;
                for i in 0..tags.len() {
                    if let Some(t) = tags.get(i) {
                        if t == *tag {
                            found = true;
                            break;
                        }
                    }
                }
                if !found {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }
}
