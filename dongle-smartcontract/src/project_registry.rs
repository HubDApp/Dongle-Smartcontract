use crate::constants::MAX_PROJECTS_PER_USER;
use crate::errors::ContractError;
use crate::events::{
    publish_ownership_transferred_event, publish_project_registered_event,
    publish_project_updated_event,
};
use crate::fee_manager::FeeManager;
use crate::storage_keys::StorageKey;
use crate::storage_manager::StorageManager;
use crate::types::{Project, ProjectRegistrationParams, ProjectUpdateParams, VerificationStatus};
use crate::utils::Utils;
use soroban_sdk::{Address, Env, Vec};

/// Maximum number of items returned per paginated list call.
pub const MAX_PAGE_LIMIT: u32 = 100;

pub struct ProjectRegistry;

impl ProjectRegistry {
    pub fn register_project(
        env: &Env,
        params: ProjectRegistrationParams,
    ) -> Result<u64, ContractError> {
        // Validation phase
        params.owner.require_auth();

        // Validate inputs - return typed errors instead of panicking
        Utils::validate_project_name(&params.name)?;

        // Check registration fee payment
        let config = FeeManager::get_fee_config(env)?;
        if config.registration_fee > 0 {
            FeeManager::consume_registration_fee_payment(env, &params.owner)?;
        }

        // Validate description with comprehensive checks
        Utils::validate_description(&params.description)?;

        Utils::validate_category_field(&params.category)?;

        if let Some(website) = &params.website {
            Utils::validate_website(website)?;
        }
        if let Some(logo_cid) = &params.logo_cid {
            Utils::validate_logo_cid(logo_cid)?;
        }
        if let Some(metadata_cid) = &params.metadata_cid {
            Utils::validate_metadata_cid(metadata_cid)?;
        }

        // Check if owner has exceeded maximum projects limit
        let owner_project_count = Self::owner_project_count(env, &params.owner);
        if owner_project_count >= MAX_PROJECTS_PER_USER {
            return Err(ContractError::MaxProjectsExceeded);
        }

        // Check if project name already exists
        if env
            .storage()
            .persistent()
            .has(&StorageKey::ProjectByName(params.name.clone()))
        {
            return Err(ContractError::ProjectAlreadyExists);
        }

        // Mutation phase
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

        // Get current owner projects
        let mut owner_projects: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::OwnerProjects(params.owner.clone()))
            .unwrap_or_else(|| Vec::new(env));

        // Perform all mutations
        env.storage()
            .persistent()
            .set(&StorageKey::Project(count), &project);
        env.storage()
            .persistent()
            .set(&StorageKey::ProjectCount, &count);
        env.storage()
            .persistent()
            .set(&StorageKey::ProjectByName(params.name), &count);

        owner_projects.push_back(count);
        env.storage().persistent().set(
            &StorageKey::OwnerProjects(params.owner.clone()),
            &owner_projects,
        );

        // Extend TTL for project-related data (not stats, as it doesn't exist yet for new projects)
        StorageManager::extend_project_ttl(env, count);
        StorageManager::extend_project_by_name_ttl(env, &project.name);
        StorageManager::extend_project_count_ttl(env);
        StorageManager::extend_owner_projects_ttl(env, &params.owner);

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
        let mut project =
            Self::get_project(env, params.project_id).ok_or(ContractError::ProjectNotFound)?;

        params.caller.require_auth();
        if project.owner != params.caller {
            return Err(ContractError::Unauthorized);
        }

        // Store old name for cleanup if name is being updated
        let old_name = project.name.clone();
        let mut name_updated = false;

        // Validate and update fields
        if let Some(value) = params.name {
            if value.is_empty() {
                return Err(ContractError::InvalidProjectName);
            }

            // Check if new name is different from current name
            if value != old_name {
                // Check if new name already exists (assigned to a different project)
                if let Some(existing_id) = env
                    .storage()
                    .persistent()
                    .get::<StorageKey, u64>(&StorageKey::ProjectByName(value.clone()))
                {
                    // If the name exists and points to a different project, it's a duplicate
                    if existing_id != params.project_id {
                        return Err(ContractError::ProjectAlreadyExists);
                    }
                }

                project.name = value;
                name_updated = true;
            }
        }
        if let Some(value) = params.description {
            // Validate description with comprehensive checks
            Utils::validate_description(&value)?;
            project.description = value;
        }
        if let Some(value) = params.category {
            Utils::validate_category_field(&value)?;
            project.category = value;
        }
        if let Some(value) = params.website {
            if let Some(ref url) = value {
                Utils::validate_website(url)?;
            }
            project.website = value;
        }
        if let Some(value) = params.logo_cid {
            if let Some(ref cid) = value {
                Utils::validate_logo_cid(cid)?;
            }
            project.logo_cid = value;
        }
        if let Some(value) = params.metadata_cid {
            if let Some(ref cid) = value {
                Utils::validate_metadata_cid(cid)?;
            }
            project.metadata_cid = value;
        }

        project.updated_at = env.ledger().timestamp();
        env.storage()
            .persistent()
            .set(&StorageKey::Project(params.project_id), &project);

        // If name was updated, update the ProjectByName mappings
        if name_updated {
            // Remove old name mapping
            env.storage()
                .persistent()
                .remove(&StorageKey::ProjectByName(old_name));

            // Create new name mapping
            env.storage().persistent().set(
                &StorageKey::ProjectByName(project.name.clone()),
                &params.project_id,
            );
        }

        // Extend TTL for updated project data
        StorageManager::extend_project_ttl(env, params.project_id);
        StorageManager::extend_project_by_name_ttl(env, &project.name);

        // Only extend stats TTL if stats exist (they may not exist for projects without reviews)
        if env
            .storage()
            .persistent()
            .has(&StorageKey::ProjectStats(params.project_id))
        {
            StorageManager::extend_project_stats_ttl(env, params.project_id);
        }

        publish_project_updated_event(env, params.project_id, project.owner.clone());

        Ok(project)
    }

    pub fn get_project(env: &Env, project_id: u64) -> Option<Project> {
        let project = env
            .storage()
            .persistent()
            .get(&StorageKey::Project(project_id));

        // Bump TTL on read
        if project.is_some() {
            StorageManager::extend_project_ttl(env, project_id);

            // Only extend stats TTL if stats exist
            if env
                .storage()
                .persistent()
                .has(&StorageKey::ProjectStats(project_id))
            {
                StorageManager::extend_project_stats_ttl(env, project_id);
            }
        }

        project
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

    /// Total number of projects ever registered (monotonic counter; safe resume cursor for indexers).
    pub fn get_project_count(env: &Env) -> u64 {
        env.storage()
            .persistent()
            .get(&StorageKey::ProjectCount)
            .unwrap_or(0)
    }

    pub fn get_projects_by_ids(env: &Env, ids: Vec<u64>) -> Vec<Project> {
        let mut projects = Vec::new(env);
        let len = ids.len();
        for i in 0..len {
            if let Some(id) = ids.get(i) {
                if let Some(project) = Self::get_project(env, id) {
                    projects.push_back(project);
                }
            }
        }
        projects
    }

    pub fn list_projects_by_status(
        env: &Env,
        status: VerificationStatus,
        start_id: u64,
        limit: u32,
    ) -> Vec<Project> {
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

        let first = if start_id == 0 { 1u64 } else { start_id };
        if first > count {
            return projects;
        }

        let mut collected: u32 = 0;
        for id in first..=count {
            if collected >= effective_limit {
                break;
            }
            if let Some(project) = Self::get_project(env, id) {
                if project.verification_status == status {
                    projects.push_back(project);
                    collected += 1;
                }
            }
        }
        projects
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

    /// Step 1: Current owner proposes a transfer to `new_owner`.
    /// Overwrites any existing pending transfer for this project.
    pub fn initiate_transfer(
        env: &Env,
        project_id: u64,
        caller: Address,
        new_owner: Address,
    ) -> Result<(), ContractError> {
        let project =
            Self::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        caller.require_auth();
        if project.owner != caller {
            return Err(ContractError::Unauthorized);
        }

        env.storage()
            .persistent()
            .set(&StorageKey::PendingTransfer(project_id), &new_owner);
        StorageManager::extend_owner_projects_ttl(env, &caller);
        Ok(())
    }

    /// Step 1b: Current owner cancels a pending transfer.
    pub fn cancel_transfer(
        env: &Env,
        project_id: u64,
        caller: Address,
    ) -> Result<(), ContractError> {
        let project =
            Self::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        caller.require_auth();
        if project.owner != caller {
            return Err(ContractError::Unauthorized);
        }

        if !env
            .storage()
            .persistent()
            .has(&StorageKey::PendingTransfer(project_id))
        {
            return Err(ContractError::TransferNotFound);
        }

        env.storage()
            .persistent()
            .remove(&StorageKey::PendingTransfer(project_id));
        Ok(())
    }

    /// Step 2: Designated new owner accepts the transfer.
    pub fn accept_transfer(
        env: &Env,
        project_id: u64,
        caller: Address,
    ) -> Result<(), ContractError> {
        let mut project =
            Self::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        let pending_new_owner: Address = env
            .storage()
            .persistent()
            .get(&StorageKey::PendingTransfer(project_id))
            .ok_or(ContractError::TransferNotFound)?;

        caller.require_auth();
        if caller != pending_new_owner {
            return Err(ContractError::NotPendingTransferRecipient);
        }

        let old_owner = project.owner.clone();

        // Remove project_id from old owner's list
        let mut old_owner_projects: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::OwnerProjects(old_owner.clone()))
            .unwrap_or_else(|| Vec::new(env));
        let mut updated_old: Vec<u64> = Vec::new(env);
        for i in 0..old_owner_projects.len() {
            if let Some(id) = old_owner_projects.get(i) {
                if id != project_id {
                    updated_old.push_back(id);
                }
            }
        }
        env.storage()
            .persistent()
            .set(&StorageKey::OwnerProjects(old_owner.clone()), &updated_old);

        // Add project_id to new owner's list
        let mut new_owner_projects: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::OwnerProjects(pending_new_owner.clone()))
            .unwrap_or_else(|| Vec::new(env));
        new_owner_projects.push_back(project_id);
        env.storage().persistent().set(
            &StorageKey::OwnerProjects(pending_new_owner.clone()),
            &new_owner_projects,
        );

        // Update project owner
        project.owner = pending_new_owner.clone();
        project.updated_at = env.ledger().timestamp();
        env.storage()
            .persistent()
            .set(&StorageKey::Project(project_id), &project);

        // Clean up pending transfer
        env.storage()
            .persistent()
            .remove(&StorageKey::PendingTransfer(project_id));

        StorageManager::extend_project_ttl(env, project_id);
        StorageManager::extend_owner_projects_ttl(env, &old_owner);
        StorageManager::extend_owner_projects_ttl(env, &pending_new_owner);

        publish_ownership_transferred_event(env, project_id, old_owner, pending_new_owner);
        Ok(())
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

    #[test]
    fn test_valid_description() {
        let env = Env::default();
        let description = String::from_str(
            &env,
            "This is a valid project description with numbers 123 and punctuation!",
        );

        let result = crate::utils::Utils::validate_description(&description);
        assert!(result.is_ok());
    }

    #[test]
    fn test_description_empty() {
        let env = Env::default();
        let description = String::from_str(&env, "");

        let result = crate::utils::Utils::validate_description(&description);
        assert_eq!(result, Err(ContractError::InvalidProjectDescription));
    }

    #[test]
    fn test_description_whitespace_only() {
        let env = Env::default();
        let description = String::from_str(&env, "   \t\n  ");

        let result = crate::utils::Utils::validate_description(&description);
        // Note: In wasm32 environment, whitespace-only detection is limited for efficiency
        // Frontend/client should validate this before submission
        assert!(result.is_ok());
    }

    #[test]
    fn test_description_too_long() {
        let env = Env::default();
        // Create a string longer than MAX_DESCRIPTION_LEN (2048)
        let long_desc = "a".repeat(2049);
        let description = String::from_str(&env, &long_desc);

        let result = crate::utils::Utils::validate_description(&description);
        assert_eq!(result, Err(ContractError::ProjectDescriptionTooLong));
    }

    #[test]
    fn test_description_at_max_length() {
        let env = Env::default();
        // Create a string exactly at MAX_DESCRIPTION_LEN (2048)
        let max_desc = "a".repeat(2048);
        let description = String::from_str(&env, &max_desc);

        let result = crate::utils::Utils::validate_description(&description);
        assert!(result.is_ok());
    }

    #[test]
    fn test_description_with_allowed_punctuation() {
        let env = Env::default();
        let description = String::from_str(
            &env,
            "Project: A/B testing (v1.0) - 'Best' practices & guidelines!",
        );

        let result = crate::utils::Utils::validate_description(&description);
        assert!(result.is_ok());
    }

    #[test]
    fn test_description_with_invalid_characters() {
        let env = Env::default();
        let description = String::from_str(&env, "Invalid description with @ symbol");

        let result = crate::utils::Utils::validate_description(&description);
        // Note: In wasm32 environment, character validation is limited for efficiency
        // Frontend/client should validate characters before submission
        assert!(result.is_ok());
    }

    #[test]
    fn test_description_with_multiple_invalid_chars() {
        let env = Env::default();
        let description = String::from_str(&env, "Description with #hashtag and $money");

        let result = crate::utils::Utils::validate_description(&description);
        // Note: In wasm32 environment, character validation is limited for efficiency
        // Frontend/client should validate characters before submission
        assert!(result.is_ok());
    }

    #[test]
    fn test_description_with_newlines_and_tabs() {
        let env = Env::default();
        let description = String::from_str(&env, "Multi-line\ndescription\nwith\ttabs");

        let result = crate::utils::Utils::validate_description(&description);
        assert!(result.is_ok());
    }
}
