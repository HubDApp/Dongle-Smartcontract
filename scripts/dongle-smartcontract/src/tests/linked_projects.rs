//! Tests for linked project relationships (Issue #152)
use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use soroban_sdk::{testutils::Address as _, Address, Env};

#[test]
fn test_owner_can_link_projects() {
    let env = Env::default();
    let (client, _) = setup_contract(&env);
    let owner = Address::generate(&env);
    let id1 = create_test_project(&client.mock_all_auths(), &owner, "ProjectA");
    let id2 = create_test_project(&client.mock_all_auths(), &owner, "ProjectB");
    client.mock_all_auths().link_project(&id1, &owner, &id2);
    let links = client.get_linked_projects(&id1);
    assert_eq!(links.len(), 1);
    assert_eq!(links.get(0).unwrap(), id2);
}

#[test]
fn test_linked_project_must_exist() {
    let env = Env::default();
    let (client, _) = setup_contract(&env);
    let owner = Address::generate(&env);
    let id1 = create_test_project(&client.mock_all_auths(), &owner, "ProjectA");
    let result = client
        .mock_all_auths()
        .try_link_project(&id1, &owner, &9999u64);
    assert_eq!(result, Err(Ok(ContractError::AlreadyLinked)));
}

#[test]
fn test_duplicate_link_prevented() {
    let env = Env::default();
    let (client, _) = setup_contract(&env);
    let owner = Address::generate(&env);
    let id1 = create_test_project(&client.mock_all_auths(), &owner, "ProjectA");
    let id2 = create_test_project(&client.mock_all_auths(), &owner, "ProjectB");
    client.mock_all_auths().link_project(&id1, &owner, &id2);
    let result = client.mock_all_auths().try_link_project(&id1, &owner, &id2);
    assert_eq!(result, Err(Ok(ContractError::AlreadyLinked)));
}

#[test]
fn test_cannot_link_to_self() {
    let env = Env::default();
    let (client, _) = setup_contract(&env);
    let owner = Address::generate(&env);
    let id1 = create_test_project(&client.mock_all_auths(), &owner, "ProjectA");
    let result = client.mock_all_auths().try_link_project(&id1, &owner, &id1);
    assert_eq!(result, Err(Ok(ContractError::CannotLinkToSelf)));
}

#[test]
fn test_owner_can_unlink_projects() {
    let env = Env::default();
    let (client, _) = setup_contract(&env);
    let owner = Address::generate(&env);
    let id1 = create_test_project(&client.mock_all_auths(), &owner, "ProjectA");
    let id2 = create_test_project(&client.mock_all_auths(), &owner, "ProjectB");
    client.mock_all_auths().link_project(&id1, &owner, &id2);
    client.mock_all_auths().unlink_project(&id1, &owner, &id2);
    let links = client.get_linked_projects(&id1);
    assert_eq!(links.len(), 0);
}

#[test]
fn test_unlink_nonexistent_link_fails() {
    let env = Env::default();
    let (client, _) = setup_contract(&env);
    let owner = Address::generate(&env);
    let id1 = create_test_project(&client.mock_all_auths(), &owner, "ProjectA");
    let id2 = create_test_project(&client.mock_all_auths(), &owner, "ProjectB");
    let result = client
        .mock_all_auths()
        .try_unlink_project(&id1, &owner, &id2);
    assert_eq!(result, Err(Ok(ContractError::AlreadyLinked)));
}

#[test]
fn test_non_owner_cannot_link() {
    let env = Env::default();
    let (client, _) = setup_contract(&env);
    let owner = Address::generate(&env);
    let non_owner = Address::generate(&env);
    let id1 = create_test_project(&client.mock_all_auths(), &owner, "ProjectA");
    let id2 = create_test_project(&client.mock_all_auths(), &owner, "ProjectB");
    let result = client
        .mock_all_auths()
        .try_link_project(&id1, &non_owner, &id2);
    assert_eq!(result, Err(Ok(ContractError::Unauthorized)));
}

#[test]
fn test_non_owner_cannot_unlink() {
    let env = Env::default();
    let (client, _) = setup_contract(&env);
    let owner = Address::generate(&env);
    let non_owner = Address::generate(&env);
    let id1 = create_test_project(&client.mock_all_auths(), &owner, "ProjectA");
    let id2 = create_test_project(&client.mock_all_auths(), &owner, "ProjectB");
    client.mock_all_auths().link_project(&id1, &owner, &id2);
    let result = client
        .mock_all_auths()
        .try_unlink_project(&id1, &non_owner, &id2);
    assert_eq!(result, Err(Ok(ContractError::Unauthorized)));
}
