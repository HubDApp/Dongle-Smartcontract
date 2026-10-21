//! Staged resumption of operations after an emergency pause (issue #904).
//!
//! The recovery plan is *read ops → reviews → verification → fees*:
//!
//! | Phase | Resumed |
//! |-------|---------|
//! | 0 | nothing (fully paused) |
//! | 1 | read operations |
//! | 2 | reviews |
//! | 3 | verification |
//! | 4 | fees — the contract is fully running again |
//!
//! The tests cover the phase transitions, the rollback between phases, the
//! rejection of an unknown phase and the `PausePhaseChanged` event.

#![cfg(test)]

use crate::errors::ContractError;
use crate::events::PausePhaseChangedEvent;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::{PauseState, ProjectRegistrationParams};
use crate::DongleContractClient;
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events},
    Address, Env, IntoVal, String, TryFromVal, TryIntoVal, Val, Vec,
};

fn setup(env: &Env) -> (DongleContractClient<'_>, Address, u64, Address) {
    let (client, admin) = setup_contract(env);
    let owner = Address::generate(env);
    let project_id = create_test_project(&client, &owner, "PhaseProject");
    (client, admin, project_id, owner)
}

fn try_generic(
    client: &DongleContractClient<'_>,
    owner: &Address,
) -> Result<
    Result<u64, soroban_sdk::ConversionError>,
    Result<ContractError, soroban_sdk::InvokeError>,
> {
    client
        .mock_all_auths()
        .try_register_project(&ProjectRegistrationParams {
            owner: owner.clone(),
            name: String::from_str(&client.env, "PhaseProjectTwo"),
            slug: String::from_str(&client.env, "phase-project-two"),
            description: String::from_str(&client.env, "Another project"),
            category: String::from_str(&client.env, "DeFi"),
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
        })
}

fn decode_event<T: soroban_sdk::TryFromVal<Env, Val>>(env: &Env, data: &Val) -> Option<T> {
    TryIntoVal::<_, T>::try_into_val(data, env).ok()
}

fn has_phase_event(env: &Env, expected_phase: u32) -> bool {
    let topics_val = (symbol_short!("CONTRACT"), symbol_short!("PHASE")).into_val(env);
    let topics: Vec<Val> = soroban_sdk::Vec::try_from_val(env, &topics_val).unwrap();
    env.events().all().iter().any(|(_, emitted, data)| {
        emitted == topics
            && decode_event::<PausePhaseChangedEvent>(env, data)
                .map(|e| e.phase == expected_phase)
                .unwrap_or(false)
    })
}

#[test]
fn test_phase_zero_keeps_everything_paused() {
    let env = Env::default();
    let (client, admin, _project_id, owner) = setup(&env);

    client.mock_all_auths().pause(&admin);
    client.mock_all_auths().resume_operations_phase(&admin, &0u32);

    assert_eq!(client.get_resume_phase(), 0);
    assert_eq!(client.get_pause_state(), PauseState::PausedAll);
    assert_eq!(
        try_generic(&client, &owner),
        Err(Ok(ContractError::ContractPaused))
    );
}

#[test]
fn test_phase_one_resumes_read_operations() {
    let env = Env::default();
    let (client, admin, project_id, _owner) = setup(&env);

    client.mock_all_auths().pause(&admin);
    client.mock_all_auths().resume_operations_phase(&admin, &1u32);

    assert_eq!(client.get_resume_phase(), 1);
    // Reads keep working during the whole recovery.
    assert!(client.get_project(&project_id).is_some());
}

#[test]
fn test_phase_two_resumes_reviews_only() {
    let env = Env::default();
    let (client, admin, _project_id, owner) = setup(&env);

    client.mock_all_auths().pause(&admin);
    client.mock_all_auths().resume_operations_phase(&admin, &2u32);

    assert_eq!(client.get_resume_phase(), 2);
    assert_eq!(client.get_pause_state(), PauseState::PausedVerificationAndFees);
    // Generic mutations stay closed until the final phase.
    assert_eq!(
        try_generic(&client, &owner),
        Err(Ok(ContractError::ContractPaused))
    );
}

#[test]
fn test_phase_three_resumes_verification() {
    let env = Env::default();
    let (client, admin, _project_id, owner) = setup(&env);

    client.mock_all_auths().pause(&admin);
    client.mock_all_auths().resume_operations_phase(&admin, &3u32);

    assert_eq!(client.get_resume_phase(), 3);
    assert_eq!(client.get_pause_state(), PauseState::PausedFees);
    assert_eq!(
        try_generic(&client, &owner),
        Err(Ok(ContractError::ContractPaused))
    );
}

#[test]
fn test_final_phase_resumes_everything() {
    let env = Env::default();
    let (client, admin, _project_id, owner) = setup(&env);

    client.mock_all_auths().pause(&admin);
    client.mock_all_auths().resume_operations_phase(&admin, &4u32);

    assert_eq!(client.get_resume_phase(), 4);
    assert_eq!(client.get_pause_state(), PauseState::Running);
    assert!(!client.is_paused());
    assert_ne!(
        try_generic(&client, &owner),
        Err(Ok(ContractError::ContractPaused))
    );
}

#[test]
fn test_invalid_phase_is_rejected() {
    let env = Env::default();
    let (client, admin, _project_id, _owner) = setup(&env);

    client.mock_all_auths().pause(&admin);

    let result = client
        .mock_all_auths()
        .try_resume_operations_phase(&admin, &5u32);

    assert_eq!(result, Err(Ok(ContractError::InvalidPausePhase)));
    assert_eq!(client.get_resume_phase(), 0);
}

#[test]
fn test_rollback_moves_back_one_phase() {
    let env = Env::default();
    let (client, admin, _project_id, _owner) = setup(&env);

    client.mock_all_auths().pause(&admin);
    client.mock_all_auths().resume_operations_phase(&admin, &3u32);
    assert_eq!(client.get_resume_phase(), 3);

    client.mock_all_auths().rollback_operations_phase(&admin);
    assert_eq!(client.get_resume_phase(), 2);
    assert_eq!(client.get_pause_state(), PauseState::PausedVerificationAndFees);

    client.mock_all_auths().rollback_operations_phase(&admin);
    assert_eq!(client.get_resume_phase(), 1);
}

#[test]
fn test_rollback_without_a_phase_is_rejected() {
    let env = Env::default();
    let (client, admin, _project_id, _owner) = setup(&env);

    client.mock_all_auths().pause(&admin);

    let result = client.mock_all_auths().try_rollback_operations_phase(&admin);

    assert_eq!(result, Err(Ok(ContractError::InvalidPhaseRollback)));
}

#[test]
fn test_phase_history_is_recorded() {
    let env = Env::default();
    let (client, admin, _project_id, _owner) = setup(&env);

    client.mock_all_auths().pause(&admin);
    client.mock_all_auths().resume_operations_phase(&admin, &1u32);
    client.mock_all_auths().resume_operations_phase(&admin, &2u32);
    client.mock_all_auths().rollback_operations_phase(&admin);

    let history = client.get_phase_history();
    assert_eq!(history.len(), 3);
    assert_eq!(history.get(0), Some(1u32));
    assert_eq!(history.get(1), Some(2u32));
    assert_eq!(history.get(2), Some(1u32));
}

#[test]
fn test_phase_change_event_is_emitted() {
    let env = Env::default();
    let (client, admin, _project_id, _owner) = setup(&env);

    client.mock_all_auths().pause(&admin);
    client.mock_all_auths().resume_operations_phase(&admin, &2u32);

    assert!(has_phase_event(&env, 2u32));
}

#[test]
fn test_resume_phase_requires_admin() {
    let env = Env::default();
    let (client, _admin, _project_id, _owner) = setup(&env);
    let stranger = Address::generate(&env);

    let result = client
        .mock_all_auths()
        .try_resume_operations_phase(&stranger, &1u32);

    assert!(result.is_err(), "a non-admin must not be able to resume");
}
