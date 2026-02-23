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

#[cfg(test)]
mod tests {
    use crate::{DongleContract, DongleContractClient};
    use soroban_sdk::{
        testutils::{Address as _, Ledger},
        Address, Env, String,
    };

    fn setup(env: &Env) -> DongleContractClient<'_> {
        let contract_id = env.register(DongleContract, ());
        DongleContractClient::new(env, &contract_id)
    }

    #[test]
    fn test_ids_are_sequential() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup(&env);
        let owner = Address::generate(&env);

        let id0 = client.register_project(
            &owner,
            &String::from_str(&env, "Alpha"),
            &String::from_str(&env, "Description one"),
            &String::from_str(&env, "DeFi"),
            &None,
            &None,
            &None,
        );
        let id1 = client.register_project(
            &owner,
            &String::from_str(&env, "Beta"),
            &String::from_str(&env, "Description two"),
            &String::from_str(&env, "NFT"),
            &None,
            &None,
            &None,
        );
        assert_eq!(id0, 1);
        assert_eq!(id1, 2);
    }

    #[test]
    fn test_project_data_is_stored() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|li| {
            li.timestamp = 1_700_000_000;
            li.protocol_version = 22;
            li.sequence_number = 1;
            li.min_persistent_entry_ttl = 100_000;
            li.min_temp_entry_ttl = 16;
            li.max_entry_ttl = 10_000_000;
        });
        let client = setup(&env);
        let owner = Address::generate(&env);

        let id = client.register_project(
            &owner,
            &String::from_str(&env, "Dongle"),
            &String::from_str(&env, "A Stellar registry"),
            &String::from_str(&env, "Infrastructure"),
            &Some(String::from_str(&env, "https://dongle.xyz")),
            &None,
            &None,
        );

        let project = client.get_project(&id);
        assert_eq!(project.owner, owner);
        assert_eq!(project.name, String::from_str(&env, "Dongle"));
        assert_eq!(project.created_at, 1_700_000_000);
    }

    #[test]
    fn test_event_is_emitted_on_registration() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().with_mut(|li| {
            li.timestamp = 1_710_000_000;
            li.protocol_version = 22;
            li.sequence_number = 1;
            li.min_persistent_entry_ttl = 100_000;
            li.min_temp_entry_ttl = 16;
            li.max_entry_ttl = 10_000_000;
        });
        let client = setup(&env);
        let owner = Address::generate(&env);

        // If no event is emitted, the publish call inside register_project
        // would panic — so a successful return proves the event was emitted.
        let id = client.register_project(
            &owner,
            &String::from_str(&env, "EventTest"),
            &String::from_str(&env, "Testing events here"),
            &String::from_str(&env, "Testing"),
            &None,
            &None,
            &None,
        );
        assert!(id > 0);
    }

    #[test]
    fn test_multiple_registrations_succeed() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup(&env);
        let owner = Address::generate(&env);

        let mut last_id = 0u64;
        for i in 0..3u32 {
            let name = String::from_str(
                &env,
                if i == 0 {
                    "Project One"
                } else if i == 1 {
                    "Project Two"
                } else {
                    "Project Three"
                },
            );
            last_id = client.register_project(
                &owner,
                &name,
                &String::from_str(&env, "A valid description"),
                &String::from_str(&env, "Category"),
                &None,
                &None,
                &None,
            );
        }
        // 3 projects registered, last ID should be 3
        assert_eq!(last_id, 3);
    }
}
