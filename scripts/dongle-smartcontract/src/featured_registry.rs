//! Featured projects registry – admin-only curation of highlighted projects.

use crate::admin_action_log::AdminActionLog;
use crate::auth::require_admin_auth;
use crate::errors::ContractError;
use crate::events::publish_featured_project_event;
use crate::storage_keys::StorageKey;
use crate::types::{AdminActionType, Project};
use soroban_sdk::{Address, Env, Vec};

pub struct FeaturedRegistry;

impl FeaturedRegistry {
    /// Mark or unmark a project as featured. Admin-only.
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
    pub fn list_featured_projects(env: &Env, start: u32, limit: u32) -> Vec<Project> {
        let ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::FeaturedProjects)
            .unwrap_or(Vec::new(env));

        let limit = limit.min(100);
        let mut result = Vec::new(env);
        let mut count = 0u32;

        for (i, project_id) in ids.iter().enumerate() {
            if (i as u32) < start {
                continue;
            }
            if count >= limit {
                break;
            }
            if let Some(project) = env
                .storage()
                .persistent()
                .get(&StorageKey::Project(project_id))
            {
                result.push_back(project);
                count += 1;
            }
        }

        result
    }
}
