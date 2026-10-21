//! Tests for project visibility modes (#874).

use crate::errors::ContractError;
use crate::types::ProjectVisibility;
use soroban_sdk::{testutils::Address as _, Address, Env};

use super::fixtures::{create_test_project, setup_contract};

#[test]
fn test_projects_default_to_public() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let id = create_test_project(&client, &owner, "Default Public");

    let project = client.get_project(&id).unwrap();
    assert_eq!(project.visibility, ProjectVisibility::Public);
}

#[test]
fn test_private_project_hidden_from_anonymous_and_listings() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let stranger = Address::generate(&env);
    let id = create_test_project(&client, &owner, "Private One");

    client
        .mock_all_auths()
        .set_project_visibility(&id, &owner, &ProjectVisibility::Private);

    assert!(client.get_project(&id).is_none());
    assert_eq!(client.list_projects(&1, &10).len(), 0);
    let env_auth = client.mock_all_auths();
    assert!(env_auth.get_project_as_viewer(&id, &owner).is_some());
    assert!(env_auth.get_project_as_viewer(&id, &admin).is_some());
    assert!(env_auth.get_project_as_viewer(&id, &stranger).is_none());
}

#[test]
fn test_only_owner_can_change_visibility() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let stranger = Address::generate(&env);
    let id = create_test_project(&client, &owner, "Owner Only");

    let result = client.mock_all_auths().try_set_project_visibility(
        &id,
        &stranger,
        &ProjectVisibility::Restricted,
    );
    assert_eq!(result, Err(Ok(ContractError::Unauthorized)));
}
