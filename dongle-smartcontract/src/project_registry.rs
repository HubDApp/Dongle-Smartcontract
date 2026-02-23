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

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{
        testutils::{Address as _, Events, Ledger, LedgerInfo},
        vec, Address, Env, IntoVal, TryIntoVal,
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

    // Registers the contract and returns a client to call it through
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
            &None, &None, &None,
        );
        let id1 = client.register_project(
            &owner,
            &String::from_str(&env, "Beta"),
            &String::from_str(&env, "desc"),
            &String::from_str(&env, "NFT"),
            &None, &None, &None,
        );

        assert_eq!(id0, 0);
        assert_eq!(id1, 1);
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
        assert_eq!(project.registered_at, 1_700_000_000);
    }

    #[test]
    fn test_event_is_emitted_on_registration() {
        let env = Env::default();
        env.mock_all_auths();
        env.ledger().set(ledger_at(1_710_000_000));
        let client = setup(&env);
        let owner = Address::generate(&env);

        let id = client.register_project(
            &owner,
            &String::from_str(&env, "EventTest"),
            &String::from_str(&env, "Testing events"),
            &String::from_str(&env, "Testing"),
            &None, &None, &None,
        );

        let all_events = env.events().all();
        assert!(!all_events.is_empty());

        let (_, topics, data) = all_events.last().unwrap();

        let expected_topics = vec![
            &env,
            symbol_short!("ProjReg").into_val(&env),
            owner.into_val(&env),
        ];
        assert_eq!(topics, expected_topics);

        let (emitted_id, emitted_ts): (u64, u64) = data.try_into_val(&env).unwrap();
        assert_eq!(emitted_id, id);
        assert_eq!(emitted_ts, 1_710_000_000u64);
    }

    #[test]
    fn test_one_event_per_registration() {
        let env = Env::default();
        env.mock_all_auths();
        let client = setup(&env);
        let owner = Address::generate(&env);

        for _ in 0..3 {
            client.register_project(
                &owner,
                &String::from_str(&env, "P"),
                &String::from_str(&env, "d"),
                &String::from_str(&env, "c"),
                &None, &None, &None,
            );
        }

        assert_eq!(env.events().all().len(), 3);
    }
}