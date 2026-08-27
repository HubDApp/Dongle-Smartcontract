//! Tests for the Contract Pause / Emergency Stop feature.
//!
//! Coverage includes:
//! - Admin can pause and unpause the contract.
//! - Mutating calls fail while paused (registration, reviews, fees, verification, etc.).
//! - Read calls continue working while paused.
//! - Admin recovery functions bypass the pause.
//! - Pause/unpause emits events.
//! - Non-admin cannot pause/unpause.

use crate::errors::ContractError;
use crate::types::{
    FeeOperation, ProjectRegistrationParams, ProjectUpdateParams, VerificationStatus,
};
use crate::DongleContract;
use crate::DongleContractClient;
use soroban_sdk::testutils::{Address as _, Events, MockAuth, MockAuthInvoke};
use soroban_sdk::{symbol_short, Address, Env, IntoVal, String, Vec};

fn setup(env: &Env) -> (DongleContractClient<'_>, Address) {
    let contract_id = env.register(DongleContract, ());
    let client = DongleContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.mock_all_auths().initialize(&admin);
    (client, admin)
}

fn create_test_project(
    client: &DongleContractClient<'_>,
    owner: &Address,
    name: &str,
) -> Result<u64, ContractError> {
    let env = &client.env;
    let params = ProjectRegistrationParams {
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
    };
    client.mock_all_auths().try_register_project(&params).map(|r| r)
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

// ── Admin pause/unpause ──────────────────────────────────────────────────────

#[test]
fn test_admin_can_pause_and_unpause() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    // Initially not paused
    assert!(!client.is_paused());

    // Admin can pause
    client.mock_all_auths().pause(&admin);
    assert!(client.is_paused());

    // Admin can unpause
    client.mock_all_auths().unpause(&admin);
    assert!(!client.is_paused());
}

#[test]
fn test_non_admin_cannot_pause() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let non_admin = Address::generate(&env);

    let result = client.mock_all_auths().try_pause(&non_admin);
    assert_eq!(result, Err(Ok(ContractError::AdminOnly)));
    assert!(!client.is_paused());
}

#[test]
fn test_non_admin_cannot_unpause() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let non_admin = Address::generate(&env);

    client.mock_all_auths().pause(&admin);
    assert!(client.is_paused());

    let result = client.mock_all_auths().try_unpause(&non_admin);
    assert_eq!(result, Err(Ok(ContractError::AdminOnly)));
    assert!(client.is_paused()); // Still paused
}

// ── Pause/unpause emits events ───────────────────────────────────────────────

#[test]
fn test_pause_emits_event() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    client.mock_all_auths().pause(&admin);

    let events = env.events().all();
    let paused_event = events.find_first(&(symbol_short!("CONTRACT"), symbol_short!("PAUSED")));
    assert!(paused_event.is_some());
}

#[test]
fn test_unpause_emits_event() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    client.mock_all_auths().pause(&admin);
    client.mock_all_auths().unpause(&admin);

    let events = env.events().all();
    let unpaused_event = events.find_first(&(symbol_short!("CONTRACT"), symbol_short!("UNPAUSED")));
    assert!(unpaused_event.is_some());
}

// ── Mutating calls fail while paused: Registration ───────────────────────────

#[test]
fn test_register_project_fails_when_paused() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);

    client.mock_all_auths().pause(&admin);
    assert!(client.is_paused());

    let params = make_project_params(&env, &owner, "TestProject");
    let result = client.mock_all_auths().try_register_project(&params);
    assert_eq!(result, Err(Ok(ContractError::ContractPaused)));
}

#[test]
fn test_update_project_fails_when_paused() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);

    // Create a project first
    let params = make_project_params(&env, &owner, "TestProject");
    let project_id = client.mock_all_auths().register_project(&params);

    // Pause
    client.mock_all_auths().pause(&admin);
    assert!(client.is_paused());

    let update = ProjectUpdateParams {
        project_id,
        caller: owner.clone(),
        name: Some(String::from_str(&env, "Updated")),
        ..Default::default()
    };
    let result = client.mock_all_auths().try_update_project(&update);
    assert_eq!(result, Err(Ok(ContractError::ContractPaused)));
}

// ── Mutating calls fail while paused: Reviews ────────────────────────────────

#[test]
fn test_add_review_fails_when_paused() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);

    let params = make_project_params(&env, &owner, "TestProject");
    let project_id = client.mock_all_auths().register_project(&params);

    client.mock_all_auths().pause(&admin);
    assert!(client.is_paused());

    let reviewer = Address::generate(&env);
    let result = client
        .mock_all_auths()
        .try_add_review(&project_id, &reviewer, &5, &None);
    assert_eq!(result, Err(Ok(ContractError::ContractPaused)));
}

// ── Mutating calls fail while paused: Verification ───────────────────────────

#[test]
fn test_request_verification_fails_when_paused() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);

    let params = make_project_params(&env, &owner, "TestProject");
    let project_id = client.mock_all_auths().register_project(&params);

    client.mock_all_auths().pause(&admin);
    assert!(client.is_paused());

    let result = client.mock_all_auths().try_request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "QmTest1234567890123456789012345678901234567890123"),
    );
    assert_eq!(result, Err(Ok(ContractError::ContractPaused)));
}

// ── Mutating calls fail while paused: Fees ───────────────────────────────────

#[test]
fn test_pay_fee_fails_when_paused() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let treasury = Address::generate(&env);
    let owner = Address::generate(&env);

    // Set up fees
    client
        .mock_all_auths()
        .set_fee(&admin, &None, &1000u128, &0u128, &treasury);

    let params = make_project_params(&env, &owner, "TestProject");
    let project_id = client.mock_all_auths().register_project(&params);

    client.mock_all_auths().pause(&admin);
    assert!(client.is_paused());

    let result = client
        .mock_all_auths()
        .try_pay_fee(&owner, &project_id, &None);
    assert_eq!(result, Err(Ok(ContractError::ContractPaused)));
}

// ── Read calls continue working while paused ────────────────────────────────

#[test]
fn test_read_calls_work_when_paused() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);

    // Create a project first
    let params = make_project_params(&env, &owner, "TestProject");
    let project_id = client.mock_all_auths().register_project(&params);

    // Pause
    client.mock_all_auths().pause(&admin);
    assert!(client.is_paused());

    // Read calls should still work
    let project = client.get_project(&project_id);
    assert!(project.is_some());
    assert_eq!(project.unwrap().name, String::from_str(&env, "TestProject"));

    let count = client.get_project_count();
    assert_eq!(count, 1);

    let is_admin = client.is_admin(&admin);
    assert!(is_admin);

    let admin_list = client.get_admin_list();
    assert_eq!(admin_list.len(), 1);

    let is_paused = client.is_paused();
    assert!(is_paused);
}

// ── Admin recovery functions bypass pause ────────────────────────────────────

#[test]
fn test_admin_management_works_when_paused() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let new_admin = Address::generate(&env);

    client.mock_all_auths().pause(&admin);
    assert!(client.is_paused());

    // Admin management should still work
    client.mock_all_auths().add_admin(&admin, &new_admin);
    assert!(client.is_admin(&new_admin));

    client.mock_all_auths().remove_admin(&admin, &new_admin);
    assert!(!client.is_admin(&new_admin));
}

#[test]
fn test_fee_config_works_when_paused() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let treasury = Address::generate(&env);

    client.mock_all_auths().pause(&admin);
    assert!(client.is_paused());

    // Setting fee config should work (admin recovery)
    let result = client
        .mock_all_auths()
        .try_set_fee(&admin, &None, &500u128, &100u128, &treasury);
    assert!(result.is_ok());

    // Reading fee config should also work
    let config = client.get_fee_config();
    assert!(config.is_ok());
    assert_eq!(config.unwrap().verification_fee, 500u128);
}

// ── Coverage: archive, reactivate, transfers, maintainers, etc. when paused ──

#[test]
fn test_archive_and_reactivate_fail_when_paused() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);

    let params = make_project_params(&env, &owner, "TestProject");
    let project_id = client.mock_all_auths().register_project(&params);

    client.mock_all_auths().pause(&admin);

    let result = client.mock_all_auths().try_archive_project(&project_id, &owner);
    assert_eq!(result, Err(Ok(ContractError::ContractPaused)));

    let result = client
        .mock_all_auths()
        .try_reactivate_project(&project_id, &owner);
    assert_eq!(result, Err(Ok(ContractError::ContractPaused)));
}

// ── Test that verification admin actions (approve/reject/revoke) bypass pause ─

#[test]
fn test_verification_admin_actions_work_when_paused() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);

    let params = make_project_params(&env, &owner, "TestProject");
    let project_id = client.mock_all_auths().register_project(&params);

    // Submit verification request before pausing
    client.mock_all_auths().request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "QmTest1234567890123456789012345678901234567890123"),
    );

    // Pause
    client.mock_all_auths().pause(&admin);
    assert!(client.is_paused());

    // Admin verification actions should still work (admin recovery)
    let result = client
        .mock_all_auths()
        .try_approve_verification(&project_id, &admin);
    assert!(result.is_ok());

    // Verify the project was actually verified
    let verification = client.get_verification(&project_id);
    assert!(verification.is_ok());
}

// ── Coverage: Other mutating operations ──────────────────────────────────

#[test]
fn test_report_project_fails_when_paused() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);

    let params = make_project_params(&env, &owner, "TestProject");
    let project_id = client.mock_all_auths().register_project(&params);

    client.mock_all_auths().pause(&admin);

    let result = client.mock_all_auths().try_report_project(
        &project_id,
        &owner,
        &String::from_str(&env, "QmReason1234567890123456789012345678901234567890123"),
    );
    assert_eq!(result, Err(Ok(ContractError::ContractPaused)));
}

#[test]
fn test_maintainer_ops_fail_when_paused() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);
    let maintainer = Address::generate(&env);

    let params = make_project_params(&env, &owner, "TestProject");
    let project_id = client.mock_all_auths().register_project(&params);

    client.mock_all_auths().pause(&admin);

    let result = client
        .mock_all_auths()
        .try_add_maintainer(&project_id, &owner, &maintainer);
    assert_eq!(result, Err(Ok(ContractError::ContractPaused)));
}

// ── Follow/unfollow, bookmark/unbookmark, endorse/unendorse ──────────────

#[test]
fn test_follow_ops_fail_when_paused() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);
    let follower = Address::generate(&env);

    let params = make_project_params(&env, &owner, "TestProject");
    let project_id = client.mock_all_auths().register_project(&params);

    client.mock_all_auths().pause(&admin);

    let result = client
        .mock_all_auths()
        .try_follow_project(&project_id, &follower);
    assert_eq!(result, Err(Ok(ContractError::ContractPaused)));
}

#[test]
fn test_bookmark_ops_fail_when_paused() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    let params = make_project_params(&env, &owner, "TestProject");
    let project_id = client.mock_all_auths().register_project(&params);

    client.mock_all_auths().pause(&admin);

    let result = client
        .mock_all_auths()
        .try_bookmark_project(&project_id, &user);
    assert_eq!(result, Err(Ok(ContractError::ContractPaused)));
}

#[test]
fn test_endorse_ops_fail_when_paused() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    let params = make_project_params(&env, &owner, "TestProject");
    let project_id = client.mock_all_auths().register_project(&params);

    client.mock_all_auths().pause(&admin);

    let result = client
        .mock_all_auths()
        .try_endorse_project(&project_id, &user);
    assert_eq!(result, Err(Ok(ContractError::ContractPaused)));
}

// ── Double pause/unpause should be idempotent ─────────────────────────────

#[test]
fn test_double_pause_is_idempotent() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    client.mock_all_auths().pause(&admin);
    assert!(client.is_paused());

    // Pausing again should still succeed
    let result = client.mock_all_auths().try_pause(&admin);
    assert!(result.is_ok());
    assert!(client.is_paused());
}

#[test]
fn test_double_unpause_is_idempotent() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    client.mock_all_auths().pause(&admin);
    client.mock_all_auths().unpause(&admin);
    assert!(!client.is_paused());

    // Unpausing again should still succeed
    let result = client.mock_all_auths().try_unpause(&admin);
    assert!(result.is_ok());
    assert!(!client.is_paused());
}

// ── Resume operations after unpause ───────────────────────────────────────

#[test]
fn test_operations_resume_after_unpause() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);

    // Pause then unpause
    client.mock_all_auths().pause(&admin);
    client.mock_all_auths().unpause(&admin);
    assert!(!client.is_paused());

    // Operations should work again
    let params = make_project_params(&env, &owner, "TestProject");
    let result = client.mock_all_auths().try_register_project(&params);
    assert!(result.is_ok());
}

