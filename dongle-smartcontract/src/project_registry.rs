use crate::errors::ContractError;
use crate::storage_keys::StorageKey;
use crate::types::{Project, ProjectRegistrationParams, ProjectUpdateParams, VerificationStatus};
use soroban_sdk::{Address, Env, String, Vec};

pub struct ProjectRegistry;

impl ProjectRegistry {
    #[allow(clippy::too_many_arguments)]
    pub fn register_project(
        env: &Env,
        params: ProjectRegistrationParams,
    ) -> Result<u64, ContractError> {
        params.owner.require_auth();

        // Validation
        if name.is_empty() {
            panic!("InvalidProjectName");
        }
        if description.is_empty() {
            panic!("InvalidProjectDescription");
        }
        if category.is_empty() {
        if params.name.is_empty() {
            panic!("InvalidProjectName");
        }
        if params.description.is_empty() {
            panic!("InvalidProjectDescription");
        }
        if params.category.is_empty() {
            panic!("InvalidProjectCategory");
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
        env.storage()
            .persistent()
            .set(&StorageKey::OwnerProjects(params.owner), &owner_projects);

        Ok(count)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_project(
        env: &Env,
        project_id: u64,
        caller: Address,
        name: Option<String>,
        description: Option<String>,
        category: Option<String>,
        website: Option<Option<String>>,
        logo_cid: Option<Option<String>>,
        metadata_cid: Option<Option<String>>,
    ) -> Option<Project> {
        let mut project = Self::get_project(env, project_id)?;

        caller.require_auth();
        if project.owner != caller {
    pub fn update_project(env: &Env, params: ProjectUpdateParams) -> Option<Project> {
        let mut project = Self::get_project(env, params.project_id)?;

        params.caller.require_auth();
        if project.owner != params.caller {
            return None;
        }

        if let Some(value) = params.name {
            project.name = value;
        }
        if let Some(value) = params.description {
            project.description = value;
        }
        if let Some(value) = params.category {
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

        Some(project)
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
        for project_id in ids.iter() {
            if let Some(project) = Self::get_project(env, project_id) {
                projects.push_back(project);
            }
        }

        projects
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

        let end_id = core::cmp::min(start_id.saturating_add(limit as u64), count + 1);

        for id in start_id..end_id {
            if let Some(project) = Self::get_project(env, id) {
                projects.push_back(project);
            }
        }

        projects
    }

    #[allow(dead_code)]
    pub fn project_exists(env: &Env, project_id: u64) -> bool {
        env.storage()
            .persistent()
            .has(&StorageKey::Project(project_id))
    }

    #[allow(dead_code)]
    pub fn validate_project_data(
        name: &String,
        description: &String,
        category: &String,
    ) -> Result<(), ContractError> {
        if name.is_empty() {
            return Err(ContractError::InvalidProjectData);
        }
        if description.is_empty() {
            return Err(ContractError::ProjectDescriptionTooLong); // Just picking one for now to match ContractError
            return Err(ContractError::ProjectDescriptionTooLong);
        }
        if category.is_empty() {
            return Err(ContractError::InvalidProjectCategory);
        }
        Ok(())
    }
}
