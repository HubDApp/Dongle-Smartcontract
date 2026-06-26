//! Tests for project launch timestamp feature (Issue #156)
#![allow(dead_code)]
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::ProjectRegistrationParams;
use crate::errors::ContractError;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

#[test]
fn test_register_project_with_launch_timestamp() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let ts = 2000000000u64;
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "LaunchProject"),
        slug: String::from_str(&env, "launch-project"),
        description: String::from_str(&env, "A project with a launch date"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: Some(ts),
    };
    let project_id = client.mock_all_auths().register_project(&params);
    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.launch_timestamp, Some(ts));
}

#[test]
fn test_register_project_without_launch_timestamp() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client.mock_all_auths(), &owner, "NoLaunch");
    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.launch_timestamp, None);
}

#[test]
fn test_owner_can_update_launch_timestamp() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client.mock_all_auths(), &owner, "UpdateLaunch");
    let ts = 2000000000u64;
    let project = client
        .mock_all_auths()
        .update_launch_timestamp(&project_id, &owner, &Some(ts));
    assert_eq!(project.launch_timestamp, Some(ts));
}

#[test]
fn test_owner_can_clear_launch_timestamp() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client.mock_all_auths(), &owner, "ClearLaunch");
    client
        .mock_all_auths()
        .update_launch_timestamp(&project_id, &owner, &Some(2000000000u64));
    let project = client
        .mock_all_auths()
        .update_launch_timestamp(&project_id, &owner, &None);
    assert_eq!(project.launch_timestamp, None);
}

#[test]
fn test_non_owner_cannot_update_launch_timestamp() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let non_owner = Address::generate(&env);
    let project_id = create_test_project(&client.mock_all_auths(), &owner, "AuthLaunch");
    let result = client
        .mock_all_auths()
        .try_update_launch_timestamp(&project_id, &non_owner, &Some(2000000000u64));
    assert_eq!(result, Err(Ok(ContractError::Unauthorized)));
}
