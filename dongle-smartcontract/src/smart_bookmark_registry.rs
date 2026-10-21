//! Smart bookmark collection registry (#914).
//!
//! Enables users to create smart collections that auto-populate based on
//! criteria: tag match, category, date range, and optional nesting within
//! existing user bookmark folders.

use crate::bookmark_registry::BookmarkRegistry;
use crate::constants::{MAX_FOLDER_NAME_LEN, MAX_SMART_BOOKMARK_COLLECTIONS_PER_USER};
use crate::errors::ContractError;
use crate::events::{
    publish_smart_bookmark_collection_created_event,
    publish_smart_bookmark_collection_refreshed_event,
};
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::{ExtensionKey, SmartBookmarkKey};
use crate::types::{Project, SmartBookmarkCollection, SmartBookmarkRules};
use crate::utils::Utils;
use soroban_sdk::{Address, Env, String, Vec};

pub struct SmartBookmarkRegistry;

impl SmartBookmarkRegistry {
    /// Create a smart bookmark collection. Auto-populates projects matching
    /// the rules from the user's bookmarked projects.
    pub fn create_smart_bookmark_collection(
        env: &Env,
        user: Address,
        name: String,
        rules: SmartBookmarkRules,
    ) -> Result<u64, ContractError> {
        user.require_auth();
        Self::validate_name(&name)?;

        // If nested under a parent folder, verify parent folder exists for this user.
        if let Some(parent_id) = rules.parent_id {
            if BookmarkRegistry::get_folder(env, user.clone(), parent_id).is_none() {
                return Err(ContractError::FolderNotFound);
            }
        }

        let user_col_ids = Self::get_user_collection_ids(env, &user);
        if user_col_ids.len() >= MAX_SMART_BOOKMARK_COLLECTIONS_PER_USER {
            return Err(ContractError::MaxSmartBookmarkCollectionsExceeded);
        }

        let id = Self::next_collection_id(env);
        let ts = env.ledger().timestamp();

        // Populate projects matching rules from user bookmarks
        let project_ids = Self::evaluate_rules_for_user(env, &user, &rules);

        let collection = SmartBookmarkCollection {
            id,
            owner: user.clone(),
            name: name.clone(),
            rules: rules.clone(),
            parent_id: rules.parent_id,
            project_ids,
            created_at: ts,
            updated_at: ts,
        };

        // Persist collection
        env.storage()
            .persistent()
            .set(&SmartBookmarkKey::Collection(id), &collection);

        // Add to user's collection list
        let mut updated_ids = user_col_ids;
        updated_ids.push_back(id);
        env.storage()
            .persistent()
            .set(&SmartBookmarkKey::UserCollections(user.clone()), &updated_ids);

        publish_smart_bookmark_collection_created_event(env, id, user, name);
        Ok(id)
    }

    /// Refresh a smart bookmark collection's member projects based on its rules
    /// and the owner's current bookmarks.
    pub fn refresh_smart_bookmark_collection(
        env: &Env,
        collection_id: u64,
    ) -> Result<Vec<u64>, ContractError> {
        let mut collection: SmartBookmarkCollection = env
            .storage()
            .persistent()
            .get(&SmartBookmarkKey::Collection(collection_id))
            .ok_or(ContractError::SmartBookmarkCollectionNotFound)?;

        collection.owner.require_auth();

        let updated_project_ids =
            Self::evaluate_rules_for_user(env, &collection.owner, &collection.rules);
        let count = updated_project_ids.len();

        collection.project_ids = updated_project_ids.clone();
        collection.updated_at = env.ledger().timestamp();

        env.storage()
            .persistent()
            .set(&SmartBookmarkKey::Collection(collection_id), &collection);

        publish_smart_bookmark_collection_refreshed_event(
            env,
            collection_id,
            collection.owner,
            count,
        );

        Ok(updated_project_ids)
    }

    /// Get a smart bookmark collection by ID.
    pub fn get_smart_bookmark_collection(
        env: &Env,
        collection_id: u64,
    ) -> Option<SmartBookmarkCollection> {
        env.storage()
            .persistent()
            .get(&SmartBookmarkKey::Collection(collection_id))
    }

    /// List all smart bookmark collections owned by a user.
    pub fn list_smart_bookmark_collections(
        env: &Env,
        user: Address,
    ) -> Vec<SmartBookmarkCollection> {
        let ids = Self::get_user_collection_ids(env, &user);
        let mut result = Vec::new(env);
        for i in 0..ids.len() {
            if let Some(id) = ids.get(i) {
                if let Some(col) = env
                    .storage()
                    .persistent()
                    .get::<_, SmartBookmarkCollection>(&SmartBookmarkKey::Collection(id))
                {
                    result.push_back(col);
                }
            }
        }
        result
    }

    /// Delete a smart bookmark collection.
    pub fn delete_smart_bookmark_collection(
        env: &Env,
        user: Address,
        collection_id: u64,
    ) -> Result<(), ContractError> {
        user.require_auth();

        let collection: SmartBookmarkCollection = env
            .storage()
            .persistent()
            .get(&SmartBookmarkKey::Collection(collection_id))
            .ok_or(ContractError::SmartBookmarkCollectionNotFound)?;

        if collection.owner != user {
            return Err(ContractError::Unauthorized);
        }

        env.storage()
            .persistent()
            .remove(&SmartBookmarkKey::Collection(collection_id));

        let ids = Self::get_user_collection_ids(env, &user);
        let updated = Utils::remove_item_from_vec(env, &ids, &collection_id);
        env.storage()
            .persistent()
            .set(&SmartBookmarkKey::UserCollections(user), &updated);

        Ok(())
    }

    /// Check if a project matches the given smart bookmark rules.
    pub fn project_matches_rules(
        _env: &Env,
        project: &Project,
        rules: &SmartBookmarkRules,
    ) -> bool {
        // Tag match rule: project must contain the specified tag
        if let Some(ref required_tag) = rules.tag {
            let mut tag_matched = false;
            if let Some(ref project_tags) = project.tags {
                for i in 0..project_tags.len() {
                    if let Some(t) = project_tags.get(i) {
                        if t == *required_tag {
                            tag_matched = true;
                            break;
                        }
                    }
                }
            }
            if !tag_matched {
                return false;
            }
        }

        // Category match rule
        if let Some(ref required_category) = rules.category {
            if project.category != *required_category {
                return false;
            }
        }

        // Date range rules (created_at timestamp)
        if let Some(min_date) = rules.min_date {
            if project.created_at < min_date {
                return false;
            }
        }

        if let Some(max_date) = rules.max_date {
            if project.created_at > max_date {
                return false;
            }
        }

        true
    }

    /// Helper to evaluate rules against all bookmarked projects of a user.
    fn evaluate_rules_for_user(
        env: &Env,
        user: &Address,
        rules: &SmartBookmarkRules,
    ) -> Vec<u64> {
        let bookmarks: Vec<u64> = env
            .storage()
            .persistent()
            .get(&ExtensionKey::UserBookmarks(user.clone()))
            .unwrap_or_else(|| Vec::new(env));

        let mut matched = Vec::new(env);
        for i in 0..bookmarks.len() {
            if let Some(project_id) = bookmarks.get(i) {
                if let Some(project) = ProjectRegistry::get_project(env, project_id) {
                    if !project.archived && Self::project_matches_rules(env, &project, rules) {
                        matched.push_back(project_id);
                    }
                }
            }
        }
        matched
    }

    fn get_user_collection_ids(env: &Env, user: &Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&SmartBookmarkKey::UserCollections(user.clone()))
            .unwrap_or_else(|| Vec::new(env))
    }

    fn next_collection_id(env: &Env) -> u64 {
        let id: u64 = env
            .storage()
            .persistent()
            .get(&SmartBookmarkKey::NextCollectionId)
            .unwrap_or(1);
        env.storage()
            .persistent()
            .set(&SmartBookmarkKey::NextCollectionId, &(id.saturating_add(1)));
        id
    }

    fn validate_name(name: &String) -> Result<(), ContractError> {
        let len = name.len();
        if len == 0 {
            return Err(ContractError::InvalidProjectData);
        }
        if len as usize > MAX_FOLDER_NAME_LEN {
            return Err(ContractError::InvalidProjectName);
        }
        Ok(())
    }
}
