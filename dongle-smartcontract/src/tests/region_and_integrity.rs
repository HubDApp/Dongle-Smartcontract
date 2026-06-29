//! Tests for project region metadata (#238) and project integrity hash (#250).

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
fn test_owner_can_clear_region() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);
    let project_id = register_project(&client, &env, &owner);

    client
        .mock_all_auths()
        .set_project_region(&project_id, &owner, &Some(String::from_str(&env, "EU")));

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

    let result = client
        .mock_all_auths()
        .try_set_project_region(&project_id, &non_owner, &Some(String::from_str(&env, "ASIA")));

    assert!(result.is_err(), "Non-owner should not be able to set region");
}

#[test]
fn test_integrity_hash_set_on_registration() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);
    let project_id = register_project(&client, &env, &owner);

    let hash = client.get_project_integrity_hash(&project_id);
    assert!(hash.is_some(), "Integrity hash should be set after registration");
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
    client.mock_all_auths().update_project(&ProjectUpdateParams {
        project_id,
        caller: owner.clone(),
        name: None,
        description: Some(String::from_str(&env, "Updated description changes the hash")),
        website: None,
        license: None,
        logo_cid: None,
        metadata_cid: None,
        security_contact: None,
        security_contact_proof_cid: None,
    }).unwrap();

    let hash_after = client.get_project_integrity_hash(&project_id).unwrap();
    assert_ne!(hash_before, hash_after, "Hash must change when metadata changes");
}
