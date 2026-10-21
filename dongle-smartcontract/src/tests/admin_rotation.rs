//! Tests for the automatic admin-key rotation schedule — issue #900.

use crate::errors::ContractError;
use crate::tests::fixtures::setup_contract;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env, String, Vec,
};

const DAY: u64 = 86_400;

#[test]
fn test_invalid_interval_is_rejected() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let pool: Vec<Address> = Vec::new(&env);
    let result = client
        .mock_all_auths()
        .try_set_rotation_schedule(&admin, &45, &pool);
    assert_eq!(result, Err(Ok(ContractError::InvalidRotationInterval)));
}

#[test]
fn test_schedule_is_installed() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let candidate = Address::generate(&env);

    let mut pool: Vec<Address> = Vec::new(&env);
    pool.push_back(candidate.clone());

    let schedule = client
        .mock_all_auths()
        .set_rotation_schedule(&admin, &60, &pool);
    assert!(schedule.enabled);
    assert_eq!(schedule.interval_days, 60);
    assert_eq!(schedule.rotation_count, 0);
    assert!(!schedule.preflight_confirmed);

    let stored = client.get_rotation_schedule().unwrap();
    assert_eq!(stored.interval_days, 60);
    assert_eq!(stored.replacement_pool.len(), 1);
}

#[test]
fn test_candidate_that_is_already_admin_is_rejected() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let mut pool: Vec<Address> = Vec::new(&env);
    pool.push_back(admin.clone());

    let result = client
        .mock_all_auths()
        .try_set_rotation_schedule(&admin, &30, &pool);
    assert_eq!(result, Err(Ok(ContractError::InvalidProjectData)));
}

#[test]
fn test_rotation_cannot_run_before_it_is_due() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let second_admin = Address::generate(&env);
    let candidate = Address::generate(&env);

    client.mock_all_auths().add_admin(&admin, &second_admin);

    let mut pool: Vec<Address> = Vec::new(&env);
    pool.push_back(candidate.clone());
    client
        .mock_all_auths()
        .set_rotation_schedule(&admin, &30, &pool);

    let result = client
        .mock_all_auths()
        .try_execute_scheduled_rotation(&admin);
    assert_eq!(result, Err(Ok(ContractError::RotationNotDue)));
}

#[test]
fn test_rotation_requires_preflight_confirmation() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let second_admin = Address::generate(&env);
    let candidate = Address::generate(&env);

    client.mock_all_auths().add_admin(&admin, &second_admin);

    let mut pool: Vec<Address> = Vec::new(&env);
    pool.push_back(candidate.clone());
    client
        .mock_all_auths()
        .set_rotation_schedule(&admin, &30, &pool);

    // Move past the interval so the rotation is due.
    let now = env.ledger().timestamp();
    env.ledger().set_timestamp(now + 31 * DAY);

    let result = client
        .mock_all_auths()
        .try_execute_scheduled_rotation(&admin);
    assert_eq!(
        result,
        Err(Ok(ContractError::RotationPreflightNotConfirmed))
    );
}

#[test]
fn test_rotation_removes_oldest_admin_and_promotes_replacement() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let second_admin = Address::generate(&env);
    let candidate = Address::generate(&env);

    client.mock_all_auths().add_admin(&admin, &second_admin);

    let mut pool: Vec<Address> = Vec::new(&env);
    pool.push_back(candidate.clone());
    client
        .mock_all_auths()
        .set_rotation_schedule(&admin, &30, &pool);

    // The first admin joined before the second one, so it rotates out.
    let now = env.ledger().timestamp();
    env.ledger().set_timestamp(now + 31 * DAY);

    let checklist = client
        .mock_all_auths()
        .get_rotation_preflight_checklist(&admin);
    assert_eq!(checklist.len(), 6);
    assert!(checklist.iter().all(|item| item.satisfied));

    let schedule = client.mock_all_auths().confirm_rotation_preflight(&admin);
    assert!(schedule.preflight_confirmed);

    let record = client.mock_all_auths().execute_scheduled_rotation(&admin);
    assert_eq!(record.removed_admins.len(), 1);
    assert_eq!(record.removed_admins.get(0), Some(admin.clone()));
    assert_eq!(record.added_admins.len(), 1);
    assert_eq!(record.added_admins.get(0), Some(candidate.clone()));

    // The admin set has been rotated.
    assert!(!client.is_admin(&admin));
    assert!(client.is_admin(&candidate));
    assert!(client.is_admin(&second_admin));

    // Notifications were emitted for both affected key holders.
    let notifications = client.get_admin_rotation_notifications(&record.rotation_id);
    assert_eq!(notifications.len(), 2);

    // The schedule is re-armed and needs a new confirmation.
    let schedule = client.get_rotation_schedule().unwrap();
    assert_eq!(schedule.rotation_count, 1);
    assert!(!schedule.preflight_confirmed);
    assert!(schedule.next_rotation_at > env.ledger().timestamp());
}

#[test]
fn test_rotation_never_removes_the_last_admin() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let candidate = Address::generate(&env);

    let mut pool: Vec<Address> = Vec::new(&env);
    pool.push_back(candidate.clone());
    client
        .mock_all_auths()
        .set_rotation_schedule(&admin, &30, &pool);

    let now = env.ledger().timestamp();
    env.ledger().set_timestamp(now + 31 * DAY);

    // The checklist flags the single-admin situation, so it cannot be
    // confirmed.
    let checklist = client
        .mock_all_auths()
        .get_rotation_preflight_checklist(&admin);
    assert!(checklist
        .iter()
        .any(|item| item.code == String::from_str(&env, "more_than_one_admin") && !item.satisfied));

    let result = client
        .mock_all_auths()
        .try_confirm_rotation_preflight(&admin);
    assert_eq!(
        result,
        Err(Ok(ContractError::RotationPreflightChecklistIncomplete))
    );

    // And even a forced execution refuses to brick the contract.
    let result = client
        .mock_all_auths()
        .try_execute_scheduled_rotation(&admin);
    assert!(result.is_err());
    assert!(client.is_admin(&admin));
}
