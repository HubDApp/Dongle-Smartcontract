//! Tests for typed error handling - ensuring no panics occur

use crate::errors::ContractError;
use crate::tests::fixtures::create_test_env;
use crate::types::{ProjectRegistrationParams, ProjectUpdateParams};
use crate::DongleContract;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

// ── Project Registration Error Tests ──

#[test]
fn test_register_project_empty_name_returns_error() {
    let env = Env::default();
    env.mock_all_auths();

    let owner = Address::generate(&env);
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, ""),
        description: String::from_str(&env, "Valid description"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };

    let result = DongleContract::register_project(env.clone(), params);
    assert_eq!(result, Err(ContractError::InvalidProjectName));
}

#[test]
fn test_register_project_empty_description_returns_error() {
    let env = Env::default();
    env.mock_all_auths();

    let owner = Address::generate(&env);
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "ValidName"),
        description: String::from_str(&env, ""),
        category: String::from_str(&env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };

    let result = DongleContract::register_project(env.clone(), params);
    assert_eq!(result, Err(ContractError::InvalidProjectDescription));
}

#[test]
fn test_register_project_empty_category_returns_error() {
    let env = Env::default();
    env.mock_all_auths();

    let owner = Address::generate(&env);
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "ValidName"),
        description: String::from_str(&env, "Valid description"),
        category: String::from_str(&env, ""),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };

    let result = DongleContract::register_project(env.clone(), params);
    assert_eq!(result, Err(ContractError::InvalidProjectCategory));
}

#[test]
fn test_register_project_duplicate_name_returns_error() {
    let env = Env::default();
    env.mock_all_auths();

    let owner = Address::generate(&env);
    let params1 = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "UniqueProject"),
        description: String::from_str(&env, "First project"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };

    // First registration should succeed
    let result1 = DongleContract::register_project(env.clone(), params1);
    assert!(result1.is_ok());

    // Second registration with same name should fail
    let params2 = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "UniqueProject"),
        description: String::from_str(&env, "Second project"),
        category: String::from_str(&env, "NFT"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };

    let result2 = DongleContract::register_project(env.clone(), params2);
    assert_eq!(result2, Err(ContractError::ProjectAlreadyExists));
}

#[test]
fn test_register_project_valid_inputs_succeeds() {
    let env = Env::default();
    env.mock_all_auths();

    let owner = Address::generate(&env);
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "ValidProject"),
        description: String::from_str(&env, "A valid project description"),
        category: String::from_str(&env, "DeFi"),
        website: Some(String::from_str(&env, "https://example.com")),
        logo_cid: Some(String::from_str(&env, "QmValidCID123")),
        metadata_cid: Some(String::from_str(&env, "QmValidMetadata456")),
    };

    let result = DongleContract::register_project(env.clone(), params);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1);
}

// ── Project Update Error Tests ──

#[test]
fn test_update_project_not_found_returns_error() {
    let env = Env::default();
    env.mock_all_auths();

    let caller = Address::generate(&env);
    let params = ProjectUpdateParams {
        project_id: 999,
        caller: caller.clone(),
        name: Some(String::from_str(&env, "NewName")),
        description: None,
        category: None,
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };

    let result = DongleContract::update_project(env.clone(), params);
    assert_eq!(result, Err(ContractError::ProjectNotFound));
}

#[test]
fn test_update_project_unauthorized_returns_error() {
    let env = Env::default();
    env.mock_all_auths();

    let owner = Address::generate(&env);
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "TestProject"),
        description: String::from_str(&env, "Description"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };

    let project_id = DongleContract::register_project(env.clone(), params).unwrap();

    // Try to update with different caller
    let unauthorized_caller = Address::generate(&env);
    let update_params = ProjectUpdateParams {
        project_id,
        caller: unauthorized_caller.clone(),
        name: Some(String::from_str(&env, "HackedName")),
        description: None,
        category: None,
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };

    let result = DongleContract::update_project(env.clone(), update_params);
    assert_eq!(result, Err(ContractError::Unauthorized));
}

#[test]
fn test_update_project_empty_name_returns_error() {
    let env = Env::default();
    env.mock_all_auths();

    let owner = Address::generate(&env);
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "TestProject"),
        description: String::from_str(&env, "Description"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };

    let project_id = DongleContract::register_project(env.clone(), params).unwrap();

    let update_params = ProjectUpdateParams {
        project_id,
        caller: owner.clone(),
        name: Some(String::from_str(&env, "")),
        description: None,
        category: None,
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };

    let result = DongleContract::update_project(env.clone(), update_params);
    assert_eq!(result, Err(ContractError::InvalidProjectName));
}

#[test]
fn test_update_project_empty_description_returns_error() {
    let env = Env::default();
    env.mock_all_auths();

    let owner = Address::generate(&env);
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "TestProject"),
        description: String::from_str(&env, "Description"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };

    let project_id = DongleContract::register_project(env.clone(), params).unwrap();

    let update_params = ProjectUpdateParams {
        project_id,
        caller: owner.clone(),
        name: None,
        description: Some(String::from_str(&env, "")),
        category: None,
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };

    let result = DongleContract::update_project(env.clone(), update_params);
    assert_eq!(result, Err(ContractError::InvalidProjectDescription));
}

#[test]
fn test_update_project_empty_category_returns_error() {
    let env = Env::default();
    env.mock_all_auths();

    let owner = Address::generate(&env);
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "TestProject"),
        description: String::from_str(&env, "Description"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };

    let project_id = DongleContract::register_project(env.clone(), params).unwrap();

    let update_params = ProjectUpdateParams {
        project_id,
        caller: owner.clone(),
        name: None,
        description: None,
        category: Some(String::from_str(&env, "")),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };

    let result = DongleContract::update_project(env.clone(), update_params);
    assert_eq!(result, Err(ContractError::InvalidProjectCategory));
}

#[test]
fn test_update_project_valid_inputs_succeeds() {
    let env = Env::default();
    env.mock_all_auths();

    let owner = Address::generate(&env);
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "TestProject"),
        description: String::from_str(&env, "Original description"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };

    let project_id = DongleContract::register_project(env.clone(), params).unwrap();

    let update_params = ProjectUpdateParams {
        project_id,
        caller: owner.clone(),
        name: Some(String::from_str(&env, "UpdatedProject")),
        description: Some(String::from_str(&env, "Updated description")),
        category: Some(String::from_str(&env, "NFT")),
        website: Some(Some(String::from_str(&env, "https://updated.com"))),
        logo_cid: None,
        metadata_cid: None,
    };

    let result = DongleContract::update_project(env.clone(), update_params);
    assert!(result.is_ok());

    let updated_project = result.unwrap();
    assert_eq!(updated_project.name, String::from_str(&env, "UpdatedProject"));
    assert_eq!(
        updated_project.description,
        String::from_str(&env, "Updated description")
    );
    assert_eq!(updated_project.category, String::from_str(&env, "NFT"));
}

// ── No Panic Tests ──

#[test]
fn test_no_panic_on_invalid_inputs() {
    let env = Env::default();
    env.mock_all_auths();

    let owner = Address::generate(&env);

    // Test all invalid input combinations - none should panic
    let test_cases = vec![
        ("", "desc", "cat", ContractError::InvalidProjectName),
        ("name", "", "cat", ContractError::InvalidProjectDescription),
        ("name", "desc", "", ContractError::InvalidProjectCategory),
    ];

    for (name, desc, cat, expected_error) in test_cases {
        let params = ProjectRegistrationParams {
            owner: owner.clone(),
            name: String::from_str(&env, name),
            description: String::from_str(&env, desc),
            category: String::from_str(&env, cat),
            website: None,
            logo_cid: None,
            metadata_cid: None,
        };

        let result = DongleContract::register_project(env.clone(), params);
        assert_eq!(result, Err(expected_error));
    }
}

#[test]
fn test_multiple_operations_no_panic() {
    let env = Env::default();
    env.mock_all_auths();

    let owner = Address::generate(&env);

    // Register valid project
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "Project1"),
        description: String::from_str(&env, "Description"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };
    let project_id = DongleContract::register_project(env.clone(), params).unwrap();

    // Try invalid updates - should return errors, not panic
    let invalid_updates = vec![
        (Some(String::from_str(&env, "")), None, None),
        (None, Some(String::from_str(&env, "")), None),
        (None, None, Some(String::from_str(&env, ""))),
    ];

    for (name, desc, cat) in invalid_updates {
        let update_params = ProjectUpdateParams {
            project_id,
            caller: owner.clone(),
            name,
            description: desc,
            category: cat,
            website: None,
            logo_cid: None,
            metadata_cid: None,
        };

        let result = DongleContract::update_project(env.clone(), update_params);
        assert!(result.is_err());
    }
}
