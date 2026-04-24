use crate::errors::ContractError;
use crate::storage_keys::StorageKey;
use crate::types::{Project, ProjectRegistrationParams, ProjectUpdateParams, VerificationStatus};
use crate::utils::Utils;
use soroban_sdk::{Address, Env, Vec};

pub struct ProjectRegistry;

impl ProjectRegistry {
    #[allow(clippy::too_many_arguments)]
    pub fn register_project(
        env: &Env,
        params: ProjectRegistrationParams,
    ) -> Result<u64, ContractError> {
        params.owner.require_auth();

        // Validate all project parameters
        Utils::validate_project_name(&params.name)?;
        Utils::validate_project_description(&params.description)?;
        Utils::validate_project_category(&params.category)?;
        Utils::validate_website_url(&params.website)?;
        Utils::validate_logo_cid(&params.logo_cid)?;
        Utils::validate_metadata_cid(&params.metadata_cid)?;

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
        env.storage()
            .persistent()
            .set(&StorageKey::OwnerProjects(params.owner), &owner_projects);

        Ok(count)
    }

    pub fn update_project(env: &Env, params: ProjectUpdateParams) -> Result<Option<Project>, ContractError> {
        let mut project = Self::get_project(env, params.project_id).ok_or(ContractError::ProjectNotFound)?;

        params.caller.require_auth();
        if project.owner != params.caller {
            return Err(ContractError::Unauthorized);
        }

        // Validate and update name
        if let Some(ref value) = params.name {
            Utils::validate_project_name(value)?;
            project.name = value.clone();
        }

        // Validate and update description
        if let Some(ref value) = params.description {
            Utils::validate_project_description(value)?;
            project.description = value.clone();
        }

        // Validate and update category
        if let Some(ref value) = params.category {
            Utils::validate_project_category(value)?;
            project.category = value.clone();
        }

        // Validate and update website
        if let Some(value) = params.website {
            Utils::validate_website_url(&value)?;
            project.website = value;
        }

        // Validate and update logo_cid
        if let Some(value) = params.logo_cid {
            Utils::validate_logo_cid(&value)?;
            project.logo_cid = value;
        }

        // Validate and update metadata_cid
        if let Some(value) = params.metadata_cid {
            Utils::validate_metadata_cid(&value)?;
            project.metadata_cid = value;
        }

        project.updated_at = env.ledger().timestamp();
        env.storage()
            .persistent()
            .set(&StorageKey::Project(params.project_id), &project);

        Ok(Some(project))
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
        let count: u64 = env
            .storage()
            .persistent()
            .get(&StorageKey::ProjectCount)
            .unwrap_or(0);

        let mut projects = Vec::new(env);
        if start_id == 0 || start_id > count {
            return projects;
        }
        let end = core::cmp::min(
            start_id.saturating_add(limit as u64),
            count.saturating_add(1),
        );
        for id in start_id..end {
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
    use crate::project_registry::ProjectRegistry;
    use crate::utils::Utils;
    use soroban_sdk::{Address, Env, String};

    #[test]
    fn test_valid_project_name() {
        let env = Env::default();
        let name = String::from_str(&env, "Valid-Project_Name123");

        let result = Utils::validate_project_name(&name);
        assert!(result.is_ok());
    }

    #[test]
    fn test_empty_or_whitespace_name() {
        let env = Env::default();
        let name = String::from_str(&env, "   ");

        let result = Utils::validate_project_name(&name);
        assert_eq!(result, Err(ContractError::ProjectNameEmpty));
    }

    #[test]
    fn test_invalid_characters_in_name() {
        let env = Env::default();
        let name = String::from_str(&env, "My Project *");

        let result = Utils::validate_project_name(&name);
        assert_eq!(result, Err(ContractError::InvalidProjectNameFormat));
    }

    #[test]
    fn test_name_too_long() {
        let env = Env::default();
        // 51 characters
        let name = String::from_str(&env, "ThisProjectNameIsWayTooLongAndExceedsTheFiftyCharL1");

        let result = Utils::validate_project_name(&name);
        assert_eq!(result, Err(ContractError::ProjectNameTooLong));
    }
}
