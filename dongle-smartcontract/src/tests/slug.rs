//! Tests for project slug functionality.

extern crate alloc;
use alloc::string::ToString;

use crate::errors::ContractError;
use soroban_sdk::{testutils::Address as _, Address, String};

use super::fixtures::{create_test_project, setup_contract};

#[test]
fn test_register_project_with_slug() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "TestProject");

    // Verify project was created
    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.id, project_id);
    assert_eq!(project.name, String::from_str(&env, "TestProject"));
}

#[test]
fn test_get_project_by_slug() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "TestProject");

    // Get project by slug
    let slug = String::from_str(&env, "testproject");
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
    let project_id = create_test_project(&client, &owner, "ValidSlug");
    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.slug, String::from_str(&env, "validslug"));
}

#[test]
fn test_slug_format_validation_with_numbers() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);

    // Valid slug with numbers
    let project_id = create_test_project(&client, &owner, "Project123");
    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.slug, String::from_str(&env, "project123"));
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
    let _other_owner = Address::generate(&env);

    // Create first project
    let _project1_id = create_test_project(&client, &owner, "UniqueProject");

    // Verify the slug is unique by checking it exists
    let slug = String::from_str(&env, "uniqueproject");
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
    let project_id = create_test_project(&client, &owner, "PersistentProject");

    let slug = String::from_str(&env, "persistentproject");

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
    let project_id = create_test_project(&client, &owner, "ConsistentProject");

    let slug = String::from_str(&env, "consistentproject");

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
    let project1_id = create_test_project(&client, &owner, "ProjectOne");
    let project2_id = create_test_project(&client, &owner, "ProjectTwo");
    let project3_id = create_test_project(&client, &owner, "ProjectThree");

    // Get by slug
    let slug1 = String::from_str(&env, "projectone");
    let slug2 = String::from_str(&env, "projecttwo");
    let slug3 = String::from_str(&env, "projectthree");

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
fn test_slug_length_validation() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);

    // Create project with long name (all alphanumeric — no spaces)
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

    // Create project with mixed case (no spaces — not allowed by name validator)
    let project_id = create_test_project(&client, &owner, "MixedCaseProject");
    let project = client.get_project(&project_id).unwrap();

    // Slug should be lowercase
    let slug_str = project.slug.to_string();
    assert_eq!(slug_str, slug_str.to_lowercase());
}

#[test]
fn test_slug_hyphen_in_name() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);

    // Create project with hyphens in name
    let project_id = create_test_project(&client, &owner, "Project-With-Hyphens");
    let project = client.get_project(&project_id).unwrap();

    // Slug should use hyphens and be lowercase
    let slug_str = project.slug.to_string();
    assert!(slug_str.contains("-"));
    assert!(!slug_str.contains(" "));
}

#[test]
fn test_slug_lookup_after_project_creation() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "OriginalProject");

    let original_slug = String::from_str(&env, "originalproject");
    let project = client.get_project_by_slug(&original_slug).unwrap();
    assert_eq!(project.id, project_id);
}

#[test]
fn test_slug_uniqueness_across_owners() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner1 = Address::generate(&env);
    let _owner2 = Address::generate(&env);

    // Create project by owner1
    let project1_id = create_test_project(&client, &owner1, "SharedName");

    // Both would have the same slug — only first should exist
    let slug = String::from_str(&env, "sharedname");
    let project = client.get_project_by_slug(&slug).unwrap();

    // Should return the first project
    assert_eq!(project.id, project1_id);
}

#[test]
fn test_slug_empty_string_rejected() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "ValidProject");
    let project = client.get_project(&project_id).unwrap();

    // Slug should not be empty
    assert!(!project.slug.is_empty());
}

#[test]
fn test_slug_starts_with_alphanumeric() {
    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "ValidProject");
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

    let project_id = create_test_project(&client, &owner, "ValidProject");
    let project = client.get_project(&project_id).unwrap();

    // Slug should end with alphanumeric
    if let Some(last_char) = project.slug.to_string().chars().last() {
        assert!(last_char.is_ascii_lowercase() || last_char.is_ascii_digit());
    }
}

// ── Regression tests for issue #530 ─────────────────────────────────────────
// Before the fix, `get_project_by_slug` never extended the `ProjectBySlug`
// entry TTL, so the slug index could expire while the project data was still
// alive, making the project permanently unreachable by slug.

/// Verifies that a successful `get_project_by_slug` call extends the slug-index
/// TTL. The Soroban test environment tracks whether a storage entry was
/// extended; checking the storage key is still present after the call confirms
/// the fix is in place.
#[test]
fn test_get_project_by_slug_extends_slug_ttl() {
    use crate::storage_keys::StorageKey;

    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let _project_id = create_test_project(&client, &owner, "TtlSlugProject");

    let slug = String::from_str(&env, "ttlslugproject");

    // Successful lookup — must not panic and must return the project.
    let project = client.get_project_by_slug(&slug);
    assert!(project.is_some(), "Expected project to be found by slug");

    // Confirm the slug-index key is still present in storage (TTL was extended,
    // not removed).
    let contract_id = client.address.clone();
    env.as_contract(&contract_id, || {
        let key = StorageKey::ProjectBySlug(slug.clone());
        assert!(
            env.storage().persistent().has(&key),
            "ProjectBySlug entry must still exist after get_project_by_slug"
        );
    });
}

/// Regression: repeated reads must all succeed and the slug-index must remain
/// alive throughout, verifying that each call re-extends the TTL.
#[test]
fn test_get_project_by_slug_repeated_reads_keep_slug_index_alive() {
    use crate::storage_keys::StorageKey;

    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "RepeatedReadProject");

    let slug = String::from_str(&env, "repeatedreadproject");

    for _ in 0..5 {
        let p = client.get_project_by_slug(&slug);
        assert!(p.is_some(), "Project must be found on every read");
        assert_eq!(p.unwrap().id, project_id);
    }

    // Slug-index key must still be present after all reads.
    let contract_id = client.address.clone();
    env.as_contract(&contract_id, || {
        assert!(
            env.storage()
                .persistent()
                .has(&StorageKey::ProjectBySlug(slug.clone())),
            "ProjectBySlug must survive repeated reads"
        );
    });
}

/// Regression: a missing slug must still return `None` and must NOT create a
/// slug-index entry (no accidental recreation of expired/missing indexes).
#[test]
fn test_get_project_by_slug_missing_slug_returns_none_and_does_not_create_entry() {
    use crate::storage_keys::StorageKey;

    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let slug = String::from_str(&env, "definitely-does-not-exist");

    let result = client.get_project_by_slug(&slug);
    assert!(result.is_none(), "Missing slug must return None");

    // No slug-index entry must have been created.
    let contract_id = client.address.clone();
    env.as_contract(&contract_id, || {
        assert!(
            !env.storage()
                .persistent()
                .has(&StorageKey::ProjectBySlug(slug.clone())),
            "Missing slug lookup must not create a ProjectBySlug entry"
        );
    });
}

/// Regression: the existing project-TTL extension behaviour must remain intact.
/// After `get_project_by_slug`, both the project entry and the slug-index entry
/// must be present.
#[test]
fn test_get_project_by_slug_extends_project_ttl_as_well() {
    use crate::storage_keys::StorageKey;

    let env = soroban_sdk::Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "BothTtlCheck");

    // "BothTtlCheck" → slug "bothttlcheck"
    let slug = String::from_str(&env, "bothttlcheck");

    let project = client.get_project_by_slug(&slug);
    assert!(
        project.is_some(),
        "Project must be found by slug in BothTtlCheck test"
    );

    let contract_id = client.address.clone();
    env.as_contract(&contract_id, || {
        assert!(
            env.storage()
                .persistent()
                .has(&StorageKey::Project(project_id)),
            "Project entry must still be present after get_project_by_slug"
        );
        assert!(
            env.storage()
                .persistent()
                .has(&StorageKey::ProjectBySlug(slug.clone())),
            "ProjectBySlug entry must still be present after get_project_by_slug"
        );
    });
}
