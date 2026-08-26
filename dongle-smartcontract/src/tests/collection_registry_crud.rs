//! CRUD tests for collection_registry (issue #496).
//!
//! Covers: create_collection, update_collection, delete_collection,
//! add_project_to_collection (incl. duplicate check),
//! remove_project_from_collection, list_collections pagination, and
//! get_collection_project_count.

#![cfg(test)]

use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    Address, Env, String,
};

// ---------------------------------------------------------------------------
// create_collection
// ---------------------------------------------------------------------------

#[test]
fn test_create_collection_stores_all_fields() {
    let env = Env::default();
    env.ledger().with_mut(|li| li.timestamp = 5_000);
    let (client, admin) = setup_contract(&env);

    let id = client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "DeFi Gems"),
        &String::from_str(&env, "Top DeFi protocols on Stellar"),
    );

    let collection = client.get_collection(&id).unwrap();
    assert_eq!(collection.id, id);
    assert_eq!(collection.name, String::from_str(&env, "DeFi Gems"));
    assert_eq!(
        collection.description,
        String::from_str(&env, "Top DeFi protocols on Stellar"),
    );
    assert_eq!(collection.created_at, 5_000);
    assert_eq!(collection.updated_at, 5_000);
}

#[test]
fn test_create_collection_requires_admin() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let non_admin = Address::generate(&env);

    let result = client.mock_all_auths().try_create_collection(
        &non_admin,
        &String::from_str(&env, "DeFi"),
        &String::from_str(&env, "desc"),
    );
    assert_eq!(result, Err(Ok(ContractError::AdminOnly)));
}

#[test]
fn test_create_collection_empty_name_rejected() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let result = client.mock_all_auths().try_create_collection(
        &admin,
        &String::from_str(&env, ""),
        &String::from_str(&env, "desc"),
    );
    assert_eq!(result, Err(Ok(ContractError::InvalidProjectData)));
}

#[test]
fn test_create_collection_name_too_long_rejected() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let long_name = "x".repeat(102);
    let result = client.mock_all_auths().try_create_collection(
        &admin,
        &String::from_str(&env, &long_name),
        &String::from_str(&env, "desc"),
    );
    assert_eq!(result, Err(Ok(ContractError::InvalidProjectName)));
}

#[test]
fn test_create_collection_duplicate_name_rejected() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "DeFi"),
        &String::from_str(&env, "desc"),
    );

    let result = client.mock_all_auths().try_create_collection(
        &admin,
        &String::from_str(&env, "DeFi"),
        &String::from_str(&env, "another desc"),
    );
    assert_eq!(result, Err(Ok(ContractError::CollectionExists)));
}

#[test]
fn test_create_collection_increments_count() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    assert_eq!(client.get_collection_count(), 0);

    client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "Alpha"),
        &String::from_str(&env, "desc"),
    );
    assert_eq!(client.get_collection_count(), 1);

    client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "Beta"),
        &String::from_str(&env, "desc"),
    );
    assert_eq!(client.get_collection_count(), 2);
}

// ---------------------------------------------------------------------------
// update_collection
// ---------------------------------------------------------------------------

#[test]
fn test_update_collection_changes_name_and_description() {
    let env = Env::default();
    env.ledger().with_mut(|li| li.timestamp = 1_000);
    let (client, admin) = setup_contract(&env);

    let id = client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "Old Name"),
        &String::from_str(&env, "Old desc"),
    );

    env.ledger().with_mut(|li| li.timestamp = 2_000);
    client.mock_all_auths().update_collection(
        &admin,
        &id,
        &String::from_str(&env, "New Name"),
        &String::from_str(&env, "New desc"),
    );

    let collection = client.get_collection(&id).unwrap();
    assert_eq!(collection.name, String::from_str(&env, "New Name"));
    assert_eq!(collection.description, String::from_str(&env, "New desc"));
    assert_eq!(collection.updated_at, 2_000);
    // created_at must not change
    assert_eq!(collection.created_at, 1_000);
}

#[test]
fn test_update_collection_requires_admin() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let non_admin = Address::generate(&env);

    let id = client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "Name"),
        &String::from_str(&env, "desc"),
    );

    let result = client.mock_all_auths().try_update_collection(
        &non_admin,
        &id,
        &String::from_str(&env, "New Name"),
        &String::from_str(&env, "New desc"),
    );
    assert_eq!(result, Err(Ok(ContractError::AdminOnly)));
}

#[test]
fn test_update_nonexistent_collection_rejected() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let result = client.mock_all_auths().try_update_collection(
        &admin,
        &999u64,
        &String::from_str(&env, "Name"),
        &String::from_str(&env, "desc"),
    );
    assert_eq!(result, Err(Ok(ContractError::CollectionNotFound)));
}

#[test]
fn test_update_collection_duplicate_name_rejected() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "Alpha"),
        &String::from_str(&env, "desc"),
    );
    let id2 = client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "Beta"),
        &String::from_str(&env, "desc"),
    );

    let result = client.mock_all_auths().try_update_collection(
        &admin,
        &id2,
        &String::from_str(&env, "Alpha"), // conflicts with existing
        &String::from_str(&env, "desc"),
    );
    assert_eq!(result, Err(Ok(ContractError::CollectionExists)));
}

#[test]
fn test_update_collection_same_name_allowed() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let id = client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "Alpha"),
        &String::from_str(&env, "desc"),
    );

    // Updating with the same name should not return CollectionExists
    let result = client.mock_all_auths().try_update_collection(
        &admin,
        &id,
        &String::from_str(&env, "Alpha"),
        &String::from_str(&env, "new desc"),
    );
    assert!(result.is_ok(), "updating with the same name should succeed");
}

// ---------------------------------------------------------------------------
// delete_collection
// ---------------------------------------------------------------------------

#[test]
fn test_delete_collection_removes_it() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let id = client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "ToDelete"),
        &String::from_str(&env, "desc"),
    );

    client.mock_all_auths().delete_collection(&admin, &id);

    assert!(client.get_collection(&id).is_none());
    assert_eq!(client.get_collection_count(), 0);
}

#[test]
fn test_delete_collection_requires_admin() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let non_admin = Address::generate(&env);

    let id = client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "ToDelete"),
        &String::from_str(&env, "desc"),
    );

    let result = client
        .mock_all_auths()
        .try_delete_collection(&non_admin, &id);
    assert_eq!(result, Err(Ok(ContractError::AdminOnly)));
}

#[test]
fn test_delete_nonexistent_collection_rejected() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let result = client
        .mock_all_auths()
        .try_delete_collection(&admin, &999u64);
    assert_eq!(result, Err(Ok(ContractError::CollectionNotFound)));
}

#[test]
fn test_delete_collection_does_not_remove_projects() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "ProjectA");
    let coll_id = client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "ToDelete"),
        &String::from_str(&env, "desc"),
    );

    client
        .mock_all_auths()
        .add_project_to_collection(&admin, &coll_id, &project_id);

    client.mock_all_auths().delete_collection(&admin, &coll_id);

    // Collection is gone
    assert!(client.get_collection(&coll_id).is_none());
    // But the project still exists
    assert!(client.get_project(&project_id).is_some());
}

// ---------------------------------------------------------------------------
// add_project_to_collection (including duplicate check)
// ---------------------------------------------------------------------------

#[test]
fn test_add_project_to_collection_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "ProjectA");
    let coll_id = client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "Wallets"),
        &String::from_str(&env, "desc"),
    );

    client
        .mock_all_auths()
        .add_project_to_collection(&admin, &coll_id, &project_id);

    assert_eq!(client.get_collection_project_count(&coll_id), 1);
    let ids = client.list_collection_projects(&coll_id, &0, &10);
    assert_eq!(ids.len(), 1);
    assert_eq!(ids.get(0).unwrap(), project_id);
}

#[test]
fn test_add_project_to_collection_duplicate_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "ProjectA");
    let coll_id = client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "Wallets"),
        &String::from_str(&env, "desc"),
    );

    client
        .mock_all_auths()
        .add_project_to_collection(&admin, &coll_id, &project_id);

    // Adding the same project a second time must fail
    let result =
        client
            .mock_all_auths()
            .try_add_project_to_collection(&admin, &coll_id, &project_id);
    assert_eq!(result, Err(Ok(ContractError::AlreadyInCollection)));

    // Count must still be 1
    assert_eq!(client.get_collection_project_count(&coll_id), 1);
}

#[test]
fn test_add_nonexistent_project_rejected() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let coll_id = client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "Wallets"),
        &String::from_str(&env, "desc"),
    );

    let result = client
        .mock_all_auths()
        .try_add_project_to_collection(&admin, &coll_id, &999u64);
    assert_eq!(result, Err(Ok(ContractError::ProjectNotFound)));
}

#[test]
fn test_add_project_to_nonexistent_collection_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "ProjectA");

    let result =
        client
            .mock_all_auths()
            .try_add_project_to_collection(&_admin, &999u64, &project_id);
    assert_eq!(result, Err(Ok(ContractError::CollectionNotFound)));
}

// ---------------------------------------------------------------------------
// remove_project_from_collection
// ---------------------------------------------------------------------------

#[test]
fn test_remove_project_from_collection_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let p1 = create_test_project(&client, &owner, "P1");
    let p2 = create_test_project(&client, &owner, "P2");
    let coll_id = client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "Top"),
        &String::from_str(&env, "desc"),
    );

    client
        .mock_all_auths()
        .add_project_to_collection(&admin, &coll_id, &p1);
    client
        .mock_all_auths()
        .add_project_to_collection(&admin, &coll_id, &p2);
    assert_eq!(client.get_collection_project_count(&coll_id), 2);

    client
        .mock_all_auths()
        .remove_project_from_collection(&admin, &coll_id, &p1);
    assert_eq!(client.get_collection_project_count(&coll_id), 1);

    let ids = client.list_collection_projects(&coll_id, &0, &10);
    assert_eq!(ids.len(), 1);
    assert_eq!(ids.get(0).unwrap(), p2);
}

#[test]
fn test_remove_project_not_in_collection_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "P1");
    let coll_id = client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "Top"),
        &String::from_str(&env, "desc"),
    );

    let result =
        client
            .mock_all_auths()
            .try_remove_project_from_collection(&admin, &coll_id, &project_id);
    assert_eq!(result, Err(Ok(ContractError::AlreadyInCollection)));
}

#[test]
fn test_remove_project_from_nonexistent_collection_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "P1");

    let result =
        client
            .mock_all_auths()
            .try_remove_project_from_collection(&admin, &999u64, &project_id);
    assert_eq!(result, Err(Ok(ContractError::CollectionNotFound)));
}

// ---------------------------------------------------------------------------
// list_collections pagination
// ---------------------------------------------------------------------------

#[test]
fn test_list_collections_pagination_full_coverage() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let names = ["A", "B", "C", "D", "E", "F"];
    for name in &names {
        client.mock_all_auths().create_collection(
            &admin,
            &String::from_str(&env, name),
            &String::from_str(&env, "desc"),
        );
    }

    // Page 1
    let page1 = client.list_collections(&0, &3);
    assert_eq!(page1.len(), 3);
    assert_eq!(page1.get(0).unwrap().name, String::from_str(&env, "A"));
    assert_eq!(page1.get(1).unwrap().name, String::from_str(&env, "B"));
    assert_eq!(page1.get(2).unwrap().name, String::from_str(&env, "C"));

    // Page 2
    let page2 = client.list_collections(&3, &3);
    assert_eq!(page2.len(), 3);
    assert_eq!(page2.get(0).unwrap().name, String::from_str(&env, "D"));
    assert_eq!(page2.get(1).unwrap().name, String::from_str(&env, "E"));
    assert_eq!(page2.get(2).unwrap().name, String::from_str(&env, "F"));

    // Offset past end → empty
    let page3 = client.list_collections(&6, &3);
    assert_eq!(page3.len(), 0);

    // Limit larger than remaining → returns what's left
    let page4 = client.list_collections(&4, &10);
    assert_eq!(page4.len(), 2);
}

#[test]
fn test_list_collections_empty_returns_empty_vec() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);

    let result = client.list_collections(&0, &10);
    assert_eq!(result.len(), 0);
}

// ---------------------------------------------------------------------------
// get_collection_project_count
// ---------------------------------------------------------------------------

#[test]
fn test_get_collection_project_count_tracks_adds_and_removes() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let coll_id = client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "Tracker"),
        &String::from_str(&env, "desc"),
    );
    assert_eq!(client.get_collection_project_count(&coll_id), 0);

    let p1 = create_test_project(&client, &owner, "Proj1");
    let p2 = create_test_project(&client, &owner, "Proj2");
    let p3 = create_test_project(&client, &owner, "Proj3");

    client
        .mock_all_auths()
        .add_project_to_collection(&admin, &coll_id, &p1);
    assert_eq!(client.get_collection_project_count(&coll_id), 1);

    client
        .mock_all_auths()
        .add_project_to_collection(&admin, &coll_id, &p2);
    client
        .mock_all_auths()
        .add_project_to_collection(&admin, &coll_id, &p3);
    assert_eq!(client.get_collection_project_count(&coll_id), 3);

    client
        .mock_all_auths()
        .remove_project_from_collection(&admin, &coll_id, &p2);
    assert_eq!(client.get_collection_project_count(&coll_id), 2);

    client
        .mock_all_auths()
        .remove_project_from_collection(&admin, &coll_id, &p1);
    client
        .mock_all_auths()
        .remove_project_from_collection(&admin, &coll_id, &p3);
    assert_eq!(client.get_collection_project_count(&coll_id), 0);
}
