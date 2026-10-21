//! Issue #973 — Contract state integrity after emergency operations.
//!
//! The emergency pause mechanism flips a single boolean flag
//! (`StorageKey::ContractPaused`) and must never mutate, drop, or roll back
//! any stored record.  These tests verify that:
//!
//! * Data integrity is preserved across pause/unpause cycles.
//! * All operations resume correctly after unpause.
//! * Partial pause scenarios (some data created before pause, some after
//!   unpause) remain consistent.
//! * TTL values are preserved through a pause cycle.
//! * Recovery procedures (admin writes during pause, reads during pause)
//!   work as documented.

use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::ProjectRegistrationParams;
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};

// ── Data integrity after pause cycle ─────────────────────────────────────────

#[test]
fn test_data_integrity_after_pause_cycle_projects() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let id1 = create_test_project(&client, &owner, "IntegrityA");
    let id2 = create_test_project(&client, &owner, "IntegrityB");

    let before1 = client.get_project(&id1);
    let before2 = client.get_project(&id2);
    let count_before = client.get_project_count();

    // Pause → unpause.
    client.mock_all_auths().pause(&admin);
    assert!(client.is_paused());
    client.mock_all_auths().unpause(&admin);
    assert!(!client.is_paused());

    // All project data is byte-for-byte identical.
    assert_eq!(client.get_project(&id1), before1);
    assert_eq!(client.get_project(&id2), before2);
    assert_eq!(client.get_project_count(), count_before);
}

#[test]
fn test_data_integrity_after_pause_cycle_reviews() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "ReviewIntegrity");

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    client.mock_all_auths().add_review(&project_id, &r1, &5, &None);
    client.mock_all_auths().add_review(&project_id, &r2, &3, &None);

    let reviews_before = client.list_reviews(&project_id, &0, &10);
    let stats_before = client.get_project_stats(&project_id);

    client.mock_all_auths().pause(&admin);
    client.mock_all_auths().unpause(&admin);

    assert_eq!(client.list_reviews(&project_id, &0, &10), reviews_before);
    assert_eq!(client.get_project_stats(&project_id), stats_before);
}

// ── All operations resume correctly after unpause ───────────────────────────

#[test]
fn test_operations_resume_after_unpause() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "ResumeTest");

    // Pause.
    client.mock_all_auths().pause(&admin);

    // Mutating ops fail while paused.
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "ShouldFail"),
        slug: String::from_str(&env, "should-fail"),
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
    assert_eq!(
        client.mock_all_auths().try_register_project(&params),
        Err(Ok(ContractError::ContractPaused))
    );

    // Unpause.
    client.mock_all_auths().unpause(&admin);

    // Mutating ops now succeed.
    let params2 = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "ShouldSucceed"),
        slug: String::from_str(&env, "should-succeed"),
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
    let new_id = client.mock_all_auths().register_project(&params2);
    assert!(new_id > project_id);

    // Reviews work too.
    let reviewer = Address::generate(&env);
    client.mock_all_auths().add_review(&project_id, &reviewer, &4, &None);
    let review = client.get_review(&project_id, &reviewer).unwrap();
    assert_eq!(review.rating, 4);
}

// ── Partial pause scenario: data before and after pause coexist ─────────────

#[test]
fn test_partial_pause_scenario_data_coexists() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    // Data created BEFORE pause.
    let pre_id = create_test_project(&client, &owner, "PrePause");
    let pre_reviewer = Address::generate(&env);
    client.mock_all_auths().add_review(&pre_id, &pre_reviewer, &5, &None);

    // Pause.
    client.mock_all_auths().pause(&admin);
    assert!(client.is_paused());

    // Reads work during pause.
    assert_eq!(
        client.get_project(&pre_id).unwrap().name,
        String::from_str(&env, "PrePause")
    );

    // Unpause.
    client.mock_all_auths().unpause(&admin);

    // Data created AFTER unpause.
    let post_id = create_test_project(&client, &owner, "PostUnpause");
    let post_reviewer = Address::generate(&env);
    client.mock_all_auths().add_review(&post_id, &post_reviewer, &4, &None);

    // Both pre and post data are intact and consistent.
    assert_eq!(client.get_project_count(), 2);
    assert_eq!(
        client.get_review(&pre_id, &pre_reviewer).unwrap().rating,
        5
    );
    assert_eq!(
        client.get_review(&post_id, &post_reviewer).unwrap().rating,
        4
    );
}

// ── TTL preservation through pause cycle ──────────────────────────────────────

#[test]
fn test_ttl_preserved_through_pause_cycle() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let p1 = create_test_project(&client, &owner, "TtlPreserveA");
    let p2 = create_test_project(&client, &owner, "TtlPreserveB");

    // Extend TTL before pause.
    let mut ids = Vec::new(&env);
    ids.push_back(p1);
    ids.push_back(p2);
    let result = client.mock_all_auths().extend_projects_ttl(&ids);
    assert_eq!(result.refreshed, 2);

    // Pause → unpause cycle.
    client.mock_all_auths().pause(&admin);
    client.mock_all_auths().unpause(&admin);

    // TTL extension still works after unpause — data wasn't lost.
    let result = client.mock_all_auths().extend_projects_ttl(&ids);
    assert_eq!(result.refreshed, 2);
    assert_eq!(result.skipped_ids.len(), 0);
}

#[test]
fn test_ttl_extension_works_during_pause() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "TtlDuringPause");

    client.mock_all_auths().pause(&admin);

    // TTL extension is an admin recovery function — should work during pause.
    let mut ids = Vec::new(&env);
    ids.push_back(project_id);
    let result = client.mock_all_auths().extend_projects_ttl(&ids);
    assert_eq!(result.refreshed, 1);

    client.mock_all_auths().unpause(&admin);

    // Project still accessible.
    assert!(client.get_project(&project_id).is_some());
}

// ── Recovery procedures ─────────────────────────────────────────────────────

#[test]
fn test_recovery_procedure_admin_writes_during_pause_persist() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let treasury = Address::generate(&env);

    client.mock_all_auths().pause(&admin);

    // Admin recovery: set fee config during pause.
    client
        .mock_all_auths()
        .set_fee(&admin, &None, &999u128, &50u128, &treasury);

    client.mock_all_auths().unpause(&admin);

    // Fee config persists after unpause.
    let config = client.get_fee_config().unwrap();
    assert_eq!(config.verification_fee, 999);
    assert_eq!(config.registration_fee, 50);
}

#[test]
fn test_recovery_procedure_reads_during_pause_return_correct_data() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "ReadDuringPause");
    let reviewer = Address::generate(&env);
    client.mock_all_auths().add_review(&project_id, &reviewer, &5, &None);

    client.mock_all_auths().pause(&admin);

    // All reads must return correct data during pause.
    assert!(client.is_paused());
    assert!(client.get_project(&project_id).is_some());
    assert_eq!(client.get_project_count(), 1);
    assert!(client.get_review(&project_id, &reviewer).is_some());

    let stats = client.get_project_stats(&project_id);
    assert_eq!(stats.review_count, 1);

    let reviews = client.list_reviews(&project_id, &0, &10);
    assert_eq!(reviews.len(), 1);

    client.mock_all_auths().unpause(&admin);
    assert!(!client.is_paused());
}

// ── Repeated pause cycles: data integrity across multiple cycles ────────────

#[test]
fn test_repeated_pause_cycles_preserve_all_state() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "MultiCycle");
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    client.mock_all_auths().add_review(&project_id, &r1, &5, &None);
    client.mock_all_auths().add_review(&project_id, &r2, &3, &None);

    let project_before = client.get_project(&project_id);
    let stats_before = client.get_project_stats(&project_id);
    let count_before = client.get_project_count();

    for _ in 0..5 {
        client.mock_all_auths().pause(&admin);
        assert!(client.is_paused());
        // Reads are consistent during pause.
        assert_eq!(client.get_project(&project_id), project_before);
        assert_eq!(client.get_project_stats(&project_id), stats_before);
        client.mock_all_auths().unpause(&admin);
        assert!(!client.is_paused());
    }

    // All state unchanged after 5 cycles.
    assert_eq!(client.get_project(&project_id), project_before);
    assert_eq!(client.get_project_stats(&project_id), stats_before);
    assert_eq!(client.get_project_count(), count_before);
}

// ── Emergency pause + admin list integrity ──────────────────────────────────

#[test]
fn test_pause_cycle_preserves_admin_list() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let admin2 = Address::generate(&env);
    let admin3 = Address::generate(&env);

    client.mock_all_auths().add_admin(&admin, &admin2);
    client.mock_all_auths().add_admin(&admin, &admin3);

    let admins_before = client.get_admin_list();
    let count_before = client.get_admin_count();

    client.mock_all_auths().pause(&admin);
    client.mock_all_auths().unpause(&admin);

    assert_eq!(client.get_admin_list(), admins_before);
    assert_eq!(client.get_admin_count(), count_before);
}
