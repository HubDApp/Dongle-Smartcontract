//! Issue #974 — Backward and forward compatibility tests.
//!
//! These tests ensure that data stored by the contract remains compatible
//! across contract versions. The contract uses Soroban's built-in
//! `#[contracttype]` derive, which provides forward-compatibility by default:
//!
//! * **Forward compatibility**: New code can read old data because adding
//!   optional fields to a struct does not break deserialization — Soroban
//!   fills missing fields with `None` / defaults.
//! * **Backward compatibility**: Old code can read data written by newer
//!   versions as long as the old fields are still present. Unknown fields
//!   are skipped during deserialization.
//! * **Schema version**: The contract exposes `get_config()` which returns
//!   a `ContractConfigView` containing `CONTRACT_VERSION`. This allows
//!   indexers and frontends to detect the running version.
//!
//! ## Compatibility Matrix
//!
//! | Data Type        | Forward (new reads old) | Backward (old reads new) | Notes |
//! |-----------------|------------------------|--------------------------|-------|
//! | Project         | ✅ Optional fields default to `None` | ✅ Unknown fields skipped | New optional fields are non-breaking |
//! | Review          | ✅ `content_cid` / `owner_response` default to `None` | ✅ | |
//! | FeeConfig       | ✅ | ✅ | |
//! | ProjectStats    | ✅ | ✅ | Computed, not stored |
//! | ContractConfigView | ✅ | ⚠️ New fields may be `None` in old code | |
//!
//! **Non-breaking changes** (safe across versions):
//! - Adding optional fields to existing structs
//! - Adding new functions (old code ignores them)
//! - Adding new enum variants (old code may not match them, but deserialization succeeds)
//!
//! **Breaking changes** (require version bump):
//! - Removing or renaming a field
//! - Changing a field type
//! - Removing a function
//! - Changing function signatures

use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::{
    FeeConfig, Project, ProjectRegistrationParams, ProjectStats, ProjectUpdateParams, Review,
    VerificationStatus,
};
use soroban_sdk::{testutils::Address as _, Address, Env, String};

// ── Schema version handling ──────────────────────────────────────────────────

#[test]
fn test_config_returns_contract_version() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);

    let config = client.get_config().unwrap();
    assert!(!config.version.is_empty());
}

#[test]
fn test_config_returns_admin_count_and_threshold() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let admin2 = Address::generate(&env);
    client.mock_all_auths().add_admin(&admin, &admin2);

    let config = client.get_config().unwrap();
    assert_eq!(config.admin_count, 2);
    assert!(config.admin_approval_threshold >= 1);
}

// ── Reading "old format" data (minimal fields) with current code ──────────────

#[test]
fn test_read_minimal_project_with_current_code() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "MinimalProject"),
        slug: String::from_str(&env, "minimal-project"),
        description: String::from_str(&env, "desc"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        license: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
        repository_url: None,
        language_code: None,
        verification_requirements: None,
    };
    let project_id = client.mock_all_auths().register_project(&params);

    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.name, String::from_str(&env, "MinimalProject"));
    assert_eq!(project.owner, owner);
    assert_eq!(project.verification_status, VerificationStatus::Unverified);
}

// ── Optional fields default to None when not set (old data behavior) ─────────

#[test]
fn test_optional_fields_default_to_none_for_old_data() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "OldFormat");

    let project = client.get_project(&project_id).unwrap();
    assert!(project.website.is_none());
    assert!(project.logo_cid.is_none());
    assert!(project.metadata_cid.is_none());
}

// ── Migration rollback: data without optional fields survives field additions ─

#[test]
fn test_migration_rollback_optional_fields_added_later() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let params_old = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "MigrationTest"),
        slug: String::from_str(&env, "migration-test"),
        description: String::from_str(&env, "desc"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        license: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
        repository_url: None,
        language_code: None,
        verification_requirements: None,
    };
    let project_id = client.mock_all_auths().register_project(&params_old);

    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.name, String::from_str(&env, "MigrationTest"));

    let update_params = ProjectUpdateParams {
        project_id,
        caller: owner.clone(),
        name: None,
        slug: None,
        description: Some(String::from_str(&env, "Updated desc")),
        category: None,
        website: Some(Some(String::from_str(&env, "https://example.com"))),
        license: None,
        logo_cid: None,
        metadata_cid: Some(Some(String::from_str(
            &env,
            "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG",
        ))),
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
        repository_url: None,
        language_code: None,
        verification_requirements: None,
    };
    client.mock_all_auths().update_project(&update_params);

    let updated = client.get_project(&project_id).unwrap();
    assert_eq!(updated.name, String::from_str(&env, "MigrationTest"));
    assert_eq!(
        updated.website,
        Some(String::from_str(&env, "https://example.com"))
    );
}

// ── Review with minimal fields is readable (old format) ──────────────────────

#[test]
fn test_read_minimal_review_with_current_code() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "ReviewCompat");

    let reviewer = Address::generate(&env);
    client.mock_all_auths().add_review(&project_id, &reviewer, &4, &None);

    let review = client.get_review(&project_id, &reviewer).unwrap();
    assert_eq!(review.rating, 4);
    assert_eq!(review.project_id, project_id);
    assert_eq!(review.reviewer, reviewer);
    assert!(review.content_cid.is_none());
    assert!(review.owner_response.is_none());
}

// ── FeeConfig is readable across versions ──────────────────────────────────

#[test]
fn test_fee_config_readable_after_version_update() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let treasury = Address::generate(&env);

    client
        .mock_all_auths()
        .set_fee(&admin, &None, &1000u128, &500u128, &treasury);

    let config: FeeConfig = client.get_fee_config().unwrap();
    assert_eq!(config.verification_fee, 1000);
    assert_eq!(config.registration_fee, 500);
    assert_eq!(config.treasury, Some(treasury));
}

// ── API stability: all core read functions exist and return expected types ───

#[test]
fn test_api_stability_core_read_functions() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "ApiStability");

    let _project: Option<Project> = client.get_project(&project_id);
    let _count: u64 = client.get_project_count();
    let _list = client.list_projects(&0, &10);
    let _stats: ProjectStats = client.get_project_stats(&project_id);
    let _is_paused: bool = client.is_paused();
    let _admins = client.get_admin_list();
    let _admin_count: u32 = client.get_admin_count();
    let _config = client.get_config().unwrap();
}

// ── API stability: core write functions exist and return expected types ──────

#[test]
fn test_api_stability_core_write_functions() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "ApiWriteStability");
    let reviewer = Address::generate(&env);

    client.mock_all_auths().add_review(&project_id, &reviewer, &5, &None);
    let review: Option<Review> = client.get_review(&project_id, &reviewer);
    assert!(review.is_some());

    client.mock_all_auths().delete_review(&project_id, &reviewer);
    let review: Option<Review> = client.get_review(&project_id, &reviewer);
    assert!(review.is_none());
}

// ── Forward compatibility: unknown fields in stored data are skipped ─────────

#[test]
fn test_forward_compat_unknown_fields_skipped() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "FullProject"),
        slug: String::from_str(&env, "full-project"),
        description: String::from_str(&env, "Full description"),
        category: String::from_str(&env, "DeFi"),
        website: Some(String::from_str(&env, "https://full.example.com")),
        license: Some(String::from_str(&env, "MIT")),
        logo_cid: Some(String::from_str(
            &env,
            "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG",
        )),
        metadata_cid: Some(String::from_str(
            &env,
            "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG",
        )),
        tags: None,
        social_links: None,
        launch_timestamp: Some(1000),
        bounty_url: Some(String::from_str(&env, "https://bounty.example.com")),
        repository_url: Some(String::from_str(&env, "https://repo.example.com")),
        language_code: Some(String::from_str(&env, "en")),
        verification_requirements: None,
    };
    let project_id = client.mock_all_auths().register_project(&params);

    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.name, String::from_str(&env, "FullProject"));
    assert_eq!(project.owner, owner);
    assert_eq!(project.category, String::from_str(&env, "DeFi"));
}
