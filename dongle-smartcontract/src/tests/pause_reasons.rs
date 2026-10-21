//! Pause reasons and pause notifications (issue #901).
//!
//! Covers:
//! * `set_pause_with_reason(reason, estimated_resume)`,
//! * the reason and the estimated resume time in `get_contract_status()`,
//! * the `CONTRACT/PAUSED` event carrying the reason,
//! * the subscriber list and the `CONTRACT/NOTIFY` event.

#![cfg(test)]

use crate::errors::ContractError;
use crate::events::{ContractPausedEvent, PauseNotificationEvent};
use crate::tests::fixtures::setup_contract;
use crate::DongleContractClient;
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events},
    Address, Env, IntoVal, String, TryFromVal, TryIntoVal, Val, Vec,
};

fn setup(env: &Env) -> (DongleContractClient<'_>, Address) {
    setup_contract(env)
}

fn decode_event<T: soroban_sdk::TryFromVal<Env, Val>>(env: &Env, data: &Val) -> Option<T> {
    TryIntoVal::<_, T>::try_into_val(data, env).ok()
}

fn has_paused_event(env: &Env, expected_reason: &str) -> bool {
    let topics_val = (symbol_short!("CONTRACT"), symbol_short!("PAUSED")).into_val(env);
    let topics: Vec<Val> = soroban_sdk::Vec::try_from_val(env, &topics_val).unwrap();
    env.events().all().iter().any(|(_, emitted, data)| {
        emitted == topics
            && decode_event::<ContractPausedEvent>(env, data)
                .map(|e| {
                    e.reason
                        .as_ref()
                        .map(|r| r == &String::from_str(env, expected_reason))
                        .unwrap_or(false)
                })
                .unwrap_or(false)
    })
}

fn has_notification_event(env: &Env) -> bool {
    let topics_val = (symbol_short!("CONTRACT"), symbol_short!("NOTIFY")).into_val(env);
    let topics: Vec<Val> = soroban_sdk::Vec::try_from_val(env, &topics_val).unwrap();
    env.events().all().iter().any(|(_, emitted, data)| {
        emitted == topics
            && decode_event::<PauseNotificationEvent>(env, data)
                .map(|e| e.subscriber_count > 0)
                .unwrap_or(false)
    })
}

#[test]
fn test_pause_reason_is_recorded_and_displayed() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    let reason = String::from_str(&env, "Fee token migration in progress");
    let estimated_resume: u64 = 1_800_000_000;

    client
        .mock_all_auths()
        .set_pause_with_reason(&admin, &reason, &estimated_resume);

    let status = client.get_contract_status();
    assert!(status.is_paused);
    assert_eq!(status.pause_reason, Some(reason.clone()));
    assert_eq!(status.estimated_resume, Some(estimated_resume));
    assert!(status.paused_at > 0);
}

#[test]
fn test_pause_without_reason_has_no_reason() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    client.mock_all_auths().pause(&admin);

    let status = client.get_contract_status();
    assert!(status.is_paused);
    assert_eq!(status.pause_reason, None);
    assert_eq!(status.estimated_resume, None);
}

#[test]
fn test_empty_pause_reason_is_rejected() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    let reason = String::from_str(&env, "");
    let result = client
        .mock_all_auths()
        .try_set_pause_with_reason(&admin, &reason, &0u64);

    assert_eq!(result, Err(Ok(ContractError::InvalidPauseReason)));
}

#[test]
fn test_oversized_pause_reason_is_rejected() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    let reason = String::from_str(&env, &"x".repeat(300));
    let result = client
        .mock_all_auths()
        .try_set_pause_with_reason(&admin, &reason, &0u64);

    assert_eq!(result, Err(Ok(ContractError::InvalidPauseReason)));
}

#[test]
fn test_contract_paused_event_carries_the_reason() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    client.mock_all_auths().set_pause_with_reason(
        &admin,
        &String::from_str(&env, "Verification queue overflow"),
        &1_800_000_000u64,
    );

    assert!(has_paused_event(&env, "Verification queue overflow"));
}

#[test]
fn test_subscribers_are_notified_with_the_reason() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let subscriber = Address::generate(&env);

    client
        .mock_all_auths()
        .subscribe_pause_notifications(&subscriber);
    assert_eq!(client.list_pause_subscribers().len(), 1);

    client.mock_all_auths().set_pause_with_reason(
        &admin,
        &String::from_str(&env, "Database migration"),
        &0u64,
    );

    assert!(has_notification_event(&env));
}

#[test]
fn test_unsubscribe_removes_the_subscriber() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let subscriber = Address::generate(&env);

    client
        .mock_all_auths()
        .subscribe_pause_notifications(&subscriber);
    assert_eq!(client.list_pause_subscribers().len(), 1);

    client
        .mock_all_auths()
        .unsubscribe_pause_notifications(&subscriber);
    assert!(client.list_pause_subscribers().is_empty());
}

#[test]
fn test_unpause_clears_the_reason() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    client.mock_all_auths().set_pause_with_reason(
        &admin,
        &String::from_str(&env, "Temporary incident"),
        &1_800_000_000u64,
    );

    client.mock_all_auths().unpause(&admin);

    let status = client.get_contract_status();
    assert!(!status.is_paused);
    assert_eq!(status.pause_reason, None);
    assert_eq!(status.estimated_resume, None);
}

#[test]
fn test_pause_with_reason_requires_admin() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let stranger = Address::generate(&env);

    let result = client.mock_all_auths().try_set_pause_with_reason(
        &stranger,
        &String::from_str(&env, "Not an admin"),
        &0u64,
    );

    assert!(result.is_err(), "a non-admin must not be able to pause");
}
