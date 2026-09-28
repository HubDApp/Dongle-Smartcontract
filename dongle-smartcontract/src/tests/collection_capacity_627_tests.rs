//! Targeted tests for Issue #627 — Collection Capacity Enforcement.

#![cfg(test)]

use crate::constants::MAX_PROJECTS_PER_COLLECTION;
use crate::errors::ContractError;
use crate::tests::fixtures::setup_contract;
use crate::types::ProjectRegistrationParams;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

fn create_test_project(
    client: &crate::DongleContractClient<'_>,
    env: &Env,
    owner: &Address,
    name: &str,
) -> u64 {
    let slug = name.to_lowercase().replace(' ', "-");
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(env, name),
        slug: String::from_str(env, &slug),
        description: String::from_str(env, "Collection capacity test project"),
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
    };
    client.register_project(&params)
}

#[test]
fn test_collection_capacity_enforcement_and_full_error() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let coll_id = client.create_collection(
        &admin,
        &String::from_str(&env, "Featured Collection"),
        &String::from_str(&env, "Collection description"),
    );

    // Add projects up to MAX_PROJECTS_PER_COLLECTION
    let mut project_ids = std::vec::Vec::new();
    for i in 0..MAX_PROJECTS_PER_COLLECTION {
        let proj_owner = Address::generate(&env);
        let name = alloc::format!("Proj{}", i);
        let pid = create_test_project(&client, &env, &proj_owner, &name);
        client.add_project_to_collection(&admin, &coll_id, &pid);
        project_ids.push(pid);
    }

    assert_eq!(
        client.get_collection_project_count(&coll_id),
        MAX_PROJECTS_PER_COLLECTION
    );

    // Attempting to add one more project must fail with CollectionFull
    let extra_pid = create_test_project(&client, &env, &owner, "ExtraProj");
    let err = client.try_add_project_to_collection(&admin, &coll_id, &extra_pid);
    assert_eq!(err, Err(Ok(ContractError::CollectionFull)));

    // Verify collection count remains unmutated at max capacity
    assert_eq!(
        client.get_collection_project_count(&coll_id),
        MAX_PROJECTS_PER_COLLECTION
    );
}

#[test]
fn test_collection_removal_frees_capacity_and_readdition() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let coll_id = client.create_collection(
        &admin,
        &String::from_str(&env, "Removable Collection"),
        &String::from_str(&env, "Collection description"),
    );

    let p1 = create_test_project(&client, &env, &owner, "ProjectOne");
    let p2 = create_test_project(&client, &env, &owner, "ProjectTwo");

    client.add_project_to_collection(&admin, &coll_id, &p1);
    assert_eq!(client.get_collection_project_count(&coll_id), 1);

    client.add_project_to_collection(&admin, &coll_id, &p2);
    assert_eq!(client.get_collection_project_count(&coll_id), 2);

    // Remove p1 frees capacity
    client.remove_project_from_collection(&admin, &coll_id, &p1);
    assert_eq!(client.get_collection_project_count(&coll_id), 1);

    // Re-adding p1 succeeds
    client.add_project_to_collection(&admin, &coll_id, &p1);
    assert_eq!(client.get_collection_project_count(&coll_id), 2);
}

#[test]
fn test_collection_capacity_one_boundary() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let coll_id = client.create_collection(
        &admin,
        &String::from_str(&env, "Single Item Collection"),
        &String::from_str(&env, "Collection description"),
    );

    let p1 = create_test_project(&client, &env, &owner, "SingleProject");
    client.add_project_to_collection(&admin, &coll_id, &p1);
    assert_eq!(client.get_collection_project_count(&coll_id), 1);

    // Removing and re-adding single project
    client.remove_project_from_collection(&admin, &coll_id, &p1);
    assert_eq!(client.get_collection_project_count(&coll_id), 0);

    client.add_project_to_collection(&admin, &coll_id, &p1);
    assert_eq!(client.get_collection_project_count(&coll_id), 1);
}
