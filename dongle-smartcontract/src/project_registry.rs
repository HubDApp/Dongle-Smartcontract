extern crate alloc;
use alloc::string::ToString;

use crate::errors::ContractError;
use crate::storage_keys::StorageKey;
use crate::types::Project;
use soroban_sdk::{Address, Env, String};

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
        // Generate unique project ID
        // Save project in Map<u64, Project>
        // Emit ProjectRegistered event
        0
    }

    pub fn update_project(env: &Env, project_id: u64, caller: Address) {
        // Validate ownership
        // Update project metadata
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
            .get(&StorageKey::Project(project_id))
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
            .set(&StorageKey::Project(project_id), &project);

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

    /// Returns the number of projects registered by an owner (for tests and admin).
    pub fn get_owner_project_count(env: &Env, owner: &Address) -> u32 {
        Self::owner_project_count(env, owner)
    }

    pub fn validate_project_data(
        name: &String,
        _description: &String,
        _category: &String,
    ) -> Result<(), ContractError> {
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
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use crate::{DongleContract, DongleContractClient};
    use soroban_sdk::{
        testutils::{Address as _, Events, Ledger, LedgerInfo},
        Address, Env, String,
    };

    fn ledger_at(timestamp: u64) -> LedgerInfo {
        LedgerInfo {
            timestamp,
            protocol_version: 20,
            sequence_number: 1,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 16,
            min_persistent_entry_ttl: 100_000,
            max_entry_ttl: 10_000_000,
    pub fn validate_project_data(
        _name: &String,
        _description: &String,
        _category: &String,
    ) -> Result<(), ContractError> {
        todo!("Project data validation not implemented")
    }

    #[test]
    fn test_valid_project_name() {
        let env = Env::default();
        let name = String::from_str(&env, "Valid-Project_Name123");
        
        let result = super::ProjectRegistry::validate_project_data(
            &name,
            &String::from_str(&env, "Desc"),
            &String::from_str(&env, "Cat")
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_empty_or_whitespace_name() {
        let env = Env::default();
        let name = String::from_str(&env, "   ");
        
        let result = super::ProjectRegistry::validate_project_data(
            &name,
            &String::from_str(&env, "Desc"),
            &String::from_str(&env, "Cat")
        );
        assert_eq!(result, Err(ContractError::InvalidProjectData));
    }

    #[test]
    fn test_invalid_characters_in_name() {
        let env = Env::default();
        let name = String::from_str(&env, "My Project *");
        
        let result = super::ProjectRegistry::validate_project_data(
            &name,
            &String::from_str(&env, "Desc"),
            &String::from_str(&env, "Cat")
        );
        assert_eq!(result, Err(ContractError::InvalidProjectNameFormat));
    }

    #[test]
    fn test_name_too_long() {
        let env = Env::default();
        // 51 characters
        let name = String::from_str(&env, "ThisProjectNameIsWayTooLongAndExceedsTheFiftyCharL1");
        
        let result = super::ProjectRegistry::validate_project_data(
            &name,
            &String::from_str(&env, "Desc"),
            &String::from_str(&env, "Cat")
        );
        assert_eq!(result, Err(ContractError::ProjectNameTooLong));
    }
    }
}
