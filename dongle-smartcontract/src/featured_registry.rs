//! Featured projects registry – admin-only curation of highlighted projects.
//!
//! ## Ordering
//! Featured projects are stored in insertion order (FIFO). The list returned
//! by [`FeaturedRegistry::list_featured_projects`] preserves that order so
//! consumers receive projects in the sequence they were featured.
//!
//! ## Limit & Eviction Policy (issue #661)
//! At most [`MAX_FEATURED_PROJECTS`] projects may be featured at any given
//! time.  When an admin calls `set_featured(…, true)` and the limit is already
//! reached, the **oldest** featured project (the one at the front of the list)
//! is automatically evicted — this is a FIFO (first-in, first-out) eviction
//! policy.  The eviction is transparent: the admin's new project is always
//! added, and the oldest slot is freed to make room.
//!
//! If finer-grained control is needed in the future (priority ordering,
//! pinned slots, etc.) the ordering can be extended without a breaking change,
//! since `list_featured_projects` already supports pagination.

use crate::admin_action_log::AdminActionLog;
use crate::auth::require_admin_auth;
use crate::constants::MAX_FEATURED_PROJECTS;
use crate::errors::ContractError;
use crate::events::publish_featured_project_event;
use crate::pagination::paginate;
use crate::storage_keys::StorageKey;
use crate::types::{AdminActionType, Project};
use soroban_sdk::{Address, Env, Vec};

pub struct FeaturedRegistry;

impl FeaturedRegistry {
    /// Mark or unmark a project as featured. Admin-only.
    ///
    /// When `featured` is `true`:
    /// - If the project is already featured, this is a no-op.
    /// - If the list is at capacity ([`MAX_FEATURED_PROJECTS`]) the oldest
    ///   featured project is silently evicted (FIFO) before the new one is
    ///   inserted.
    ///
    /// When `featured` is `false` the project is simply removed from the list.
    pub fn set_featured(
        env: &Env,
        admin: Address,
        project_id: u64,
        featured: bool,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;

        // Ensure the project exists.
        if !env
            .storage()
            .persistent()
            .has(&StorageKey::Project(project_id))
        {
            return Err(ContractError::ProjectNotFound);
        }

        let mut ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::FeaturedProjects)
            .unwrap_or(Vec::new(env));

        let already_featured = ids.iter().any(|id| id == project_id);

        if featured && !already_featured {
            // Enforce the maximum-featured-projects limit via FIFO eviction.
            // If we are at capacity, remove the oldest entry (index 0) first.
            if ids.len() >= MAX_FEATURED_PROJECTS {
                // Evict the oldest (front) entry by rebuilding the list
                // starting from index 1, then appending the new project.
                let mut updated = Vec::new(env);
                let len = ids.len();
                for i in 1..len {
                    if let Some(id) = ids.get(i) {
                        updated.push_back(id);
                    }
                }
                ids = updated;
            }

            ids.push_back(project_id);
            env.storage()
                .persistent()
                .set(&StorageKey::FeaturedProjects, &ids);
        } else if !featured && already_featured {
            let mut updated = Vec::new(env);
            for id in ids.iter() {
                if id != project_id {
                    updated.push_back(id);
                }
            }
            env.storage()
                .persistent()
                .set(&StorageKey::FeaturedProjects, &updated);
        }

        publish_featured_project_event(env, project_id, featured, admin.clone());

        let action_type = if featured {
            AdminActionType::ProjectFeatured
        } else {
            AdminActionType::ProjectUnfeatured
        };
        AdminActionLog::record_action(env, admin, action_type, Some(project_id), None, None);

        Ok(())
    }

    /// List featured projects with pagination.
    ///
    /// Projects are returned in insertion order (oldest featured first).
    /// Use `start_index` and `limit` for pagination.
    pub fn list_featured_projects(env: &Env, start_index: u32, limit: u32) -> Vec<Project> {
        let ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::FeaturedProjects)
            .unwrap_or(Vec::new(env));
        let page_ids = paginate(env, &ids, start_index, limit);
        let mut result = Vec::new(env);
        for project_id in page_ids.iter() {
            if let Some(project) = env
                .storage()
                .persistent()
                .get(&StorageKey::Project(project_id))
            {
                result.push_back(project);
            }
        }
        result
    }

    /// Return how many projects are currently featured.
    pub fn get_featured_count(env: &Env) -> u32 {
        let ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::FeaturedProjects)
            .unwrap_or(Vec::new(env));
        ids.len()
    }
}
