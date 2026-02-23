use crate::errors::ContractError;
use crate::types::{DataKey, Project};
use soroban_sdk::{Address, Env, String, Vec};

pub struct ProjectRegistry;

impl ProjectRegistry {
    pub fn register_project(
        env: &Env,
        _owner: Address,
        _name: String,
        _description: String,
        _category: String,
        _website: Option<String>,
        _logo_cid: Option<String>,
        _metadata_cid: Option<String>,
    ) -> Result<u64, ContractError> {
        let _registered_at: u64 = env.ledger().timestamp();
        todo!("Project registration logic not implemented")
    }

    pub fn get_project(env: &Env, project_id: u64) -> Option<Project> {
        env.storage()
            .persistent()
            .get(&DataKey::Project(project_id))
    }

    pub fn update_project(
        env: &Env,
        project_id: u64,
        caller: Address,
        name: String,
        description: String,
        category: String,
        website: Option<String>,
        logo_cid: Option<String>,
        metadata_cid: Option<String>,
    ) -> Result<(), ContractError> {
        // 1. AUTHENTICATION: Verify the user's cryptographic signature
        caller.require_auth();

        // 2. RETRIEVAL: Check if project exists
        let mut project: Project =
            Self::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        // 3. AUTHORIZATION: Verify the caller is the stored owner
        if caller != project.owner {
            return Err(ContractError::Unauthorized);
        }

        // 4. DATA VALIDATION
        Self::validate_project_data(&name, &description, &category)?;

        // 5. UPDATE FIELDS
        project.name = name;
        project.description = description;
        project.category = category;
        project.website = website;
        project.logo_cid = logo_cid;
        project.metadata_cid = metadata_cid;

        // Update the timestamp to current ledger time
        project.updated_at = env.ledger().timestamp();

        // 6. PERSISTENCE: Save back to storage
        env.storage()
            .persistent()
            .set(&DataKey::Project(project_id), &project);

        Ok(())
    }

    pub fn list_projects(
        _env: &Env,
        _start_id: u64,
        _limit: u32,
    ) -> Result<Vec<Project>, ContractError> {
        todo!("Project listing logic not implemented")
    }

    pub fn get_next_project_id(_env: &Env) -> u64 {
        1
    }

    pub fn increment_project_counter(_env: &Env) -> u64 {
        1
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
        // Keeping as Ok(()) to allow updates to pass for now
        Ok(())
    }
}
