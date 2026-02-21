//! Project registration with validation, per-user limits, and events.

use crate::constants::*;
use crate::errors::Error;
use crate::events::ProjectRegistered;
use crate::events::ProjectUpdated;
use crate::storage_keys::StorageKey;
use crate::types::Project;
use soroban_sdk::{Address, Env, String as SorobanString};

fn validate_string_length(s: &str, max: usize) -> Result<(), Error> {
    if s.len() > max {
        return Err(Error::StringLengthExceeded);
    }
    Ok(())
}

fn validate_optional_string(s: &Option<String>, max: usize) -> Result<(), Error> {
    if let Some(ref x) = s {
        validate_string_length(x, max)?;
    }
    Ok(())
}

/// Validates project registration inputs. Returns Ok(()) or Err.
pub fn validate_project_inputs(
    name: &str,
    description: &str,
    category: &str,
    website: &Option<String>,
    logo_cid: &Option<String>,
    metadata_cid: &Option<String>,
) -> Result<(), Error> {
    if name.trim().len() < MIN_STRING_LEN {
        return Err(Error::InvalidProjectName);
    }
    if description.trim().len() < MIN_STRING_LEN {
        return Err(Error::InvalidProjectDescription);
    }
    if category.trim().len() < MIN_STRING_LEN {
        return Err(Error::InvalidProjectCategory);
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
        let key = StorageKey::NextProjectId;
        let next: u64 = env
            .storage()
            .persistent()
            .get(&key)
            .unwrap_or(1);
        next
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
    ) -> Result<u64, Error> {
        validate_project_inputs(&name, &description, &category, &website, &logo_cid, &metadata_cid)?;

        let count = Self::owner_project_count(env, &owner);
        if count >= MAX_PROJECTS_PER_USER {
            return Err(Error::MaxProjectsPerUserExceeded);
        }

        let project_id = Self::next_project_id(env);
        if project_id == 0 {
            return Err(Error::InvalidProjectId);
        }

        let ledger_timestamp = env.ledger().timestamp();
        let project = Project {
            id: project_id,
            owner: owner.clone(),
            name: SorobanString::from_str(env, &name),
            description: SorobanString::from_str(env, &description),
            category: SorobanString::from_str(env, &category),
            website: website.map(|s| SorobanString::from_str(env, &s)),
            logo_cid: logo_cid.map(|s| SorobanString::from_str(env, &s)),
            metadata_cid: metadata_cid.map(|s| SorobanString::from_str(env, &s)),
            created_at: ledger_timestamp,
            updated_at: ledger_timestamp,
        };

        env.storage()
            .persistent()
            .set(&StorageKey::Project(project_id), &project);
        Self::set_next_project_id(env, project_id);
        Self::inc_owner_project_count(env, &owner);

        ProjectRegistered {
            project_id,
            owner: owner.clone(),
            name: SorobanString::from_str(env, &name),
            category: SorobanString::from_str(env, &category),
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
    ) -> Result<(), Error> {
        if project_id == 0 {
            return Err(Error::InvalidProjectId);
        }
        let mut project: Project = env
            .storage()
            .persistent()
            .get(&StorageKey::Project(project_id))
            .ok_or(Error::ProjectNotFound)?;

        if project.owner != caller {
            return Err(Error::NotProjectOwner);
        }

        validate_project_inputs(&name, &description, &category, &website, &logo_cid, &metadata_cid)?;

        let ledger_timestamp = env.ledger().timestamp();
        project.name = SorobanString::from_str(env, &name);
        project.description = SorobanString::from_str(env, &description);
        project.category = SorobanString::from_str(env, &category);
        project.website = website.map(|s| SorobanString::from_str(env, &s));
        project.logo_cid = logo_cid.map(|s| SorobanString::from_str(env, &s));
        project.metadata_cid = metadata_cid.map(|s| SorobanString::from_str(env, &s));
        project.updated_at = ledger_timestamp;

        env.storage()
            .persistent()
            .set(&StorageKey::Project(project_id), &project);

        ProjectUpdated {
            project_id,
            owner: caller,
            updated_at: ledger_timestamp,
        }
        .publish(env);

        Ok(())
    }

    pub fn get_project(env: &Env, project_id: u64) -> Result<Option<Project>, Error> {
        if project_id == 0 {
            return Err(Error::InvalidProjectId);
        }
        let project: Option<Project> = env.storage().persistent().get(&StorageKey::Project(project_id));
        Ok(project)
    }

    /// Returns the number of projects registered by an owner (for tests and admin).
    pub fn get_owner_project_count(env: &Env, owner: &Address) -> u32 {
        Self::owner_project_count(env, owner)
    }
}
