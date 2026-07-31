use crate::errors::ContractError;
use crate::events::{publish_project_bookmarked_event, publish_project_unbookmarked_event};
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::ExtensionKey;
use crate::storage_manager::StorageManager;
use crate::utils::Utils;
use soroban_sdk::{Address, Env, Vec};

pub const MAX_PAGE_LIMIT: u32 = 100;

pub struct BookmarkRegistry;

impl BookmarkRegistry {
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

        if !Self::is_bookmarked(env, project_id, &user) {
            return Err(ContractError::NotBookmarked);
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

    pub fn get_user_bookmarks(env: &Env, user: Address, start: u32, limit: u32) -> Vec<u64> {
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
        if start >= len {
            return Vec::new(env);
        }

        let end = core::cmp::min(start.saturating_add(effective_limit), len);
        let mut page = Vec::new(env);
        for i in start..end {
            if let Some(pid) = bookmarks.get(i) {
                page.push_back(pid);
            }
        }
        page
    }
}
