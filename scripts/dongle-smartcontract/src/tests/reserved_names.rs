//! Tests for reserved project names (Issue #231).
//!
//! Verifies that admin can manage a reserved name list,
//! registration and updates reject reserved names,
//! and events are emitted on changes.

use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::ProjectRegistrationParams;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

#[test]
fn test_add_and_get_reserved_names() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    // Initially empty
    let names = client.get_reserved_names();
    assert_eq!(names.len(), 0);

    // Add reserved names
    client.add_reserved_name(&admin, &String::from_str(&env, "admin"));
    client.add_reserved_name(&admin, &String::from_str(&env, "stellar"));
    client.add_reserved_name(&admin, &String::from_str(&env, "official"));

    let names = client.get_reserved_names();
    assert_eq!(names.len(), 3);
}

#[test]
fn test_reserved_name_blocks_registration() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    client.add_reserved_name(&admin, &String::from_str(&env, "admin"));

    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "admin"),
        slug: String::from_str(&env, "admin"),
        description: String::from_str(&env, "Test project"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
    };

    let result = client.try_register_project(&params);
    assert_eq!(result, Err(Ok(ContractError::ReservedName)));
}

#[test]
fn test_reserved_name_case_insensitive() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    client.add_reserved_name(&admin, &String::from_str(&env, "Stellar"));

    // Try registering with different case — should still be blocked
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "STELLAR"),
        slug: String::from_str(&env, "stellar"),
        description: String::from_str(&env, "Test project"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
    };

    let result = client.try_register_project(&params);
    assert_eq!(result, Err(Ok(ContractError::ReservedName)));
}

#[test]
fn test_remove_reserved_name() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    client.add_reserved_name(&admin, &String::from_str(&env, "admin"));
    assert!(client.is_name_reserved(&String::from_str(&env, "admin")));

    // Remove the reserved name
    client.remove_reserved_name(&admin, &String::from_str(&env, "admin"));
    assert!(!client.is_name_reserved(&String::from_str(&env, "admin")));

    // Registration should now succeed
    let _project_id = create_test_project(&client, &owner, "admin");
}

#[test]
fn test_reserved_name_blocks_update() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "MyProject");

    // Reserve a name
    client.add_reserved_name(&admin, &String::from_str(&env, "official"));

    // Try updating project name to reserved name
    let params = crate::types::ProjectUpdateParams {
        project_id,
        caller: owner.clone(),
        name: Some(String::from_str(&env, "official")),
        slug: None,
        description: None,
        category: None,
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
    };

    let result = client.try_update_project(&params);
    assert_eq!(result, Err(Ok(ContractError::ReservedName)));
}

#[test]
fn test_non_admin_cannot_add_reserved_name() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let non_admin = Address::generate(&env);

    let result = client.try_add_reserved_name(&non_admin, &String::from_str(&env, "test"));
    assert_eq!(result, Err(Ok(ContractError::AdminOnly)));
}

#[test]
fn test_add_duplicate_reserved_name_is_noop() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    client.add_reserved_name(&admin, &String::from_str(&env, "admin"));
    client.add_reserved_name(&admin, &String::from_str(&env, "admin"));

    // Should still be only one entry
    let names = client.get_reserved_names();
    assert_eq!(names.len(), 1);
}
