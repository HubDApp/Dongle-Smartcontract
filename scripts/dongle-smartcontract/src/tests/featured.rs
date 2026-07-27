//! Tests for the featured projects admin flow (issue #126).

use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use soroban_sdk::{testutils::Address as _, Address, Env};

#[test]
fn test_set_featured_admin_only() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let non_admin = Address::generate(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Alpha");

    let result = client
        .mock_all_auths()
        .try_set_featured(&non_admin, &project_id, &true);

    assert_eq!(result, Err(Ok(ContractError::AdminOnly)));
}

#[test]
fn test_set_featured_project_not_found() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let result = client
        .mock_all_auths()
        .try_set_featured(&admin, &999u64, &true);

    assert_eq!(result, Err(Ok(ContractError::ProjectNotFound)));
}

#[test]
fn test_set_featured_and_list() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let id1 = create_test_project(&client, &owner, "Alpha");
    let id2 = create_test_project(&client, &owner, "Beta");
    let id3 = create_test_project(&client, &owner, "Gamma");

    client.mock_all_auths().set_featured(&admin, &id1, &true);
    client.mock_all_auths().set_featured(&admin, &id3, &true);

    let featured = client.list_featured_projects(&0, &10);
    assert_eq!(featured.len(), 2);
    assert_eq!(featured.get(0).unwrap().id, id1);
    assert_eq!(featured.get(1).unwrap().id, id3);

    // id2 was never featured
    let _ = id2; // suppress unused warning
}

#[test]
fn test_unfeature_project() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let id1 = create_test_project(&client, &owner, "Alpha");
    let id2 = create_test_project(&client, &owner, "Beta");

    client.mock_all_auths().set_featured(&admin, &id1, &true);
    client.mock_all_auths().set_featured(&admin, &id2, &true);
    client.mock_all_auths().set_featured(&admin, &id1, &false);

    let featured = client.list_featured_projects(&0, &10);
    assert_eq!(featured.len(), 1);
    assert_eq!(featured.get(0).unwrap().id, id2);
}

#[test]
fn test_set_featured_idempotent() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let id = create_test_project(&client, &owner, "Alpha");

    client.mock_all_auths().set_featured(&admin, &id, &true);
    client.mock_all_auths().set_featured(&admin, &id, &true); // duplicate – no-op

    let featured = client.list_featured_projects(&0, &10);
    assert_eq!(featured.len(), 1);
}

#[test]
fn test_list_featured_pagination() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    for i in 0..5u32 {
        let name = match i {
            0 => "Alpha",
            1 => "Beta",
            2 => "Gamma",
            3 => "Delta",
            _ => "Epsilon",
        };
        let id = create_test_project(&client, &owner, name);
        client.mock_all_auths().set_featured(&admin, &id, &true);
    }

    let page1 = client.list_featured_projects(&0, &3);
    let page2 = client.list_featured_projects(&3, &3);

    assert_eq!(page1.len(), 3);
    assert_eq!(page2.len(), 2);
}

#[test]
fn test_list_featured_empty() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);

    let featured = client.list_featured_projects(&0, &10);
    assert_eq!(featured.len(), 0);
}
