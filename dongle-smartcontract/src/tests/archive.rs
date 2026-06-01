//! Tests for project archive and reactivate functionality.

use crate::errors::ContractError;
use crate::types::VerificationStatus;
use soroban_sdk::{testutils::Address as _, Address, String};

use super::fixtures::{create_test_project, setup_contract};

#[test]
fn test_archive_project_by_owner() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Test Project");

    // Verify project is not archived initially
    let project = client.get_project(&project_id).unwrap();
    assert!(!project.archived);

    // Archive the project
    let result = client.mock_all_auths().archive_project(&project_id, &owner);
    assert!(result.is_ok());

    // Verify project is now archived
    let archived_project = client.get_project(&project_id).unwrap();
    assert!(archived_project.archived);
}

#[test]
fn test_archive_project_updates_timestamp() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Test Project");

    let original_project = client.get_project(&project_id).unwrap();
    let original_updated_at = original_project.updated_at;

    // Advance ledger to ensure timestamp changes
    env.ledger().with_mut(|l| {
        l.timestamp = l.timestamp + 100;
    });

    // Archive the project
    client.mock_all_auths().archive_project(&project_id, &owner).ok();

    // Verify updated_at was changed
    let archived_project = client.get_project(&project_id).unwrap();
    assert!(archived_project.updated_at > original_updated_at);
}

#[test]
fn test_archive_project_unauthorized() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let other_user = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Test Project");

    // Try to archive as non-owner
    let result = client.mock_all_auths().archive_project(&project_id, &other_user);
    assert_eq!(result, Err(ContractError::Unauthorized));

    // Verify project is still not archived
    let project = client.get_project(&project_id).unwrap();
    assert!(!project.archived);
}

#[test]
fn test_archive_nonexistent_project() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let nonexistent_id = 99999u64;

    let result = client.mock_all_auths().archive_project(&nonexistent_id, &owner);
    assert_eq!(result, Err(ContractError::ProjectNotFound));
}

#[test]
fn test_archive_already_archived_project() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Test Project");

    // Archive the project
    client.mock_all_auths().archive_project(&project_id, &owner).ok();

    // Try to archive again
    let result = client.mock_all_auths().archive_project(&project_id, &owner);
    assert_eq!(result, Err(ContractError::ProjectAlreadyArchived));
}

#[test]
fn test_reactivate_project_by_owner() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Test Project");

    // Archive the project
    client.mock_all_auths().archive_project(&project_id, &owner).ok();

    // Verify project is archived
    let archived_project = client.get_project(&project_id).unwrap();
    assert!(archived_project.archived);

    // Reactivate the project
    let result = client.mock_all_auths().reactivate_project(&project_id, &owner);
    assert!(result.is_ok());

    // Verify project is no longer archived
    let reactivated_project = client.get_project(&project_id).unwrap();
    assert!(!reactivated_project.archived);
}

#[test]
fn test_reactivate_project_updates_timestamp() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Test Project");

    // Archive the project
    client.mock_all_auths().archive_project(&project_id, &owner).ok();

    let archived_project = client.get_project(&project_id).unwrap();
    let archived_updated_at = archived_project.updated_at;

    // Advance ledger to ensure timestamp changes
    env.ledger().with_mut(|l| {
        l.timestamp = l.timestamp + 100;
    });

    // Reactivate the project
    client.mock_all_auths().reactivate_project(&project_id, &owner).ok();

    // Verify updated_at was changed
    let reactivated_project = client.get_project(&project_id).unwrap();
    assert!(reactivated_project.updated_at > archived_updated_at);
}

#[test]
fn test_reactivate_project_unauthorized() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let other_user = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Test Project");

    // Archive the project
    client.mock_all_auths().archive_project(&project_id, &owner).ok();

    // Try to reactivate as non-owner
    let result = client.mock_all_auths().reactivate_project(&project_id, &other_user);
    assert_eq!(result, Err(ContractError::Unauthorized));

    // Verify project is still archived
    let project = client.get_project(&project_id).unwrap();
    assert!(project.archived);
}

#[test]
fn test_reactivate_nonexistent_project() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let nonexistent_id = 99999u64;

    let result = client.mock_all_auths().reactivate_project(&nonexistent_id, &owner);
    assert_eq!(result, Err(ContractError::ProjectNotFound));
}

#[test]
fn test_reactivate_non_archived_project() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Test Project");

    // Try to reactivate a project that is not archived
    let result = client.mock_all_auths().reactivate_project(&project_id, &owner);
    assert_eq!(result, Err(ContractError::ProjectNotArchived));
}

#[test]
fn test_archived_project_excluded_from_list_projects() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project1_id = create_test_project(&client, &owner, "Project 1");
    let project2_id = create_test_project(&client, &owner, "Project 2");
    let project3_id = create_test_project(&client, &owner, "Project 3");

    // Archive project 2
    client.mock_all_auths().archive_project(&project2_id, &owner).ok();

    // List projects
    let projects = client.list_projects(&1u64, &100u32);

    // Verify archived project is not in the list
    assert_eq!(projects.len(), 2);
    let project_ids: soroban_sdk::Vec<u64> = projects.iter().map(|p| p.id).collect();
    assert!(project_ids.contains(&project1_id));
    assert!(!project_ids.contains(&project2_id));
    assert!(project_ids.contains(&project3_id));
}

#[test]
fn test_archived_project_excluded_from_list_projects_by_status() {
    let env = soroban_sdk::Env::default();
    let (client, admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project1_id = create_test_project(&client, &owner, "Project 1");
    let project2_id = create_test_project(&client, &owner, "Project 2");

    // Verify both projects
    client.mock_all_auths().approve_verification(&project1_id, &admin).ok();
    client.mock_all_auths().approve_verification(&project2_id, &admin).ok();

    // Archive project 2
    client.mock_all_auths().archive_project(&project2_id, &owner).ok();

    // List verified projects
    let projects = client.list_projects_by_status(&VerificationStatus::Verified, &1u64, &100u32);

    // Verify archived project is not in the list
    assert_eq!(projects.len(), 1);
    assert_eq!(projects.get(0).unwrap().id, project1_id);
}

#[test]
fn test_archived_project_excluded_from_list_projects_by_category() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project1_id = create_test_project(&client, &owner, "Project 1");
    let project2_id = create_test_project(&client, &owner, "Project 2");

    // Archive project 2
    client.mock_all_auths().archive_project(&project2_id, &owner).ok();

    // List projects by category
    let category = String::from_str(&env, "DeFi");
    let projects = client.list_projects_by_category(&category, &0u32, &100u32);

    // Verify archived project is not in the list
    assert_eq!(projects.len(), 1);
    assert_eq!(projects.get(0).unwrap().id, project1_id);
}

#[test]
fn test_archived_project_excluded_from_get_projects_by_owner() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project1_id = create_test_project(&client, &owner, "Project 1");
    let project2_id = create_test_project(&client, &owner, "Project 2");
    let project3_id = create_test_project(&client, &owner, "Project 3");

    // Archive project 2
    client.mock_all_auths().archive_project(&project2_id, &owner).ok();

    // Get projects by owner
    let projects = client.get_projects_by_owner(&owner);

    // Verify archived project is not in the list
    assert_eq!(projects.len(), 2);
    let project_ids: soroban_sdk::Vec<u64> = projects.iter().map(|p| p.id).collect();
    assert!(project_ids.contains(&project1_id));
    assert!(!project_ids.contains(&project2_id));
    assert!(project_ids.contains(&project3_id));
}

#[test]
fn test_archive_reactivate_lifecycle() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Test Project");

    // Initial state: not archived
    let project = client.get_project(&project_id).unwrap();
    assert!(!project.archived);
    let initial_updated_at = project.updated_at;

    // Advance time and archive
    env.ledger().with_mut(|l| {
        l.timestamp = l.timestamp + 100;
    });
    client.mock_all_auths().archive_project(&project_id, &owner).ok();

    let archived_project = client.get_project(&project_id).unwrap();
    assert!(archived_project.archived);
    let archived_updated_at = archived_project.updated_at;
    assert!(archived_updated_at > initial_updated_at);

    // Advance time and reactivate
    env.ledger().with_mut(|l| {
        l.timestamp = l.timestamp + 100;
    });
    client.mock_all_auths().reactivate_project(&project_id, &owner).ok();

    let reactivated_project = client.get_project(&project_id).unwrap();
    assert!(!reactivated_project.archived);
    let reactivated_updated_at = reactivated_project.updated_at;
    assert!(reactivated_updated_at > archived_updated_at);

    // Verify other fields remain unchanged
    assert_eq!(reactivated_project.id, project_id);
    assert_eq!(reactivated_project.owner, owner);
    assert_eq!(reactivated_project.created_at, initial_updated_at);
}

#[test]
fn test_multiple_archive_reactivate_cycles() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Test Project");

    // First cycle: archive and reactivate
    client.mock_all_auths().archive_project(&project_id, &owner).ok();
    let project = client.get_project(&project_id).unwrap();
    assert!(project.archived);

    client.mock_all_auths().reactivate_project(&project_id, &owner).ok();
    let project = client.get_project(&project_id).unwrap();
    assert!(!project.archived);

    // Second cycle: archive and reactivate again
    client.mock_all_auths().archive_project(&project_id, &owner).ok();
    let project = client.get_project(&project_id).unwrap();
    assert!(project.archived);

    client.mock_all_auths().reactivate_project(&project_id, &owner).ok();
    let project = client.get_project(&project_id).unwrap();
    assert!(!project.archived);

    // Third cycle: archive and reactivate once more
    client.mock_all_auths().archive_project(&project_id, &owner).ok();
    let project = client.get_project(&project_id).unwrap();
    assert!(project.archived);

    client.mock_all_auths().reactivate_project(&project_id, &owner).ok();
    let project = client.get_project(&project_id).unwrap();
    assert!(!project.archived);
}

#[test]
fn test_archived_project_still_accessible_via_get_project() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Test Project");

    // Archive the project
    client.mock_all_auths().archive_project(&project_id, &owner).ok();

    // Verify we can still retrieve it via get_project
    let project = client.get_project(&project_id);
    assert!(project.is_some());
    assert!(project.unwrap().archived);
}

#[test]
fn test_archive_preserves_project_metadata() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Test Project");

    let original_project = client.get_project(&project_id).unwrap();
    let original_name = original_project.name.clone();
    let original_description = original_project.description.clone();
    let original_category = original_project.category.clone();
    let original_verification_status = original_project.verification_status;

    // Archive the project
    client.mock_all_auths().archive_project(&project_id, &owner).ok();

    // Verify metadata is preserved
    let archived_project = client.get_project(&project_id).unwrap();
    assert_eq!(archived_project.name, original_name);
    assert_eq!(archived_project.description, original_description);
    assert_eq!(archived_project.category, original_category);
    assert_eq!(archived_project.verification_status, original_verification_status);
    assert_eq!(archived_project.owner, owner);
}

#[test]
fn test_reactivate_preserves_project_metadata() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Test Project");

    let original_project = client.get_project(&project_id).unwrap();
    let original_name = original_project.name.clone();
    let original_description = original_project.description.clone();
    let original_category = original_project.category.clone();
    let original_verification_status = original_project.verification_status;

    // Archive and reactivate
    client.mock_all_auths().archive_project(&project_id, &owner).ok();
    client.mock_all_auths().reactivate_project(&project_id, &owner).ok();

    // Verify metadata is preserved
    let reactivated_project = client.get_project(&project_id).unwrap();
    assert_eq!(reactivated_project.name, original_name);
    assert_eq!(reactivated_project.description, original_description);
    assert_eq!(reactivated_project.category, original_category);
    assert_eq!(reactivated_project.verification_status, original_verification_status);
    assert_eq!(reactivated_project.owner, owner);
}
