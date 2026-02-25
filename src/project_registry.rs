//! Project registration with validation, per-user limits, and events.

use crate::constants::*;
use crate::errors::ContractError;
use crate::events::{ProjectRegistered, ProjectUpdated};
use crate::types::{DataKey, Project};
use soroban_sdk::{Address, Env, String as SorobanString};

// --- Validation Helpers ---

/// Checks if the provided category matches the allowed enumeration (Issue #8).
fn is_valid_category(env: &Env, category: &SorobanString) -> bool {
    let categories = [
        "DeFi", "NFT", "Gaming", "DAO", "Tools"
    ];
    for cat in categories {
        if category == &SorobanString::from_str(env, cat) {
            return true;
        }
    }
    false
}
fn validate_string_length(s: &SorobanString, max: usize) -> Result<(), ContractError> {
    if s.len() as usize > max {
        return Err(ContractError::ProjectNameTooLong);
    }
    Ok(())
}

fn validate_optional_string(s: &Option<SorobanString>, max: usize) -> Result<(), ContractError> {
    if let Some(ref x) = s {
        validate_string_length(x, max)?;
    }
    Ok(())
}

/// Validates project registration inputs. Returns Ok(()) or Err.
pub fn validate_project_inputs(
    name: &SorobanString,
    description: &SorobanString,
    category: &SorobanString,
    website: &Option<SorobanString>,
    logo_cid: &Option<SorobanString>,
    metadata_cid: &Option<SorobanString>,
) -> Result<(), ContractError> {
    if name.len() < MIN_STRING_LEN as u32 {
        return Err(ContractError::InvalidProjectData);
    }
    if description.len() < MIN_STRING_LEN as u32 {
        return Err(ContractError::InvalidProjectData);
    }

    // Strict category validation for Issue #8
    if !is_valid_category(category) {
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
            .get(&DataKey::NextProjectId)
            .unwrap_or(1)
    }

    fn set_next_project_id(env: &Env, id: u64) {
        env.storage()
            .persistent()
            .set(&DataKey::NextProjectId, &(id + 1));
    }

    fn owner_project_count(env: &Env, owner: &Address) -> u32 {
        env.storage()
            .persistent()
            .get(&DataKey::OwnerProjectCount(owner.clone()))
            .unwrap_or(0)
    }

    fn inc_owner_project_count(env: &Env, owner: &Address) {
        let count = Self::owner_project_count(env, owner);
        env.storage()
            .persistent()
            .set(&DataKey::OwnerProjectCount(owner.clone()), &(count + 1));
    }

    pub fn register_project(
        env: &Env,
        owner: Address,
        name: SorobanString,
        description: SorobanString,
        category: SorobanString,
        website: Option<SorobanString>,
        logo_cid: Option<SorobanString>,
        metadata_cid: Option<SorobanString>,
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
            return Err(ContractError::InvalidProjectData);
        }

        let project_id = Self::next_project_id(env);
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
            .set(&DataKey::Project(project_id), &project);

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
        name: SorobanString,
        description: SorobanString,
        category: SorobanString,
        website: Option<SorobanString>,
        logo_cid: Option<SorobanString>,
        metadata_cid: Option<SorobanString>,
    ) -> Result<(), ContractError> {
        caller.require_auth();

        let mut project: Project = env
            .storage()
            .persistent()
            .get(&DataKey::Project(project_id))
            .ok_or(ContractError::ProjectNotFound)?;

        if project.owner != caller {
            return Err(ContractError::Unauthorized);
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
            .set(&DataKey::Project(project_id), &project);

        ProjectUpdated {
            project_id,
            owner: caller,
            updated_at: ledger_timestamp,
        }
        .publish(env);

        Ok(())
    }

    pub fn get_project(env: &Env, project_id: u64) -> Option<Project> {
        env.storage()
            .persistent()
            .get(&DataKey::Project(project_id))
    }

    pub fn get_owner_project_count(env: &Env, owner: &Address) -> u32 {
        Self::owner_project_count(env, owner)
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
        }
    }

    fn setup(env: &Env) -> DongleContractClient {
        let contract_id = env.register_contract(None, DongleContract);
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
            &String::from_str(&env, "desc"),
            &String::from_str(&env, "DeFi"),
            &None,
            &None,
            &None,
        );
        let id1 = client.register_project(
            &owner,
            &String::from_str(&env, "Beta"),
            &String::from_str(&env, "desc"),
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
        env.ledger().set(ledger_at(1_700_000_000));
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

        let project = client.get_project(&id).unwrap();
        assert_eq!(project.owner, owner);
        assert_eq!(project.name, String::from_str(&env, "Dongle"));
        assert_eq!(project.created_at, 1_700_000_000);
    }

    #[test]
    fn test_event_is_emitted_on_registration() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set(ledger_at(1_710_000_000));
        let client = setup(&env);
        let owner = Address::generate(&env);

        let _id = client.register_project(
            &owner,
            &String::from_str(&env, "EventTest"),
            &String::from_str(&env, "Testing events"),
            &String::from_str(&env, "Tools"), // Valid Category
            &None,
            &None,
            &None,
        );

        let all_events = env.events().all();
        assert!(!all_events.is_empty());
        assert_eq!(all_events.len(), 1);
    }

    #[test]
    fn test_category_assignment_and_retrieval() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup(&env);
        let owner = Address::generate(&env);

        let category = String::from_str(&env, "DeFi");
        let id = client.register_project(
            &owner,
            &String::from_str(&env, "Test Project"),
            &String::from_str(&env, "Description"),
            &category,
            &None,
            &None,
            &None,
        );

        let project = client.get_project(&id).unwrap();
        assert_eq!(project.category, category);
    }

    #[test]
    #[should_panic(expected = "Status(ContractError(14))")] // InvalidProjectCategory
    fn test_invalid_category_fails() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup(&env);
        let owner = Address::generate(&env);

        client.register_project(
            &owner,
            &String::from_str(&env, "Bad Cat"),
            &String::from_str(&env, "Desc"),
            &String::from_str(&env, "UnknownCategory"),
            &None,
            &None,
            &None,
        );
    }
}
