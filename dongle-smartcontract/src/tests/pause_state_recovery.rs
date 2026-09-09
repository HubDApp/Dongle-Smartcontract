//! Issue #628 — Contract pause state recovery.
//!
//! These tests back the acceptance criterion "tests verify pause/unpause cycles
//! preserve data integrity" and the post-unpause state-validation section of
//! `docs/EMERGENCY_PAUSE_RECOVERY.md`.
//!
//! The contract's emergency pause (`EmergencyPause::pause` / `unpause`, surfaced
//! as `pause()` / `unpause()` / `is_paused()`) only flips a single boolean
//! (`StorageKey::ContractPaused`). It must never mutate, drop, or roll back any
//! stored record. Every test here registers real state, drives one or more
//! pause/unpause cycles, and asserts the state is byte-for-byte unchanged and
//! the contract is fully operational afterwards.

use crate::types::ProjectRegistrationParams;
use crate::DongleContract;
use crate::DongleContractClient;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env, String};

fn setup(env: &Env) -> (DongleContractClient<'_>, Address) {
    let contract_id = env.register(DongleContract, ());
    let client = DongleContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.mock_all_auths().initialize(&admin);
    (client, admin)
}

fn make_project_params(env: &Env, owner: &Address, name: &str) -> ProjectRegistrationParams {
    ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(env, name),
        slug: String::from_str(env, name),
        description: String::from_str(env, "Test project"),
        category: String::from_str(env, "DeFi"),
        website: None,
        license: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
        repository_url: None,
    }
}

/// Baseline: a pause immediately followed by an unpause leaves the pause flag
/// clear and is observable as a no-op.
#[test]
fn pause_then_unpause_returns_to_operational_state() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    assert!(!client.is_paused());
    client.mock_all_auths().pause(&admin);
    assert!(client.is_paused());
    client.mock_all_auths().unpause(&admin);
    assert!(!client.is_paused());
}

/// Every stored project must be byte-for-byte identical after a pause/unpause
/// cycle, and the project count must be unchanged.
#[test]
fn pause_unpause_preserves_project_data() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);

    let id1 = client
        .mock_all_auths()
        .register_project(&make_project_params(&env, &owner, "Alpha"));
    let id2 = client
        .mock_all_auths()
        .register_project(&make_project_params(&env, &owner, "Beta"));

    let before1 = client.get_project(&id1);
    let before2 = client.get_project(&id2);
    let count_before = client.get_project_count();

    client.mock_all_auths().pause(&admin);
    client.mock_all_auths().unpause(&admin);

    assert_eq!(client.get_project(&id1), before1);
    assert_eq!(client.get_project(&id2), before2);
    assert_eq!(client.get_project_count(), count_before);
}

/// Reviews and their aggregated stats must survive a pause/unpause cycle.
#[test]
fn pause_unpause_preserves_reviews_and_stats() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);

    let project_id = client
        .mock_all_auths()
        .register_project(&make_project_params(&env, &owner, "Reviewed"));

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    client.mock_all_auths().add_review(&project_id, &r1, &5, &None);
    client.mock_all_auths().add_review(&project_id, &r2, &3, &None);

    let reviews_before = client.list_reviews(&project_id, &0u32, &10u32);
    let stats_before = client.get_project_stats(&project_id);

    client.mock_all_auths().pause(&admin);
    client.mock_all_auths().unpause(&admin);

    assert_eq!(client.list_reviews(&project_id, &0u32, &10u32), reviews_before);
    assert_eq!(client.get_project_stats(&project_id), stats_before);
    assert_eq!(client.get_review(&project_id, &r1).unwrap().rating, 5);
    assert_eq!(client.get_review(&project_id, &r2).unwrap().rating, 3);
}

/// Admin-managed curation state (featured list, admin set) must survive a cycle.
#[test]
fn pause_unpause_preserves_featured_and_admin_state() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);
    let second_admin = Address::generate(&env);

    let project_id = client
        .mock_all_auths()
        .register_project(&make_project_params(&env, &owner, "Featured"));
    client.mock_all_auths().set_featured(&admin, &project_id, &true);
    client.mock_all_auths().add_admin(&admin, &second_admin);

    let featured_before = client.list_featured_projects(&0u32, &10u32);
    let admins_before = client.get_admin_list();

    client.mock_all_auths().pause(&admin);
    client.mock_all_auths().unpause(&admin);

    assert_eq!(client.list_featured_projects(&0u32, &10u32), featured_before);
    assert_eq!(client.get_admin_list(), admins_before);
}

/// Repeated pause/unpause cycles are a stable no-op: reads return identical data
/// throughout (including while paused), and the contract lands unpaused and
/// fully operational.
#[test]
fn repeated_pause_unpause_cycles_preserve_data_integrity() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);

    let project_id = client
        .mock_all_auths()
        .register_project(&make_project_params(&env, &owner, "Durable"));
    let reviewer = Address::generate(&env);
    client
        .mock_all_auths()
        .add_review(&project_id, &reviewer, &4, &None);

    let project_before = client.get_project(&project_id);
    let stats_before = client.get_project_stats(&project_id);
    let count_before = client.get_project_count();

    for _ in 0..5 {
        client.mock_all_auths().pause(&admin);
        assert!(client.is_paused());
        // Reads still work while paused and return the same data.
        assert_eq!(client.get_project(&project_id), project_before);
        client.mock_all_auths().unpause(&admin);
        assert!(!client.is_paused());
    }

    assert_eq!(client.get_project(&project_id), project_before);
    assert_eq!(client.get_project_stats(&project_id), stats_before);
    assert_eq!(client.get_project_count(), count_before);

    // Contract is fully operational after the final unpause.
    let new_owner = Address::generate(&env);
    let result = client
        .mock_all_auths()
        .try_register_project(&make_project_params(&env, &new_owner, "PostCycle"));
    assert!(result.is_ok());
    assert_eq!(client.get_project_count(), count_before + 1);
}

/// An admin-recovery write that lands between pause and unpause is persisted
/// normally — the pause flag does not roll anything back on unpause.
#[test]
fn admin_recovery_writes_during_pause_persist_after_unpause() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let treasury = Address::generate(&env);

    client.mock_all_auths().pause(&admin);
    client
        .mock_all_auths()
        .set_fee(&admin, &None, &777u128, &0u128, &treasury);
    client.mock_all_auths().unpause(&admin);

    let config = client.get_fee_config().unwrap();
    assert_eq!(config.verification_fee, 777u128);
}
