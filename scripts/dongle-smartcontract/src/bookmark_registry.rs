use crate::events::{publish_project_bookmarked_event, publish_project_unbookmarked_event};
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::ExtensionKey;
use crate::storage_manager::StorageManager;
use soroban_sdk::{contracterror, Address, Env, Vec};

pub const MAX_PAGE_LIMIT: u32 = 100;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum BookmarkError {
    AlreadyBookmarked = 1,
    NotBookmarked = 2,
}

pub struct BookmarkRegistry;

impl BookmarkRegistry {
    pub fn bookmark_project(
        env: &Env,
        project_id: u64,
        user: Address,
    ) -> Result<(), BookmarkError> {
        user.require_auth();

        if ProjectRegistry::get_project(env, project_id).is_none() {
            panic!("project not found");
        }

        if Self::is_bookmarked(env, project_id, &user) {
            return Err(BookmarkError::AlreadyBookmarked);
        }

        let mut bookmarks: Vec<u64> = env
            .storage()
            .persistent()
            .get(&ExtensionKey::UserBookmarks(user.clone()))
            .unwrap_or_else(|| Vec::new(env));
        bookmarks.push_back(project_id);
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
    ) -> Result<(), BookmarkError> {
        user.require_auth();

        if !Self::is_bookmarked(env, project_id, &user) {
            return Err(BookmarkError::NotBookmarked);
        }

        let bookmarks: Vec<u64> = env
            .storage()
            .persistent()
            .get(&ExtensionKey::UserBookmarks(user.clone()))
            .unwrap_or_else(|| Vec::new(env));

        let mut new_bookmarks: Vec<u64> = Vec::new(env);
        for i in 0..bookmarks.len() {
            if let Some(pid) = bookmarks.get(i) {
                if pid != project_id {
                    new_bookmarks.push_back(pid);
                }
            }
        }
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
