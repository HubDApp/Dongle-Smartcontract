use crate::auth::{require_owner_auth, require_self_auth};
use crate::errors::ContractError;
use crate::events::{publish_project_registered_event, publish_project_updated_event};
use crate::storage_keys::StorageKey;
use crate::types::{Project, ProjectRegistrationParams, ProjectUpdateParams, VerificationStatus};
use soroban_sdk::{Address, Env, Vec};

/// Maximum number of items returned per paginated list call.
pub const MAX_PAGE_LIMIT: u32 = 100;

pub struct ProjectRegistry;

impl ProjectRegistry {
    #[allow(clippy::too_many_arguments)]
    pub fn register_project(
        env: &Env,
        params: ProjectRegistrationParams,
    ) -> Result<u64, ContractError> {
        require_self_auth(&params.owner);

        // Validate inputs - return typed errors instead of panicking
        if params.name.is_empty() {
            return Err(ContractError::InvalidProjectName);
        }
        if params.description.is_empty() {
            return Err(ContractError::InvalidProjectDescription);
        }
        if params.category.is_empty() {
            return Err(ContractError::InvalidProjectCategory);
        }

        // Check if project name already exists
        if env
            .storage()
            .persistent()
            .has(&StorageKey::ProjectByName(params.name.clone()))
        {
            return Err(ContractError::ProjectAlreadyExists);
        }

        let mut count: u64 = env
            .storage()
            .persistent()
            .get(&StorageKey::ProjectCount)
            .unwrap_or(0);
        count = count.saturating_add(1);

        let now = env.ledger().timestamp();
        let project = Project {
            id: count,
            owner: params.owner.clone(),
            name: params.name.clone(),
            description: params.description,
            category: params.category,
            website: params.website,
            logo_cid: params.logo_cid,
            metadata_cid: params.metadata_cid,
            verification_status: VerificationStatus::Unverified,
            created_at: now,
            updated_at: now,
        };

        env.storage()
            .persistent()
            .set(&StorageKey::Project(count), &project);
        env.storage()
            .persistent()
            .set(&StorageKey::ProjectCount, &count);
        env.storage()
            .persistent()
            .set(&StorageKey::ProjectByName(params.name), &count);

        let mut owner_projects: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::OwnerProjects(params.owner.clone()))
            .unwrap_or_else(|| Vec::new(env));
        owner_projects.push_back(count);
        env.storage().persistent().set(
            &StorageKey::OwnerProjects(params.owner.clone()),
            &owner_projects,
        );

        publish_project_registered_event(
            env,
            count,
            params.owner,
            project.name.clone(),
            project.category.clone(),
        );

        Ok(count)
    }

    pub fn update_project(
        env: &Env,
        params: ProjectUpdateParams,
    ) -> Result<Project, ContractError> {
        let mut project = Self::get_project(env, params.project_id)
            .ok_or(ContractError::ProjectNotFound)?;

        params.caller.require_auth();
        if project.owner != params.caller {
            return Err(ContractError::Unauthorized);
        }

        // Validate and update fields
        if let Some(value) = params.name {
            if value.is_empty() {
                return Err(ContractError::InvalidProjectName);
            }
            project.name = value;
        }
        if let Some(value) = params.description {
            if value.is_empty() {
                return Err(ContractError::InvalidProjectDescription);
            }
            project.description = value;
        }
        if let Some(value) = params.category {
            if value.is_empty() {
                return Err(ContractError::InvalidProjectCategory);
            }
            project.category = value;
        }
        if let Some(value) = params.website {
            project.website = value;
        }
        if let Some(value) = params.logo_cid {
            project.logo_cid = value;
        }
        if let Some(value) = params.metadata_cid {
            project.metadata_cid = value;
        }

        project.updated_at = env.ledger().timestamp();
        env.storage()
            .persistent()
            .set(&StorageKey::Project(params.project_id), &project);

        Ok(project)
    }

    pub fn get_project(env: &Env, project_id: u64) -> Option<Project> {
        env.storage()
            .persistent()
            .get(&StorageKey::Project(project_id))
    }

    pub fn get_projects_by_owner(env: &Env, owner: Address) -> Vec<Project> {
        let ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::OwnerProjects(owner))
            .unwrap_or_else(|| Vec::new(env));

        let mut projects = Vec::new(env);
        let len = ids.len();
        for i in 0..len {
            if let Some(project_id) = ids.get(i) {
                if let Some(project) = Self::get_project(env, project_id) {
                    projects.push_back(project);
                }
            }
        }

        projects
    }

    fn owner_project_count(env: &Env, owner: &Address) -> u32 {
        env.storage()
            .persistent()
            .get(&StorageKey::OwnerProjects(owner.clone()))
            .unwrap_or_else(|| Vec::<u64>::new(env))
            .len()
    }

    pub fn get_owner_project_count(env: &Env, owner: &Address) -> u32 {
        Self::owner_project_count(env, owner)
    }

    pub fn list_projects(env: &Env, start_id: u64, limit: u32) -> Vec<Project> {
        // Enforce pagination limits: limit must be 1..=MAX_PAGE_LIMIT
        let effective_limit = if limit == 0 || limit > MAX_PAGE_LIMIT {
            MAX_PAGE_LIMIT
        } else {
            limit
        };

        let count: u64 = env
            .storage()
            .persistent()
            .get(&StorageKey::ProjectCount)
            .unwrap_or(0);

        let mut projects = Vec::new(env);
        if count == 0 {
            return projects;
        }

        // start_id is 1-based (projects are stored with IDs starting at 1).
        // Clamp to valid range.
        let first = if start_id == 0 { 1u64 } else { start_id };
        if first > count {
            return projects;
        }

        let end = core::cmp::min(
            first.saturating_add(effective_limit as u64),
            count.saturating_add(1),
        );

        for id in first..end {
            if let Some(project) = Self::get_project(env, id) {
                projects.push_back(project);
            }
        }
        projects
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use crate::errors::ContractError;
    use soroban_sdk::{Env, String};

    // Validation function only used in tests
    fn validate_project_data(
        name: &String,
        _description: &String,
        _category: &String,
    ) -> Result<(), ContractError> {
        extern crate alloc;
        use alloc::string::ToString;

        let name_str = name.to_string();

        // 1. Validate Non-empty and not only whitespace
        if name_str.trim().is_empty() {
            return Err(ContractError::InvalidProjectData);
        }

        // 2. Validate max length using the CONSTANT
        let max_len = crate::constants::MAX_NAME_LEN;
        if name_str.len() > max_len {
            return Err(ContractError::ProjectNameTooLong);
        }

        // 3. Validate alphanumeric, underscore, hyphen
        for c in name_str.chars() {
            if !c.is_ascii_alphanumeric() && c != '_' && c != '-' {
                return Err(ContractError::InvalidProjectNameFormat);
            }
        }

        Ok(())
    }

    #[test]
    fn test_valid_project_name() {
        let env = Env::default();
        let name = String::from_str(&env, "Valid-Project_Name123");

        let result = validate_project_data(
            &name,
            &String::from_str(&env, "Desc"),
            &String::from_str(&env, "Cat"),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_empty_or_whitespace_name() {
        let env = Env::default();
        let name = String::from_str(&env, "   ");

        let result = validate_project_data(
            &name,
            &String::from_str(&env, "Desc"),
            &String::from_str(&env, "Cat"),
        );
        assert_eq!(result, Err(ContractError::InvalidProjectData));
    }

    #[test]
    fn test_invalid_characters_in_name() {
        let env = Env::default();
        let name = String::from_str(&env, "My Project *");

        let result = validate_project_data(
            &name,
            &String::from_str(&env, "Desc"),
            &String::from_str(&env, "Cat"),
        );
        assert_eq!(result, Err(ContractError::InvalidProjectNameFormat));
    }

    #[test]
    fn test_name_too_long() {
        let env = Env::default();
        // 51 characters
        let name = String::from_str(&env, "ThisProjectNameIsWayTooLongAndExceedsTheFiftyCharL1");

        let result = validate_project_data(
            &name,
            &String::from_str(&env, "Desc"),
            &String::from_str(&env, "Cat"),
        );
        assert_eq!(result, Err(ContractError::ProjectNameTooLong));
    }
}
