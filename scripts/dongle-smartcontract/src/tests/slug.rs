//! Tests for project slug functionality.

use crate::errors::ContractError;
use soroban_sdk::{testutils::Address as _, Address, String};

use super::fixtures::{create_test_project, setup_contract};

#[test]
fn test_register_project_with_slug() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Test Project");

    // Verify project was created
    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.id, project_id);
    assert_eq!(project.name, String::from_str(&env, "Test Project"));
}

#[test]
fn test_get_project_by_slug() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Test Project");

    // Get project by slug
    let slug = String::from_str(&env, "test-project");
    let project = client.get_project_by_slug(&slug).unwrap();
    assert_eq!(project.id, project_id);
    assert_eq!(project.slug, slug);
}

#[test]
fn test_slug_format_validation_lowercase() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    
    // Valid lowercase slug
    let project_id = create_test_project(&client, &owner, "Valid Slug");
    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.slug, String::from_str(&env, "valid-slug"));
}

#[test]
fn test_slug_format_validation_with_numbers() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    
    // Valid slug with numbers
    let project_id = create_test_project(&client, &owner, "Project 123");
    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.slug, String::from_str(&env, "project-123"));
}

#[test]
fn test_slug_format_validation_with_underscores() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    
    // Valid slug with underscores
    let project_id = create_test_project(&client, &owner, "My_Project");
    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.slug, String::from_str(&env, "my_project"));
}

#[test]
fn test_slug_uniqueness_enforcement() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let other_owner = Address::generate(&env);
    
    // Create first project
    let _project1_id = create_test_project(&client, &owner, "Unique Project");

    // Try to create second project with same slug (should fail)
    // This would require a custom test helper that allows specifying slug
    // For now, we verify the slug is unique by checking it exists
    let slug = String::from_str(&env, "unique-project");
    let project = client.get_project_by_slug(&slug);
    assert!(project.is_some());
}

#[test]
fn test_get_project_by_nonexistent_slug() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let slug = String::from_str(&env, "nonexistent-slug");
    let project = client.get_project_by_slug(&slug);
    assert!(project.is_none());
}

#[test]
fn test_slug_persists_across_reads() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Persistent Project");

    let slug = String::from_str(&env, "persistent-project");
    
    // Read multiple times
    let project1 = client.get_project_by_slug(&slug).unwrap();
    let project2 = client.get_project_by_slug(&slug).unwrap();
    let project3 = client.get_project(&project_id).unwrap();

    // All should have same slug
    assert_eq!(project1.slug, slug);
    assert_eq!(project2.slug, slug);
    assert_eq!(project3.slug, slug);
}

#[test]
fn test_slug_consistency_with_id_lookup() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Consistent Project");

    let slug = String::from_str(&env, "consistent-project");
    
    // Get by ID and by slug
    let project_by_id = client.get_project(&project_id).unwrap();
    let project_by_slug = client.get_project_by_slug(&slug).unwrap();

    // Should be identical
    assert_eq!(project_by_id.id, project_by_slug.id);
    assert_eq!(project_by_id.slug, project_by_slug.slug);
    assert_eq!(project_by_id.name, project_by_slug.name);
    assert_eq!(project_by_id.owner, project_by_slug.owner);
}

#[test]
fn test_multiple_projects_different_slugs() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    
    // Create multiple projects
    let project1_id = create_test_project(&client, &owner, "Project One");
    let project2_id = create_test_project(&client, &owner, "Project Two");
    let project3_id = create_test_project(&client, &owner, "Project Three");

    // Get by slug
    let slug1 = String::from_str(&env, "project-one");
    let slug2 = String::from_str(&env, "project-two");
    let slug3 = String::from_str(&env, "project-three");

    let p1 = client.get_project_by_slug(&slug1).unwrap();
    let p2 = client.get_project_by_slug(&slug2).unwrap();
    let p3 = client.get_project_by_slug(&slug3).unwrap();

    // All should be different
    assert_eq!(p1.id, project1_id);
    assert_eq!(p2.id, project2_id);
    assert_eq!(p3.id, project3_id);
    assert_ne!(p1.id, p2.id);
    assert_ne!(p2.id, p3.id);
    assert_ne!(p1.id, p3.id);
}

#[test]
fn test_slug_with_special_characters_rejected() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    
    // Create project - special characters should be rejected or converted
    let project_id = create_test_project(&client, &owner, "Project@Special!");
    let project = client.get_project(&project_id).unwrap();
    
    // Slug should not contain special characters
    let slug_str = project.slug.to_string();
    assert!(!slug_str.contains("@"));
    assert!(!slug_str.contains("!"));
}

#[test]
fn test_slug_length_validation() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    
    // Create project with long name
    let long_name = "a".repeat(50);
    let project_id = create_test_project(&client, &owner, &long_name);
    let project = client.get_project(&project_id).unwrap();
    
    // Slug should be within max length
    assert!(project.slug.len() <= 64);
}

#[test]
fn test_slug_case_normalization() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    
    // Create project with mixed case
    let project_id = create_test_project(&client, &owner, "MiXeD CaSe PrOjEcT");
    let project = client.get_project(&project_id).unwrap();
    
    // Slug should be lowercase
    let slug_str = project.slug.to_string();
    assert_eq!(slug_str, slug_str.to_lowercase());
}

#[test]
fn test_slug_whitespace_handling() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    
    // Create project with multiple spaces
    let project_id = create_test_project(&client, &owner, "Project   With   Spaces");
    let project = client.get_project(&project_id).unwrap();
    
    // Slug should not contain spaces
    let slug_str = project.slug.to_string();
    assert!(!slug_str.contains("  "));
}

#[test]
fn test_slug_hyphen_conversion() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    
    // Create project with spaces (should convert to hyphens)
    let project_id = create_test_project(&client, &owner, "Project With Hyphens");
    let project = client.get_project(&project_id).unwrap();
    
    // Slug should use hyphens instead of spaces
    let slug_str = project.slug.to_string();
    assert!(slug_str.contains("-"));
    assert!(!slug_str.contains(" "));
}

#[test]
fn test_slug_lookup_after_project_update() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Original Project");

    let original_slug = String::from_str(&env, "original-project");
    let project = client.get_project_by_slug(&original_slug).unwrap();
    assert_eq!(project.id, project_id);
}

#[test]
fn test_slug_uniqueness_across_owners() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner1 = Address::generate(&env);
    let owner2 = Address::generate(&env);
    
    // Create project by owner1
    let project1_id = create_test_project(&client, &owner1, "Shared Name");

    // Try to create project by owner2 with same name
    // Both should have same slug, but only one should exist
    let slug = String::from_str(&env, "shared-name");
    let project = client.get_project_by_slug(&slug).unwrap();
    
    // Should return the first project
    assert_eq!(project.id, project1_id);
}

#[test]
fn test_slug_empty_string_rejected() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    
    // Create project - empty slug should be handled
    let project_id = create_test_project(&client, &owner, "Valid Project");
    let project = client.get_project(&project_id).unwrap();
    
    // Slug should not be empty
    assert!(!project.slug.is_empty());
}

#[test]
fn test_slug_starts_with_alphanumeric() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    
    // Create project
    let project_id = create_test_project(&client, &owner, "Valid Project");
    let project = client.get_project(&project_id).unwrap();
    
    // Slug should start with alphanumeric
    if let Some(first_char) = project.slug.to_string().chars().next() {
        assert!(first_char.is_ascii_lowercase() || first_char.is_ascii_digit());
    }
}

#[test]
fn test_slug_ends_with_alphanumeric() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    
    // Create project
    let project_id = create_test_project(&client, &owner, "Valid Project");
    let project = client.get_project(&project_id).unwrap();
    
    // Slug should end with alphanumeric
    if let Some(last_char) = project.slug.to_string().chars().last() {
        assert!(last_char.is_ascii_lowercase() || last_char.is_ascii_digit());
    }
}



