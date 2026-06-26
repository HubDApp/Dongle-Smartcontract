//! Tests for project archival feature (issue #121).

use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use soroban_sdk::{testutils::Address as _, Address, Env};

#[test]
fn test_owner_can_archive_project() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "MyProject");
    client.archive_project(&project_id, &owner);

    let project = client.get_project(&project_id).unwrap();
    assert!(project.archived);
}

#[test]
fn test_unauthorized_cannot_archive_project() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let stranger = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "MyProject");

    let result = client.try_archive_project(&project_id, &stranger);
    assert_eq!(result, Err(Ok(ContractError::Unauthorized)));
}

#[test]
fn test_admin_can_force_archive_project() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "MyProject");
    client.archive_project(&project_id, &admin);

    let project = client.get_project(&project_id).unwrap();
    assert!(project.archived);
}

#[test]
fn test_archived_project_excluded_from_list_projects() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let id1 = create_test_project(&client, &owner, "ProjectA");
    let id2 = create_test_project(&client, &owner, "ProjectB");

    client.archive_project(&id1, &owner);

    let projects = client.list_projects(&0, &10);
    assert_eq!(projects.len(), 1);
    assert_eq!(projects.get(0).unwrap().id, id2);
}

#[test]
fn test_archived_project_excluded_from_list_by_category() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let id1 = create_test_project(&client, &owner, "ProjectA");
    let id2 = create_test_project(&client, &owner, "ProjectB");

    client.archive_project(&id1, &owner);

    let category = soroban_sdk::String::from_str(&env, "DeFi");
    let projects = client.list_projects_by_category(&category, &0, &10);
    assert_eq!(projects.len(), 1);
    assert_eq!(projects.get(0).unwrap().id, id2);
}

#[test]
fn test_archive_nonexistent_project_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let caller = Address::generate(&env);

    let result = client.try_archive_project(&999, &caller);
    assert_eq!(result, Err(Ok(ContractError::ProjectNotFound)));
}
