//! Pause/unpause state machine tests and data integrity verification (closes #664).
//!
//! ## State Machine
//!
//! ```text
//!               pause(admin)
//!   RUNNING ─────────────────► PAUSED
//!      ▲                          │
//!      └──────────────────────────┘
//!           unpause(admin)
//! ```
//!
//! ### States
//!
//! | State   | `ContractPaused` key | Allowed operations |
//! |---------|----------------------|--------------------|
//! | RUNNING | absent or `false`    | All operations |
//! | PAUSED  | `true`               | Read-only + admin recovery functions |
//!
//! ### Transitions
//!
//! | From    | Event           | Guard            | To      |
//! |---------|-----------------|------------------|---------|
//! | RUNNING | `pause(admin)`  | Caller is admin  | PAUSED  |
//! | PAUSED  | `unpause(admin)`| Caller is admin  | RUNNING |
//! | PAUSED  | `pause(admin)`  | Caller is admin  | PAUSED (idempotent) |
//! | RUNNING | `unpause(admin)`| Caller is admin  | RUNNING (idempotent) |
//!
//! Non-admin callers always get `AdminOnly` regardless of current state.
//!
//! ## Recovery Checklist (for operations team)
//!
//! 1. Identify the admin address(es) authorised to call `unpause`.
//! 2. Call `is_paused()` to confirm the contract is paused.
//! 3. Investigate the incident root cause before unpausing.
//! 4. Call `unpause(admin)` with admin auth.
//! 5. Call `is_paused()` again to confirm the contract is running.
//! 6. Optionally call `get_project`, `get_admin_list`, `get_fee_config` to
//!    verify that all state is intact — the pause flag is the **only** thing
//!    that changes during pause/unpause.
//! 7. Monitor events: a `ContractUnpaused` event should appear on the ledger.
//!
//! ## Data Integrity After Pause/Unpause
//!
//! The pause mechanism stores a single boolean flag under `StorageKey::ContractPaused`.
//! No project, review, admin, fee, or verification data is modified by pause
//! or unpause.  The tests below verify this invariant.

use crate::errors::ContractError;
use crate::types::{ProjectRegistrationParams, ProjectUpdateParams};
use crate::DongleContract;
use crate::DongleContractClient;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

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

// ── State machine: valid transitions ─────────────────────────────────────────

#[test]
fn state_machine_running_to_paused() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    assert!(!client.is_paused(), "initial state must be RUNNING");

    client.mock_all_auths().pause(&admin);

    assert!(client.is_paused(), "state must be PAUSED after pause()");
}

#[test]
fn state_machine_paused_to_running() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    client.mock_all_auths().pause(&admin);
    assert!(client.is_paused(), "state must be PAUSED");

    client.mock_all_auths().unpause(&admin);

    assert!(!client.is_paused(), "state must be RUNNING after unpause()");
}

#[test]
fn state_machine_multiple_pause_unpause_cycles() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    for cycle in 0..5 {
        client.mock_all_auths().pause(&admin);
        assert!(client.is_paused(), "should be PAUSED at cycle {cycle}");

        client.mock_all_auths().unpause(&admin);
        assert!(!client.is_paused(), "should be RUNNING at cycle {cycle}");
    }
}

#[test]
fn state_machine_pause_is_idempotent() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    client.mock_all_auths().pause(&admin);
    assert!(client.is_paused());

    // Calling pause again must not error
    let result = client.mock_all_auths().try_pause(&admin);
    assert!(result.is_ok(), "second pause should succeed (idempotent)");
    assert!(client.is_paused(), "must still be PAUSED");
}

#[test]
fn state_machine_unpause_is_idempotent() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    // Already RUNNING
    let result = client.mock_all_auths().try_unpause(&admin);
    assert!(result.is_ok(), "unpause on RUNNING should succeed (idempotent)");
    assert!(!client.is_paused(), "must still be RUNNING");
}

#[test]
fn state_machine_non_admin_cannot_pause() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let non_admin = Address::generate(&env);

    let result = client.mock_all_auths().try_pause(&non_admin);
    assert_eq!(result, Err(Ok(ContractError::AdminOnly)));
    assert!(!client.is_paused(), "state must remain RUNNING");
}

#[test]
fn state_machine_non_admin_cannot_unpause() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let non_admin = Address::generate(&env);

    client.mock_all_auths().pause(&admin);

    let result = client.mock_all_auths().try_unpause(&non_admin);
    assert_eq!(result, Err(Ok(ContractError::AdminOnly)));
    assert!(client.is_paused(), "state must remain PAUSED");
}

// ── Data integrity after pause/unpause cycles ─────────────────────────────────

/// Projects registered before a pause/unpause cycle must be unchanged.
#[test]
fn data_integrity_projects_survive_pause_unpause() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);

    // Register projects before pause.
    let params = make_project_params(&env, &owner, "StateMachineProject");
    let project_id = client.mock_all_auths().register_project(&params);

    let before = client.get_project(&project_id).expect("project must exist");

    // Pause and unpause.
    client.mock_all_auths().pause(&admin);
    client.mock_all_auths().unpause(&admin);

    let after = client.get_project(&project_id).expect("project must exist after unpause");

    // All fields must be identical.
    assert_eq!(before.name, after.name, "name must not change");
    assert_eq!(before.owner, after.owner, "owner must not change");
    assert_eq!(
        before.verification_status, after.verification_status,
        "verification_status must not change"
    );
    assert_eq!(before.description, after.description);
    assert_eq!(before.category, after.category);
}

/// Project count must not change through pause/unpause.
#[test]
fn data_integrity_project_count_unchanged() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);

    let params_a = make_project_params(&env, &owner, "CountProject-A");
    let params_b = make_project_params(&env, &owner, "CountProject-B");
    client.mock_all_auths().register_project(&params_a);
    client.mock_all_auths().register_project(&params_b);

    let count_before = client.get_project_count();

    client.mock_all_auths().pause(&admin);
    client.mock_all_auths().unpause(&admin);

    let count_after = client.get_project_count();
    assert_eq!(count_before, count_after, "project count must not change");
}

/// Admin list must not change through pause/unpause.
#[test]
fn data_integrity_admin_list_unchanged() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    let list_before = client.get_admin_list();

    client.mock_all_auths().pause(&admin);
    client.mock_all_auths().unpause(&admin);

    let list_after = client.get_admin_list();
    assert_eq!(
        list_before.len(),
        list_after.len(),
        "admin list length must not change"
    );
}

/// Reviews submitted before pause must be readable after unpause.
#[test]
fn data_integrity_reviews_survive_pause_unpause() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);

    let params = make_project_params(&env, &owner, "ReviewedProject");
    let project_id = client.mock_all_auths().register_project(&params);

    client
        .mock_all_auths()
        .add_review(&project_id, &reviewer, &4u32, &None);

    let review_before = client.get_review(&project_id, &reviewer);
    assert!(review_before.is_some());

    // Pause → unpause cycle.
    client.mock_all_auths().pause(&admin);
    client.mock_all_auths().unpause(&admin);

    let review_after = client.get_review(&project_id, &reviewer);
    assert!(review_after.is_some(), "review must survive pause/unpause");

    let r_before = review_before.unwrap();
    let r_after = review_after.unwrap();
    assert_eq!(r_before.rating, r_after.rating);
    assert_eq!(r_before.reviewer, r_after.reviewer);
}

/// Fee configuration must not change through pause/unpause.
#[test]
fn data_integrity_fee_config_survives_pause_unpause() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let treasury = Address::generate(&env);

    client
        .mock_all_auths()
        .set_fee(&admin, &None, &500u128, &50u128, &treasury);

    let fee_before = client.get_fee_config();

    client.mock_all_auths().pause(&admin);
    client.mock_all_auths().unpause(&admin);

    let fee_after = client.get_fee_config();
    assert_eq!(
        fee_before.verification_fee, fee_after.verification_fee,
        "fee config must survive pause/unpause"
    );
    assert_eq!(fee_before.registration_fee, fee_after.registration_fee);
}

/// After unpause, all mutating operations must work again (full recovery).
#[test]
fn recovery_all_operations_work_after_unpause() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);

    client.mock_all_auths().pause(&admin);
    assert!(client.is_paused());
    client.mock_all_auths().unpause(&admin);
    assert!(!client.is_paused());

    // Register project should work.
    let params = make_project_params(&env, &owner, "RecoveryProject");
    let project_id = client.mock_all_auths().register_project(&params);

    // Add review should work.
    let review_result = client
        .mock_all_auths()
        .try_add_review(&project_id, &reviewer, &5u32, &None);
    assert!(review_result.is_ok(), "add_review must work after recovery");

    // Update project should work.
    let update = ProjectUpdateParams {
        project_id,
        caller: owner.clone(),
        name: Some(String::from_str(&env, "UpdatedRecovery")),
        slug: None,
        description: None,
        category: None,
        website: None,
        license: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
        repository_url: None,
    };
    let update_result = client.mock_all_auths().try_update_project(&update);
    assert!(update_result.is_ok(), "update_project must work after recovery");
}

/// Validate that a project's follower list survives pause/unpause cycles.
#[test]
fn data_integrity_followers_survive_pause_unpause() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);
    let follower = Address::generate(&env);

    let params = make_project_params(&env, &owner, "FollowedProject");
    let project_id = client.mock_all_auths().register_project(&params);

    client
        .mock_all_auths()
        .follow_project(&project_id, &follower);

    let count_before = client.get_follower_count(&project_id);
    assert_eq!(count_before, 1);

    // Multiple pause/unpause cycles.
    for _ in 0..3 {
        client.mock_all_auths().pause(&admin);
        client.mock_all_auths().unpause(&admin);
    }

    let count_after = client.get_follower_count(&project_id);
    assert_eq!(
        count_after, 1,
        "follower count must survive pause/unpause cycles"
    );

    let still_following = client.is_following(&project_id, &follower);
    assert!(still_following, "follow relationship must survive pause/unpause");
}

// ── State validation (post-unpause checks) ────────────────────────────────────

/// After unpause, `is_paused()` returns false.
#[test]
fn state_validation_after_unpause_is_paused_is_false() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    client.mock_all_auths().pause(&admin);
    client.mock_all_auths().unpause(&admin);

    assert!(!client.is_paused());
}

/// After unpause, `get_config` continues to report the correct pause state.
#[test]
fn state_validation_get_config_reflects_unpause() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    client.mock_all_auths().pause(&admin);
    client.mock_all_auths().unpause(&admin);

    // get_config should return without error and reflect running state.
    let config = client.get_config();
    assert!(!config.paused, "get_config should show paused=false after unpause");
}
