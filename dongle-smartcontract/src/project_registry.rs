use crate::errors::ContractError;
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
    ) -> u64 {
        owner.require_auth();

        let mut count: u64 = env
            .storage()
            .persistent()
            .get(&DataKey::ProjectCount)
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
            .set(&DataKey::Project(count), &project);
        env.storage()
            .persistent()
            .set(&DataKey::ProjectCount, &count);

        let mut owner_projects: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::OwnerProjects(owner.clone()))
            .unwrap_or(Vec::new(env));
        owner_projects.push_back(count);
        env.storage()
            .persistent()
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
    ) -> Option<Project> {
        let mut project = Self::get_project(env, project_id)?;

        caller.require_auth();
        if project.owner != caller {
            return None;
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

        Some(project)
    }

    pub fn get_project(env: &Env, project_id: u64) -> Option<Project> {
        env.storage()
            .persistent()
            .get(&DataKey::Project(project_id))
    }

    pub fn get_projects_by_owner(env: &Env, owner: Address) -> Vec<Project> {
        let ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::OwnerProjects(owner))
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
        _env: &Env,
        _start_id: u64,
        _limit: u32,
    ) -> Result<Vec<Project>, ContractError> {
        todo!("Project listing logic not implemented")
    }

    pub fn project_exists(env: &Env, project_id: u64) -> bool {
        env.storage()
            .persistent()
            .has(&DataKey::Project(project_id))
    }

    pub fn validate_project_data(
        _name: &String,
        _description: &String,
        _category: &String,
    ) -> Result<(), ContractError> {
        todo!("Project data validation not implemented")
    }
}
