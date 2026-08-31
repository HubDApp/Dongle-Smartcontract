//! Tests for project region metadata (#238) and project integrity hash (#250).

use crate::constants::MAX_DESCRIPTION_LEN;
use crate::project_registry::ProjectRegistry;
use crate::types::ProjectRegistrationParams;
use crate::{DongleContract, DongleContractClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    Address, Env, String,
};

fn setup(env: &Env) -> (DongleContractClient<'_>, Address) {
    env.ledger().set(LedgerInfo {
        timestamp: 1_700_000_000,
        protocol_version: 22,
        sequence_number: 1,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 16,
        min_persistent_entry_ttl: 4096,
        max_entry_ttl: 6_312_000,
    });
    let contract_id = env.register(DongleContract, ());
    let client = DongleContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.mock_all_auths().initialize(&admin);
    (client, admin)
}

fn register_project(client: &DongleContractClient<'_>, env: &Env, owner: &Address) -> u64 {
    client
        .mock_all_auths()
        .register_project(&ProjectRegistrationParams {
            owner: owner.clone(),
            name: String::from_str(env, "Test-Project"),
            slug: String::from_str(env, "test-project"),
            description: String::from_str(env, "A test project description"),
            category: String::from_str(env, "DeFi"),
            website: None,
            license: None,
            logo_cid: None,
            metadata_cid: None,
            tags: None,
            social_links: None,
            launch_timestamp: None,
            bounty_url: None,
            repository_url: None,
        })
}

#[test]
fn test_region_is_none_by_default() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);
    let project_id = register_project(&client, &env, &owner);

    let region = client.get_project_region(&project_id);
    assert!(region.is_none(), "Region should be None when never set");
}

#[test]
fn test_owner_can_set_and_get_region() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);
    let project_id = register_project(&client, &env, &owner);

    let region_str = String::from_str(&env, "AFRICA");
    client
        .mock_all_auths()
        .set_project_region(&project_id, &owner, &Some(region_str.clone()));

    let stored = client.get_project_region(&project_id);
    assert_eq!(stored, Some(region_str));
}

#[test]
fn test_region_accepts_documented_codes_and_rejects_invalid_values() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);

    for region in [
        "AFRICA",
        "ASIA",
        "EU",
        "LATAM",
        "NA",
        "GLOBAL",
        "north_america",
    ] {
        let project_id = register_project(&client, &env, &owner);
        let result = client.mock_all_auths().try_set_project_region(
            &project_id,
            &owner,
            &Some(String::from_str(&env, region)),
        );
        assert!(result.is_ok(), "Region should be accepted: {region}");
    }

    let invalid_project_id = register_project(&client, &env, &owner);
    let invalid = client.mock_all_auths().try_set_project_region(
        &invalid_project_id,
        &owner,
        &Some(String::from_str(&env, "INVALID")),
    );
    assert!(invalid.is_err(), "Invalid region should be rejected");
}

#[test]
fn test_owner_can_change_region_value_when_still_valid() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);
    let project_id = register_project(&client, &env, &owner);

    client.mock_all_auths().set_project_region(
        &project_id,
        &owner,
        &Some(String::from_str(&env, "AFRICA")),
    );
    client.mock_all_auths().set_project_region(
        &project_id,
        &owner,
        &Some(String::from_str(&env, "ASIA")),
    );

    assert_eq!(
        client.get_project_region(&project_id),
        Some(String::from_str(&env, "ASIA"))
    );
}

#[test]
fn test_region_based_filtering_matches_project_region_values() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);

    let africa_project = register_project(&client, &env, &owner);
    client.mock_all_auths().set_project_region(
        &africa_project,
        &owner,
        &Some(String::from_str(&env, "AFRICA")),
    );

    let asia_project = register_project(&client, &env, &owner);
    client.mock_all_auths().set_project_region(
        &asia_project,
        &owner,
        &Some(String::from_str(&env, "ASIA")),
    );

    let eu_project = register_project(&client, &env, &owner);
    client.mock_all_auths().set_project_region(
        &eu_project,
        &owner,
        &Some(String::from_str(&env, "EU")),
    );

    let all_projects = client.list_projects(&0, &10);
    let africa_only: Vec<u64> = all_projects
        .iter()
        .filter_map(|project| {
            if client.get_project_region(&project.id).unwrap_or(String::from_str(&env, ""))
                == String::from_str(&env, "AFRICA")
            {
                Some(project.id)
            } else {
                None
            }
        })
        .collect();

    assert_eq!(africa_only.len(), 1);
    assert_eq!(africa_only.get(0).copied(), Some(africa_project));
    assert!(all_projects.iter().any(|project| project.id == asia_project));
}

#[test]
fn test_owner_can_clear_region() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);
    let project_id = register_project(&client, &env, &owner);

    client.mock_all_auths().set_project_region(
        &project_id,
        &owner,
        &Some(String::from_str(&env, "EU")),
    );

    client
        .mock_all_auths()
        .set_project_region(&project_id, &owner, &None);

    let stored = client.get_project_region(&project_id);
    assert!(stored.is_none(), "Region should be cleared");
}

#[test]
fn test_non_owner_cannot_set_region() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);
    let non_owner = Address::generate(&env);
    let project_id = register_project(&client, &env, &owner);

    let result = client.mock_all_auths().try_set_project_region(
        &project_id,
        &non_owner,
        &Some(String::from_str(&env, "ASIA")),
    );

    assert!(
        result.is_err(),
        "Non-owner should not be able to set region"
    );
}

#[test]
fn test_integrity_hash_set_on_registration() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);
    let project_id = register_project(&client, &env, &owner);

    let hash = client.get_project_integrity_hash(&project_id);
    assert!(
        hash.is_some(),
        "Integrity hash should be set after registration"
    );
    assert_eq!(hash.unwrap().len(), 32, "SHA-256 hash must be 32 bytes");
}

#[test]
fn test_integrity_hash_changes_on_update() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);
    let project_id = register_project(&client, &env, &owner);

    let hash_before = client.get_project_integrity_hash(&project_id).unwrap();

    use crate::types::ProjectUpdateParams;
    client
        .mock_all_auths()
        .update_project(&ProjectUpdateParams {
            project_id,
            caller: owner.clone(),
            name: None,
            description: Some(String::from_str(
                &env,
                "Updated description changes the hash",
            )),
            website: None,
            license: None,
            logo_cid: None,
            metadata_cid: None,
            slug: None,
            category: None,
            tags: None,
            social_links: None,
            launch_timestamp: None,
            bounty_url: None,
            repository_url: None,
        });

    let hash_after = client.get_project_integrity_hash(&project_id).unwrap();
    assert_ne!(
        hash_before, hash_after,
        "Hash must change when metadata changes"
    );
}

#[test]
fn test_integrity_hash_accepts_description_longer_than_previous_scratch_buffer() {
    let env = Env::default();
    let description = "a".repeat(MAX_DESCRIPTION_LEN + 1);

    let hash = ProjectRegistry::compute_integrity_hash(
        &env,
        &String::from_str(&env, "Test-Project"),
        &String::from_str(&env, "test-project"),
        &String::from_str(&env, "DeFi"),
        &String::from_str(&env, &description),
    );

    assert_eq!(hash.len(), 32, "SHA-256 hash must be 32 bytes");
}
