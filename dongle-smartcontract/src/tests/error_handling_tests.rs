//! Tests for typed error handling - ensuring no panics occur

use crate::errors::ContractError;
use crate::types::{ProjectRegistrationParams, ProjectUpdateParams};
use crate::DongleContract;
use crate::DongleContractClient;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

fn setup(env: &Env) -> (DongleContractClient<'_>, Address) {
    let contract_id = env.register(DongleContract, ());
    let client = DongleContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.initialize(&admin);
    (client, admin)
}

// ── Project Registration Error Tests ──

#[test]
fn test_register_project_empty_name_returns_error() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
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

    let result = client.try_register_project(&params);
    assert_eq!(result, Err(Ok(ContractError::InvalidProjectData)));
}

#[test]
fn test_register_project_empty_description_returns_error() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
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

    let result = client.try_register_project(&params);
    assert_eq!(result, Err(Ok(ContractError::InvalidProjectDescription)));
}

#[test]
fn test_register_project_empty_category_returns_error() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
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

    let result = client.try_register_project(&params);
    assert_eq!(result, Err(Ok(ContractError::InvalidProjectCategory)));
}

#[test]
fn test_register_project_duplicate_name_returns_error() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
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
    let result1 = client.try_register_project(&params1);
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

    let result2 = client.try_register_project(&params2);
    assert_eq!(result2, Err(Ok(ContractError::ProjectAlreadyExists)));
}

#[test]
fn test_register_project_valid_inputs_succeeds() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    env.mock_all_auths();

    let owner = Address::generate(&env);
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "ValidProject"),
        description: String::from_str(&env, "A valid project description"),
        category: String::from_str(&env, "DeFi"),
        website: Some(String::from_str(&env, "https://example.com")),
        logo_cid: Some(String::from_str(
            &env,
            "QmYyQSo1c1Ym7orWxLYvCrM2Wu3m39mY5A2zR3EebnXZ7G",
        )),
        metadata_cid: Some(String::from_str(
            &env,
            "QmYyQSo1c1Ym7orWxLYvCrM2Wu3m39mY5A2zR3EebnXZ7G",
        )),
    };

    let result = client.try_register_project(&params);
    assert!(result.is_ok());
    let project_id = result.unwrap().unwrap();
    assert_eq!(project_id, 1);
}

// ── Project Update Error Tests ──

#[test]
fn test_update_project_not_found_returns_error() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
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

    let result = client.try_update_project(&params);
    assert_eq!(result, Err(Ok(ContractError::ProjectNotFound)));
}

#[test]
fn test_update_project_unauthorized_returns_error() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
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

    let project_id = client.register_project(&params);

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

    let result = client.try_update_project(&update_params);
    assert_eq!(result, Err(Ok(ContractError::Unauthorized)));
}

#[test]
fn test_update_project_empty_name_returns_error() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
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

    let project_id = client.register_project(&params);

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

    let result = client.try_update_project(&update_params);
    assert_eq!(result, Err(Ok(ContractError::InvalidProjectName)));
}

#[test]
fn test_update_project_empty_description_returns_error() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
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

    let project_id = client.register_project(&params);

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

    let result = client.try_update_project(&update_params);
    assert_eq!(result, Err(Ok(ContractError::InvalidProjectDescription)));
}

#[test]
fn test_update_project_empty_category_returns_error() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
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

    let project_id = client.register_project(&params);

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

    let result = client.try_update_project(&update_params);
    assert_eq!(result, Err(Ok(ContractError::InvalidProjectCategory)));
}

#[test]
fn test_update_project_valid_inputs_succeeds() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
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

    let project_id = client.register_project(&params);

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

    let updated_project = client.update_project(&update_params);
    assert_eq!(
        updated_project.name,
        String::from_str(&env, "UpdatedProject")
    );
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
    let (client, _admin) = setup(&env);
    env.mock_all_auths();

    let owner = Address::generate(&env);

    // Test all invalid input combinations - none should panic
    let test_cases = [
        ("", "desc", "cat", ContractError::InvalidProjectData),
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

        let result = client.try_register_project(&params);
        assert_eq!(result, Err(Ok(expected_error)));
    }
}

#[test]
fn test_multiple_operations_no_panic() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
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
    let project_id = client.register_project(&params);

    // Try invalid updates - should return errors, not panic
    let invalid_updates = [
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

        let result = client.try_update_project(&update_params);
        assert!(result.is_err());
    }
}

// ── Project Limit Tests ──

#[test]
fn test_register_project_exceeds_max_limit() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    env.mock_all_auths();

    let owner = Address::generate(&env);

    // Register MAX_PROJECTS_PER_USER (50) projects with unique names
    let project_names = [
        "Proj00", "Proj01", "Proj02", "Proj03", "Proj04", "Proj05", "Proj06", "Proj07", "Proj08",
        "Proj09", "Proj10", "Proj11", "Proj12", "Proj13", "Proj14", "Proj15", "Proj16", "Proj17",
        "Proj18", "Proj19", "Proj20", "Proj21", "Proj22", "Proj23", "Proj24", "Proj25", "Proj26",
        "Proj27", "Proj28", "Proj29", "Proj30", "Proj31", "Proj32", "Proj33", "Proj34", "Proj35",
        "Proj36", "Proj37", "Proj38", "Proj39", "Proj40", "Proj41", "Proj42", "Proj43", "Proj44",
        "Proj45", "Proj46", "Proj47", "Proj48", "Proj49",
    ];

    for name in &project_names {
        let params = ProjectRegistrationParams {
            owner: owner.clone(),
            name: String::from_str(&env, name),
            description: String::from_str(&env, "Valid description"),
            category: String::from_str(&env, "DeFi"),
            website: None,
            logo_cid: None,
            metadata_cid: None,
        };
        let result = client.try_register_project(&params);
        assert!(result.is_ok());
    }

    // Try to register the 51st project - should fail
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "Proj50"),
        description: String::from_str(&env, "Valid description"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };

    let result = client.try_register_project(&params);
    assert_eq!(result, Err(Ok(ContractError::MaxProjectsExceeded)));
}

#[test]
fn test_register_project_at_max_limit_succeeds() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    env.mock_all_auths();

    let owner = Address::generate(&env);

    // Register exactly MAX_PROJECTS_PER_USER (50) projects
    let project_names = [
        "ProjA00", "ProjA01", "ProjA02", "ProjA03", "ProjA04", "ProjA05", "ProjA06", "ProjA07",
        "ProjA08", "ProjA09", "ProjA10", "ProjA11", "ProjA12", "ProjA13", "ProjA14", "ProjA15",
        "ProjA16", "ProjA17", "ProjA18", "ProjA19", "ProjA20", "ProjA21", "ProjA22", "ProjA23",
        "ProjA24", "ProjA25", "ProjA26", "ProjA27", "ProjA28", "ProjA29", "ProjA30", "ProjA31",
        "ProjA32", "ProjA33", "ProjA34", "ProjA35", "ProjA36", "ProjA37", "ProjA38", "ProjA39",
        "ProjA40", "ProjA41", "ProjA42", "ProjA43", "ProjA44", "ProjA45", "ProjA46", "ProjA47",
        "ProjA48", "ProjA49",
    ];

    for name in &project_names {
        let params = ProjectRegistrationParams {
            owner: owner.clone(),
            name: String::from_str(&env, name),
            description: String::from_str(&env, "Valid description"),
            category: String::from_str(&env, "DeFi"),
            website: None,
            logo_cid: None,
            metadata_cid: None,
        };
        let result = client.try_register_project(&params);
        assert!(result.is_ok());
    }

    // Verify we have exactly 50 projects
    let project_count = client.get_owner_project_count(&owner);
    assert_eq!(project_count, 50);
}

#[test]
fn test_different_owners_independent_limits() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    env.mock_all_auths();

    let owner1 = Address::generate(&env);
    let owner2 = Address::generate(&env);

    // Owner 1 registers 5 projects (simplified for test speed)
    for i in 0..5 {
        let name = match i {
            0 => "Owner1Proj0",
            1 => "Owner1Proj1",
            2 => "Owner1Proj2",
            3 => "Owner1Proj3",
            _ => "Owner1Proj4",
        };

        let params = ProjectRegistrationParams {
            owner: owner1.clone(),
            name: String::from_str(&env, name),
            description: String::from_str(&env, "Valid description"),
            category: String::from_str(&env, "DeFi"),
            website: None,
            logo_cid: None,
            metadata_cid: None,
        };
        client.register_project(&params);
    }

    // Owner 2 should still be able to register projects
    let params = ProjectRegistrationParams {
        owner: owner2.clone(),
        name: String::from_str(&env, "Owner2Project1"),
        description: String::from_str(&env, "Valid description"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };

    let result = client.try_register_project(&params);
    assert!(result.is_ok());

    // Verify counts
    assert_eq!(client.get_owner_project_count(&owner1), 5);
    assert_eq!(client.get_owner_project_count(&owner2), 1);
}

#[test]
fn test_register_project_below_limit_succeeds() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    env.mock_all_auths();

    let owner = Address::generate(&env);

    // Register 10 projects (well below limit)
    let project_names = [
        "ProjectB0",
        "ProjectB1",
        "ProjectB2",
        "ProjectB3",
        "ProjectB4",
        "ProjectB5",
        "ProjectB6",
        "ProjectB7",
        "ProjectB8",
        "ProjectB9",
    ];

    for name in &project_names {
        let params = ProjectRegistrationParams {
            owner: owner.clone(),
            name: String::from_str(&env, name),
            description: String::from_str(&env, "Valid description"),
            category: String::from_str(&env, "DeFi"),
            website: None,
            logo_cid: None,
            metadata_cid: None,
        };
        let result = client.try_register_project(&params);
        assert!(result.is_ok());
    }

    // Verify count
    assert_eq!(client.get_owner_project_count(&owner), 10);
}

// ── Project Name Update Tests ──

#[test]
fn test_update_project_name_updates_mapping() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    env.mock_all_auths();

    let owner = Address::generate(&env);

    // Register a project
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "OriginalName"),
        description: String::from_str(&env, "Description"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };
    let project_id = client.register_project(&params);

    // Update the project name
    let update_params = ProjectUpdateParams {
        project_id,
        caller: owner.clone(),
        name: Some(String::from_str(&env, "NewName")),
        description: None,
        category: None,
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };
    let updated_project = client.update_project(&update_params);

    // Verify the project name was updated
    assert_eq!(updated_project.name, String::from_str(&env, "NewName"));

    // Verify we can retrieve the project by new name
    let retrieved = client.get_project(&project_id);
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().name, String::from_str(&env, "NewName"));
}

#[test]
fn test_update_project_name_to_existing_name_fails() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    env.mock_all_auths();

    let owner = Address::generate(&env);

    // Register first project
    let params1 = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "Project1"),
        description: String::from_str(&env, "Description"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };
    let project_id1 = client.register_project(&params1);

    // Register second project
    let params2 = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "Project2"),
        description: String::from_str(&env, "Description"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };
    client.register_project(&params2);

    // Try to update project1's name to project2's name - should fail
    let update_params = ProjectUpdateParams {
        project_id: project_id1,
        caller: owner.clone(),
        name: Some(String::from_str(&env, "Project2")),
        description: None,
        category: None,
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };

    let result = client.try_update_project(&update_params);
    assert_eq!(result, Err(Ok(ContractError::ProjectAlreadyExists)));
}

#[test]
fn test_update_project_name_to_same_name_succeeds() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    env.mock_all_auths();

    let owner = Address::generate(&env);

    // Register a project
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "ProjectName"),
        description: String::from_str(&env, "Description"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };
    let project_id = client.register_project(&params);

    // Update the project name to the same name - should succeed
    let update_params = ProjectUpdateParams {
        project_id,
        caller: owner.clone(),
        name: Some(String::from_str(&env, "ProjectName")),
        description: Some(String::from_str(&env, "Updated description")),
        category: None,
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };

    let updated = client.update_project(&update_params);
    assert_eq!(updated.name, String::from_str(&env, "ProjectName"));
    assert_eq!(
        updated.description,
        String::from_str(&env, "Updated description")
    );
}

#[test]
fn test_update_project_name_old_name_no_longer_works() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    env.mock_all_auths();

    let owner = Address::generate(&env);

    // Register a project
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "OldProjectName"),
        description: String::from_str(&env, "Description"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };
    let project_id = client.register_project(&params);

    // Update the project name
    let update_params = ProjectUpdateParams {
        project_id,
        caller: owner.clone(),
        name: Some(String::from_str(&env, "NewProjectName")),
        description: None,
        category: None,
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };
    client.update_project(&update_params);

    // Try to register a new project with the old name - should succeed
    // (proving the old mapping was removed)
    let new_params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "OldProjectName"),
        description: String::from_str(&env, "New project with old name"),
        category: String::from_str(&env, "NFT"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };

    let new_project_id = client.register_project(&new_params);

    // Verify the new project has a different ID
    assert_ne!(new_project_id, project_id);
}

#[test]
fn test_update_project_multiple_name_changes() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    env.mock_all_auths();

    let owner = Address::generate(&env);

    // Register a project
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "Name1"),
        description: String::from_str(&env, "Description"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };
    let project_id = client.register_project(&params);

    // Update name to Name2
    let update_params1 = ProjectUpdateParams {
        project_id,
        caller: owner.clone(),
        name: Some(String::from_str(&env, "Name2")),
        description: None,
        category: None,
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };
    client.update_project(&update_params1);

    // Update name to Name3
    let update_params2 = ProjectUpdateParams {
        project_id,
        caller: owner.clone(),
        name: Some(String::from_str(&env, "Name3")),
        description: None,
        category: None,
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };
    client.update_project(&update_params2);

    // Verify final name is Name3
    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.name, String::from_str(&env, "Name3"));

    // Verify Name1 and Name2 can be used for new projects
    let new_params1 = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "Name1"),
        description: String::from_str(&env, "Reusing Name1"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };
    client.register_project(&new_params1);

    let new_params2 = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "Name2"),
        description: String::from_str(&env, "Reusing Name2"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };
    client.register_project(&new_params2);
}

// ── Validation Tests for Website, Category, CIDs ──

#[test]
fn test_register_project_invalid_website_fails() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    env.mock_all_auths();
    let owner = Address::generate(&env);

    let test_cases = [
        "ftp://example.com", // invalid scheme
        "",                  // empty
        "example.com",       // no scheme
    ];

    for website in test_cases {
        let params = ProjectRegistrationParams {
            owner: owner.clone(),
            name: String::from_str(&env, "ValidName"),
            description: String::from_str(&env, "Valid desc"),
            category: String::from_str(&env, "DeFi"),
            website: Some(String::from_str(&env, website)),
            logo_cid: None,
            metadata_cid: None,
        };
        let result = client.try_register_project(&params);
        assert_eq!(result, Err(Ok(ContractError::InvalidProjectWebsite)));
    }
}

#[test]
fn test_register_project_website_too_long_fails() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    env.mock_all_auths();
    let owner = Address::generate(&env);

    extern crate alloc;
    let mut long_website = alloc::string::String::from("https://");
    long_website.push_str(&"a".repeat(300));

    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "ValidName"),
        description: String::from_str(&env, "Valid desc"),
        category: String::from_str(&env, "DeFi"),
        website: Some(String::from_str(&env, &long_website)),
        logo_cid: None,
        metadata_cid: None,
    };
    let result = client.try_register_project(&params);
    assert_eq!(result, Err(Ok(ContractError::ProjectWebsiteTooLong)));
}

#[test]
fn test_register_project_invalid_logo_cid_fails() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    env.mock_all_auths();
    let owner = Address::generate(&env);

    let test_cases = ["invalid_cid", "", "QmShort"];

    for cid in test_cases {
        let params = ProjectRegistrationParams {
            owner: owner.clone(),
            name: String::from_str(&env, "ValidName"),
            description: String::from_str(&env, "Valid desc"),
            category: String::from_str(&env, "DeFi"),
            website: None,
            logo_cid: Some(String::from_str(&env, cid)),
            metadata_cid: None,
        };
        let result = client.try_register_project(&params);
        assert_eq!(result, Err(Ok(ContractError::InvalidProjectLogoCid)));
    }
}

#[test]
fn test_register_project_invalid_metadata_cid_fails() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    env.mock_all_auths();
    let owner = Address::generate(&env);

    let test_cases = ["invalid_cid", "", "QmShort"];

    for cid in test_cases {
        let params = ProjectRegistrationParams {
            owner: owner.clone(),
            name: String::from_str(&env, "ValidName"),
            description: String::from_str(&env, "Valid desc"),
            category: String::from_str(&env, "DeFi"),
            website: None,
            logo_cid: None,
            metadata_cid: Some(String::from_str(&env, cid)),
        };
        let result = client.try_register_project(&params);
        assert_eq!(result, Err(Ok(ContractError::InvalidProjectMetadataCid)));
    }
}

#[test]
fn test_register_project_category_too_long_fails() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    env.mock_all_auths();
    let owner = Address::generate(&env);

    let long_category = "a".repeat(100);

    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "ValidName"),
        description: String::from_str(&env, "Valid desc"),
        category: String::from_str(&env, &long_category),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };
    let result = client.try_register_project(&params);
    assert_eq!(result, Err(Ok(ContractError::ProjectCategoryTooLong)));
}
