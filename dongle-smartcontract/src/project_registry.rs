use crate::errors::ContractError;
use crate::storage_keys::StorageKey;
use crate::types::{Project, VerificationStatus};
use crate::types::{Project, DataKey, VerificationStatus};
use soroban_sdk::{Address, Env, String, Vec};

pub struct ProjectRegistry;

impl ProjectRegistry {
    pub fn register_project(
        env: &Env,
        owner: Address,
        name: String,
        description: String,
        category: String,
        website: Option<String>,
        logo_cid: Option<String>,
        metadata_cid: Option<String>,
    ) -> Result<u64, ContractError> {
    ) -> u64 {
        owner.require_auth();

        let mut count: u64 = env
            .storage()
            .persistent()
            .get(&StorageKey::NextProjectId)
            .unwrap_or(0);
        count = count.saturating_add(1);

        let now = env.ledger().timestamp();
        let project = Project {
            id: count,
            owner: owner.clone(),
            name,
            description,
            category,
            website,
            logo_cid,
            metadata_cid,
            verification_status: VerificationStatus::Unverified,
            created_at: now,
            updated_at: now,
        };

        env.storage()
            .persistent()
            .set(&StorageKey::Project(count), &project);
        env.storage()
            .persistent()
            .set(&StorageKey::NextProjectId, &count);

        let mut owner_projects: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::OwnerProjects(owner.clone()))
            .unwrap_or(Vec::new(env));
        owner_projects.push_back(count);
        env.storage()
            .persistent()
            .set(&StorageKey::OwnerProjects(owner), &owner_projects);

        Ok(count)
            .set(&DataKey::OwnerProjects(owner.clone()), &owner_projects);

        count
    }

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
    ) -> Result<Project, ContractError> {
        let mut project = Self::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        caller.require_auth();
        if project.owner != caller {
            return Err(ContractError::Unauthorized);
        }

        if let Some(value) = name {
            project.name = value;
        }
        if let Some(value) = description {
            project.description = value;
        }
        if let Some(value) = category {
            project.category = value;
        }
        if let Some(value) = website {
            project.website = value;
        }
        if let Some(value) = logo_cid {
            project.logo_cid = value;
        }
        if let Some(value) = metadata_cid {
            project.metadata_cid = value;
        }

        project.updated_at = env.ledger().timestamp();
        env.storage()
            .persistent()
            .set(&DataKey::Project(project_id), &project);

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
            .unwrap_or(Vec::new(env));

        let mut projects = Vec::new(env);
        for project_id in ids.iter() {
            if let Some(project) = Self::get_project(env, project_id) {
                projects.push_back(project);
            }
        }

        projects
    }

    pub fn list_projects(
        env: &Env,
        start_id: u64,
        limit: u32,
    ) -> Result<Vec<Project>, ContractError> {
        let mut projects = Vec::new(env);
        let max_id: u64 = env
            .storage()
            .persistent()
            .get(&StorageKey::NextProjectId)
            .unwrap_or(0);
        
        let mut current_id = start_id;
        let mut count = 0;
        
        while current_id <= max_id && count < limit {
            if let Some(project) = Self::get_project(env, current_id) {
                projects.push_back(project);
                count += 1;
            }
            current_id += 1;
        }
        
        Ok(projects)
    }

    pub fn project_exists(env: &Env, project_id: u64) -> bool {
        env.storage()
            .persistent()
            .has(&DataKey::Project(project_id))
    }

    pub fn update_verification_status(
        env: &Env,
        project_id: u64,
        status: VerificationStatus,
    ) -> Result<(), ContractError> {
        let mut project = Self::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;
        project.verification_status = status.clone();
        project.updated_at = env.ledger().timestamp();
        
        env.storage()
            .persistent()
            .set(&StorageKey::Project(project_id), &project);
            
        Ok(())
    }
}
