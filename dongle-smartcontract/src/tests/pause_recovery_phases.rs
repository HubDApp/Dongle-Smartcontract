//! Tests for the staged pause-recovery plan — issue #903.

use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::ProjectRegistrationParams;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

fn register_project(env: &Env, client: &crate::DongleContractClient<'_>, owner: &Address) -> u64 {
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(env, "Phase Test Project"),
        slug: String::from_str(env, "phase-test-project"),
        description: String::from_str(env, "Project used by the pause recovery tests"),
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
        language_code: None,
    };
    client.mock_all_auths().register_project(&params)
}

#[test]
fn test_resume_phase_requires_paused_contract() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let result = client
        .mock_all_auths()
        .try_resume_operations_phase(&admin, &2);
    assert_eq!(result, Err(Ok(ContractError::ContractNotPaused)));
}

#[test]
fn test_unknown_phase_is_rejected() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    client.mock_all_auths().pause(&admin);

    let result = client
        .mock_all_auths()
        .try_resume_operations_phase(&admin, &99);
    assert_eq!(result, Err(Ok(ContractError::InvalidPausePhase)));
}

#[test]
fn test_resume_only_enables_the_requested_area() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    client.mock_all_auths().pause(&admin);

    // Core writes are still halted while only the reviews phase is resumed.
    client.mock_all_auths().resume_operations_phase(&admin, &3);

    let state = client.get_pause_recovery_state();
    assert!(state.is_paused);
    assert_eq!(state.enabled_phases.len(), 1);
    assert_eq!(state.enabled_phases.get(0), Some(3));

    // A core write is still rejected.
    let result = client
        .mock_all_auths()
        .try_register_project(&ProjectRegistrationParams {
            owner: owner.clone(),
            name: String::from_str(&env, "Blocked While Paused"),
            slug: String::from_str(&env, "blocked-while-paused"),
            description: String::from_str(&env, "Must be rejected while the core phase is halted"),
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
        });
    assert_eq!(result, Err(Ok(ContractError::ContractPaused)));
}

#[test]
fn test_resumed_phase_allows_its_operations() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = register_project(&env, &client, &owner);

    client.mock_all_auths().pause(&admin);
    client.mock_all_auths().resume_operations_phase(&admin, &3);

    // Reviews belong to the resumed phase and go through.
    let reviewer = Address::generate(&env);
    client
        .mock_all_auths()
        .add_review(&project_id, &reviewer, &5u32, &None);

    let tally = client.get_project_stats(&project_id);
    assert_eq!(tally.review_count, 1);
}

#[test]
fn test_rollback_halts_the_last_phase_again() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    client.mock_all_auths().pause(&admin);
    client.mock_all_auths().resume_operations_phase(&admin, &2);
    client.mock_all_auths().resume_operations_phase(&admin, &3);

    let state = client.get_pause_recovery_state();
    assert_eq!(state.enabled_phases.len(), 2);

    let state = client.mock_all_auths().rollback_recovery_phase(&admin);
    assert_eq!(state.enabled_phases.len(), 1);
    assert_eq!(state.enabled_phases.get(0), Some(2));

    // Nothing left to roll back.
    let result = client.mock_all_auths().try_rollback_recovery_phase(&admin);
    assert_eq!(result, Err(Ok(ContractError::NoPhaseToRollBack)));
}

#[test]
fn test_pause_resets_the_recovery_state() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    client.mock_all_auths().pause(&admin);
    client.mock_all_auths().resume_operations_phase(&admin, &2);
    assert_eq!(client.get_pause_recovery_state().enabled_phases.len(), 1);

    // A new incident starts from a full pause again.
    client.mock_all_auths().pause(&admin);
    let state = client.get_pause_recovery_state();
    assert!(state.is_paused);
    assert_eq!(state.enabled_phases.len(), 0);

    // ... and the default plan is intact.
    assert_eq!(state.plan.len(), 6);
}

#[test]
fn test_custom_recovery_plan_is_installed() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let mut plan: soroban_sdk::Vec<u32> = soroban_sdk::Vec::new(&env);
    plan.push_back(1);
    plan.push_back(3);

    let state = client
        .mock_all_auths()
        .set_pause_recovery_plan(&admin, &plan);
    assert_eq!(state.plan.len(), 2);

    // Phase 2 is not part of the custom plan any more.
    client.mock_all_auths().pause(&admin);
    let result = client
        .mock_all_auths()
        .try_resume_operations_phase(&admin, &2);
    assert_eq!(result, Err(Ok(ContractError::InvalidPausePhase)));

    let result = client
        .mock_all_auths()
        .try_resume_operations_phase(&admin, &3);
    assert!(result.is_ok());
}

#[test]
fn test_core_writes_resume_in_their_own_phase() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    client.mock_all_auths().pause(&admin);
    client.mock_all_auths().resume_operations_phase(&admin, &2);

    let project_id = register_project(&env, &client, &owner);
    assert!(project_id > 0);

    let _ = create_test_project(&client, &owner, "Another Project");
}
