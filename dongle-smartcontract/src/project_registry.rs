//! Project registration with validation, per-user limits, and events.
use crate::constants::*;
use crate::errors::ContractError;
use crate::events::ProjectRegistered;
use crate::events::ProjectUpdated;
use crate::storage_keys::StorageKey;
use crate::types::Project;
use soroban_sdk::{Address, Env, String};

fn validate_string_length(s: &String, max: usize) -> Result<(), ContractError> {
    if s.len() as usize > max {
        return Err(ContractError::StringLengthExceeded);
    }
    Ok(())
}

fn validate_optional_string(s: &Option<String>, max: usize) -> Result<(), ContractError> {
    if let Some(ref x) = s {
        validate_string_length(x, max)?;
    }
    Ok(())
}

pub fn validate_project_inputs(
    name: &String,
    description: &String,
    category: &String,
    website: &Option<String>,
    logo_cid: &Option<String>,
    metadata_cid: &Option<String>,
) -> Result<(), ContractError> {
    // Soroban String has no trim() — check raw len against MIN_STRING_LEN
    if (name.len() as usize) < MIN_STRING_LEN {
        return Err(ContractError::InvalidProjectName);
    }
    if (description.len() as usize) < MIN_STRING_LEN {
        return Err(ContractError::InvalidProjectDescription);
    }
    if (category.len() as usize) < MIN_STRING_LEN {
        return Err(ContractError::InvalidProjectCategory);
    }
    validate_string_length(name, MAX_NAME_LEN)?;
    validate_string_length(description, MAX_DESCRIPTION_LEN)?;
    validate_string_length(category, MAX_CATEGORY_LEN)?;
    validate_optional_string(website, MAX_WEBSITE_LEN)?;
    validate_optional_string(logo_cid, MAX_CID_LEN)?;
    validate_optional_string(metadata_cid, MAX_CID_LEN)?;
    Ok(())
}

pub struct ProjectRegistry;

impl ProjectRegistry {
    fn next_project_id(env: &Env) -> u64 {
        env.storage()
            .persistent()
            .get(&StorageKey::NextProjectId)
            .unwrap_or(1)
    }

    fn set_next_project_id(env: &Env, id: u64) {
        env.storage()
            .persistent()
            .set(&StorageKey::NextProjectId, &(id + 1));
    }

    fn owner_project_count(env: &Env, owner: &Address) -> u32 {
        env.storage()
            .persistent()
            .get(&StorageKey::OwnerProjectCount(owner.clone()))
            .unwrap_or(0)
    }

    fn inc_owner_project_count(env: &Env, owner: &Address) {
        let count = Self::owner_project_count(env, owner);
        env.storage()
            .persistent()
            .set(&StorageKey::OwnerProjectCount(owner.clone()), &(count + 1));
    }

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
        owner.require_auth();

        validate_project_inputs(
            &name,
            &description,
            &category,
            &website,
            &logo_cid,
            &metadata_cid,
        )?;

        let count = Self::owner_project_count(env, &owner);
        if count >= MAX_PROJECTS_PER_USER {
            return Err(ContractError::MaxProjectsPerUserExceeded);
        }

        let project_id = Self::next_project_id(env);
        if project_id == 0 {
            return Err(ContractError::InvalidProjectId);
        }

        let ledger_timestamp = env.ledger().timestamp();
        let project = Project {
            id: project_id,
            owner: owner.clone(),
            name: name.clone(),
            description: description.clone(),
            category: category.clone(),
            website: website.clone(),
            logo_cid: logo_cid.clone(),
            metadata_cid: metadata_cid.clone(),
            created_at: ledger_timestamp,
            updated_at: ledger_timestamp,
        };

    pub fn get_project(env: &Env, project_id: u64) -> Option<Project> {
        env.storage()
            .persistent()
            .set(&StorageKey::Project(project_id), &project);
        Self::set_next_project_id(env, project_id);
        Self::inc_owner_project_count(env, &owner);

        ProjectRegistered {
            project_id,
            owner: owner.clone(),
            name: name.clone(),
            category: category.clone(),
        }
        .publish(env);

        Ok(project_id)
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
        caller.require_auth();

        if project_id == 0 {
            return Err(ContractError::InvalidProjectId);
        }

        let mut project: Project = env
            .storage()
            .persistent()
            .get(&StorageKey::Project(project_id))
            .ok_or(ContractError::ProjectNotFound)?;

        if project.owner != caller {
            return Err(ContractError::NotProjectOwner);
        }

        validate_project_inputs(
            &name,
            &description,
            &category,
            &website,
            &logo_cid,
            &metadata_cid,
        )?;

        let ledger_timestamp = env.ledger().timestamp();
        project.name = name;
        project.description = description;
        project.category = category;
        project.website = website;
        project.logo_cid = logo_cid;
        project.metadata_cid = metadata_cid;
        project.updated_at = ledger_timestamp;

        // Update the timestamp to current ledger time
        project.updated_at = env.ledger().timestamp();

        // 6. PERSISTENCE: Save back to storage
        env.storage()
            .persistent()
            .set(&DataKey::Project(project_id), &project);

        Ok(())
    }

    pub fn get_project(env: &Env, project_id: u64) -> Result<Option<Project>, ContractError> {
        if project_id == 0 {
            return Err(ContractError::InvalidProjectId);
        }
        Ok(env
            .storage()
            .persistent()
            .get(&StorageKey::Project(project_id)))
    }

    pub fn get_owner_project_count(env: &Env, owner: &Address) -> u32 {
        Self::owner_project_count(env, owner)
    }
}
