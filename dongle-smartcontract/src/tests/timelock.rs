use crate::constants::TIMELOCK_MIN_DELAY;
use crate::ContractError;
use crate::DongleContract;
use crate::DongleContractClient;
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{Address, Env};

fn setup(env: &Env) -> (DongleContractClient<'_>, Address) {
    let contract_id = env.register(DongleContract, ());
    let client = DongleContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.mock_all_auths().initialize(&admin);
    (client, admin)
}

fn fast_forward(env: &Env, seconds: u64) {
    let current = env.ledger().timestamp();
    env.ledger().set_timestamp(current + seconds);
}

#[test]
fn test_schedule_set_fee() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let treasury = Address::generate(&env);

    let execution_time = env.ledger().timestamp() + TIMELOCK_MIN_DELAY + 1000;
    let action_id = client.mock_all_auths().schedule_set_fee(
        &admin,
        &None,
        &1000u128,
        &500u128,
        &treasury,
        &execution_time,
    );

    let action = client.get_scheduled_action(&action_id).unwrap();
    assert!(!action.executed);
    assert!(!action.cancelled);
    assert_eq!(action.execution_timestamp, execution_time);
    assert_eq!(action.admin, admin);

    assert_eq!(client.get_scheduled_action_count(), 1);
}

#[test]
fn test_schedule_with_insufficient_delay_returns_error() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let treasury = Address::generate(&env);
    let execution_time = env.ledger().timestamp() + TIMELOCK_MIN_DELAY - 1;

    let result = client.mock_all_auths().try_schedule_set_fee(
        &admin,
        &None,
        &1000u128,
        &500u128,
        &treasury,
        &execution_time,
    );

    assert_eq!(result, Err(Ok(ContractError::InvalidInput)));
}

#[test]
fn test_cancel_nonexistent_action_returns_error() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    let result = client
        .mock_all_auths()
        .try_cancel_scheduled_action(&admin, &999u64);

    assert_eq!(result, Err(Ok(ContractError::InvalidStatus)));
}

#[test]
fn test_schedule_add_admin() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let new_admin = Address::generate(&env);

    let execution_time = env.ledger().timestamp() + TIMELOCK_MIN_DELAY + 1000;
    let action_id = client
        .mock_all_auths()
        .schedule_add_admin(&admin, &new_admin, &execution_time);

    let action = client.get_scheduled_action(&action_id).unwrap();
    assert!(!action.executed);
    assert!(!action.cancelled);
    assert_eq!(action.execution_timestamp, execution_time);
}

#[test]
fn test_schedule_remove_admin() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let new_admin = Address::generate(&env);
    client.mock_all_auths().add_admin(&admin, &new_admin);

    let execution_time = env.ledger().timestamp() + TIMELOCK_MIN_DELAY + 1000;
    let action_id =
        client
            .mock_all_auths()
            .schedule_remove_admin(&admin, &new_admin, &execution_time);

    let action = client.get_scheduled_action(&action_id).unwrap();
    assert!(!action.executed);
    assert!(!action.cancelled);
}

#[test]
fn test_cancel_scheduled_action() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let treasury = Address::generate(&env);

    let execution_time = env.ledger().timestamp() + TIMELOCK_MIN_DELAY + 1000;
    let action_id = client.mock_all_auths().schedule_set_fee(
        &admin,
        &None,
        &1000u128,
        &500u128,
        &treasury,
        &execution_time,
    );

    client
        .mock_all_auths()
        .cancel_scheduled_action(&admin, &action_id);

    let action = client.get_scheduled_action(&action_id).unwrap();
    assert!(action.cancelled);
    assert!(!action.executed);
}

#[test]
fn test_cancel_already_cancelled_fails() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let treasury = Address::generate(&env);

    let execution_time = env.ledger().timestamp() + TIMELOCK_MIN_DELAY + 1000;
    let action_id = client.mock_all_auths().schedule_set_fee(
        &admin,
        &None,
        &1000u128,
        &500u128,
        &treasury,
        &execution_time,
    );

    client
        .mock_all_auths()
        .cancel_scheduled_action(&admin, &action_id);
    let result = client
        .mock_all_auths()
        .try_cancel_scheduled_action(&admin, &action_id);
    assert_eq!(result, Err(Ok(ContractError::InvalidStatus)));
}

#[test]
fn test_early_execute_fails() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let treasury = Address::generate(&env);

    let execution_time = env.ledger().timestamp() + TIMELOCK_MIN_DELAY + 1000;
    let action_id = client.mock_all_auths().schedule_set_fee(
        &admin,
        &None,
        &1000u128,
        &500u128,
        &treasury,
        &execution_time,
    );

    let result = client
        .mock_all_auths()
        .try_execute_scheduled_set_fee(&admin, &action_id);
    assert_eq!(result, Err(Ok(ContractError::TimelockNotExpired)));
}

#[test]
fn test_execute_twice_fails() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let treasury = Address::generate(&env);

    let execution_time = env.ledger().timestamp() + TIMELOCK_MIN_DELAY + 1000;
    let action_id = client.mock_all_auths().schedule_set_fee(
        &admin,
        &None,
        &1000u128,
        &500u128,
        &treasury,
        &execution_time,
    );

    fast_forward(&env, TIMELOCK_MIN_DELAY + 2000);

    client
        .mock_all_auths()
        .execute_scheduled_set_fee(&admin, &action_id);
    let result = client
        .mock_all_auths()
        .try_execute_scheduled_set_fee(&admin, &action_id);
    assert_eq!(result, Err(Ok(ContractError::InvalidStatus)));
}

#[test]
fn test_successful_execute_set_fee() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let treasury = Address::generate(&env);

    let execution_time = env.ledger().timestamp() + TIMELOCK_MIN_DELAY + 1000;
    let action_id = client.mock_all_auths().schedule_set_fee(
        &admin,
        &None,
        &1000u128,
        &500u128,
        &treasury,
        &execution_time,
    );

    fast_forward(&env, TIMELOCK_MIN_DELAY + 2000);

    client
        .mock_all_auths()
        .execute_scheduled_set_fee(&admin, &action_id);

    let action = client.get_scheduled_action(&action_id).unwrap();
    assert!(action.executed);
    assert!(!action.cancelled);

    let config = client.get_fee_config();
    assert_eq!(config.verification_fee, 1000);
    assert_eq!(config.registration_fee, 500);
}

#[test]
fn test_successful_execute_add_admin() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let new_admin = Address::generate(&env);

    let execution_time = env.ledger().timestamp() + TIMELOCK_MIN_DELAY + 1000;
    let action_id = client
        .mock_all_auths()
        .schedule_add_admin(&admin, &new_admin, &execution_time);

    fast_forward(&env, TIMELOCK_MIN_DELAY + 2000);

    client
        .mock_all_auths()
        .execute_scheduled_add_admin(&admin, &action_id);

    let action = client.get_scheduled_action(&action_id).unwrap();
    assert!(action.executed);

    assert!(client.is_admin(&new_admin));
}

#[test]
fn test_successful_execute_remove_admin() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let admin2 = Address::generate(&env);
    client.mock_all_auths().add_admin(&admin, &admin2);

    let execution_time = env.ledger().timestamp() + TIMELOCK_MIN_DELAY + 1000;
    let action_id = client
        .mock_all_auths()
        .schedule_remove_admin(&admin, &admin2, &execution_time);

    fast_forward(&env, TIMELOCK_MIN_DELAY + 2000);

    client
        .mock_all_auths()
        .execute_scheduled_remove_admin(&admin, &action_id);

    assert!(!client.is_admin(&admin2));
}

#[test]
fn test_list_scheduled_actions() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let treasury = Address::generate(&env);

    let execution_time = env.ledger().timestamp() + TIMELOCK_MIN_DELAY + 1000;
    let id1 = client.mock_all_auths().schedule_set_fee(
        &admin,
        &None,
        &1000u128,
        &500u128,
        &treasury,
        &execution_time,
    );
    let id2 = client.mock_all_auths().schedule_set_fee(
        &admin,
        &None,
        &2000u128,
        &1000u128,
        &treasury,
        &(execution_time + 1000),
    );

    let actions = client.list_scheduled_actions(&0u32, &10u32);
    assert_eq!(actions.len(), 2);
    assert_eq!(actions.get(0).unwrap().id, id1);
    assert_eq!(actions.get(1).unwrap().id, id2);
}

#[test]
fn test_list_scheduled_actions_empty() {
    let env = Env::default();
    let (client, _admin) = setup(&env);

    let actions = client.list_scheduled_actions(&0u32, &10u32);
    assert_eq!(actions.len(), 0);
    assert_eq!(client.get_scheduled_action_count(), 0);
}

#[test]
fn test_get_nonexistent_action() {
    let env = Env::default();
    let (client, _admin) = setup(&env);

    let action = client.get_scheduled_action(&999u64);
    assert!(action.is_none());
}

#[test]
fn test_cancel_before_execute_allows_replacement() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let treasury = Address::generate(&env);

    let execution_time = env.ledger().timestamp() + TIMELOCK_MIN_DELAY + 1000;
    let action_id = client.mock_all_auths().schedule_set_fee(
        &admin,
        &None,
        &1000u128,
        &500u128,
        &treasury,
        &execution_time,
    );

    client
        .mock_all_auths()
        .cancel_scheduled_action(&admin, &action_id);

    let new_treasury = Address::generate(&env);
    let action_id2 = client.mock_all_auths().schedule_set_fee(
        &admin,
        &None,
        &2000u128,
        &1000u128,
        &new_treasury,
        &(execution_time + 2000),
    );

    fast_forward(&env, TIMELOCK_MIN_DELAY + 3000);

    client
        .mock_all_auths()
        .execute_scheduled_set_fee(&admin, &action_id2);

    let config = client.get_fee_config();
    assert_eq!(config.verification_fee, 2000);
}

/// Cancellation must prevent subsequent execution of the same action.
#[test]
fn test_cancel_set_fee_prevents_execution() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let treasury = Address::generate(&env);

    let execution_time = env.ledger().timestamp() + TIMELOCK_MIN_DELAY + 1000;
    let action_id = client.mock_all_auths().schedule_set_fee(
        &admin,
        &None,
        &9999u128,
        &1111u128,
        &treasury,
        &execution_time,
    );

    // Cancel before the timelock expires.
    client
        .mock_all_auths()
        .cancel_scheduled_action(&admin, &action_id);

    // Fast-forward past the execution timestamp so delay is satisfied.
    fast_forward(&env, TIMELOCK_MIN_DELAY + 2000);

    // Execution must be rejected because the action is cancelled.
    let result = client
        .mock_all_auths()
        .try_execute_scheduled_set_fee(&admin, &action_id);
    assert_eq!(result, Err(Ok(ContractError::InvalidStatus)));
    // The action is marked cancelled — verify this directly.
    let action = client.get_scheduled_action(&action_id).unwrap();
    assert!(action.cancelled);
    assert!(!action.executed);
}

/// Cancellation must prevent execution of a scheduled add_admin action.
#[test]
fn test_cancel_add_admin_prevents_execution() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let new_admin = Address::generate(&env);

    let execution_time = env.ledger().timestamp() + TIMELOCK_MIN_DELAY + 1000;
    let action_id = client
        .mock_all_auths()
        .schedule_add_admin(&admin, &new_admin, &execution_time);

    client
        .mock_all_auths()
        .cancel_scheduled_action(&admin, &action_id);

    fast_forward(&env, TIMELOCK_MIN_DELAY + 2000);

    let result = client
        .mock_all_auths()
        .try_execute_scheduled_add_admin(&admin, &action_id);
    assert_eq!(result, Err(Ok(ContractError::InvalidStatus)));

    // new_admin must NOT have been added.
    assert!(!client.is_admin(&new_admin));
}

/// Cancellation must prevent execution of a scheduled remove_admin action.
#[test]
fn test_cancel_remove_admin_prevents_execution() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let admin2 = Address::generate(&env);
    client.mock_all_auths().add_admin(&admin, &admin2);
    assert!(client.is_admin(&admin2));

    let execution_time = env.ledger().timestamp() + TIMELOCK_MIN_DELAY + 1000;
    let action_id = client
        .mock_all_auths()
        .schedule_remove_admin(&admin, &admin2, &execution_time);

    client
        .mock_all_auths()
        .cancel_scheduled_action(&admin, &action_id);

    fast_forward(&env, TIMELOCK_MIN_DELAY + 2000);

    let result = client
        .mock_all_auths()
        .try_execute_scheduled_remove_admin(&admin, &action_id);
    assert_eq!(result, Err(Ok(ContractError::InvalidStatus)));

    // admin2 must still be an admin.
    assert!(client.is_admin(&admin2));
}

/// schedule_set_fee with one second less than the minimum delay must fail.
/// This complements the existing test and makes the boundary explicit.
#[test]
fn test_schedule_set_fee_one_below_min_delay_rejected() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let treasury = Address::generate(&env);
    // One second below the minimum — must be rejected.
    let execution_time = env.ledger().timestamp() + TIMELOCK_MIN_DELAY - 1;

    let result = client.mock_all_auths().try_schedule_set_fee(
        &admin,
        &None,
        &1000u128,
        &500u128,
        &treasury,
        &execution_time,
    );
    assert_eq!(result, Err(Ok(ContractError::InvalidInput)));
}

/// schedule_set_fee at exactly the minimum delay boundary must succeed.
#[test]
fn test_schedule_set_fee_at_min_delay_accepted() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let treasury = Address::generate(&env);
    // Exactly at the minimum — must be accepted.
    let execution_time = env.ledger().timestamp() + TIMELOCK_MIN_DELAY;

    let action_id = client.mock_all_auths().schedule_set_fee(
        &admin,
        &None,
        &1000u128,
        &500u128,
        &treasury,
        &execution_time,
    );
    let action = client.get_scheduled_action(&action_id).unwrap();
    assert!(!action.executed);
    assert!(!action.cancelled);
}

// ---------------------------------------------------------------------------
// Issue #471 — minimum-delay enforcement on every scheduling entry point.
//
// `validate_timelock` already rejects a too-soon `execution_timestamp`, and
// `schedule_set_fee` had boundary coverage. `schedule_add_admin` and
// `schedule_remove_admin` did not, even though bypassing the delay on an admin
// change is the more dangerous of the two: it is how an attacker would grant
// themselves an admin key without the community getting a day's notice.
//
// These pin the boundary on both, so a future refactor cannot quietly drop the
// check from one scheduler while `set_fee` keeps the suite green.
// ---------------------------------------------------------------------------

#[test]
fn test_schedule_add_admin_one_below_min_delay_rejected() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let new_admin = Address::generate(&env);
    let execution_time = env.ledger().timestamp() + TIMELOCK_MIN_DELAY - 1;

    let result =
        client
            .mock_all_auths()
            .try_schedule_add_admin(&admin, &new_admin, &execution_time);

    assert_eq!(result, Err(Ok(ContractError::InvalidInput)));
}

#[test]
fn test_schedule_add_admin_at_min_delay_accepted() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let new_admin = Address::generate(&env);
    let execution_time = env.ledger().timestamp() + TIMELOCK_MIN_DELAY;

    let action_id = client
        .mock_all_auths()
        .schedule_add_admin(&admin, &new_admin, &execution_time);

    let action = client.get_scheduled_action(&action_id).unwrap();
    assert_eq!(action.execution_timestamp, execution_time);
}

#[test]
fn test_schedule_remove_admin_one_below_min_delay_rejected() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let target = Address::generate(&env);
    client.mock_all_auths().add_admin(&admin, &target);
    let execution_time = env.ledger().timestamp() + TIMELOCK_MIN_DELAY - 1;

    let result =
        client
            .mock_all_auths()
            .try_schedule_remove_admin(&admin, &target, &execution_time);

    assert_eq!(result, Err(Ok(ContractError::InvalidInput)));
}

#[test]
fn test_schedule_remove_admin_at_min_delay_accepted() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let target = Address::generate(&env);
    client.mock_all_auths().add_admin(&admin, &target);
    let execution_time = env.ledger().timestamp() + TIMELOCK_MIN_DELAY;

    let action_id = client
        .mock_all_auths()
        .schedule_remove_admin(&admin, &target, &execution_time);

    let action = client.get_scheduled_action(&action_id).unwrap();
    assert_eq!(action.execution_timestamp, execution_time);
}

#[test]
fn test_schedule_in_the_past_rejected_on_every_scheduler() {
    // A timestamp already behind the ledger is the crudest bypass attempt.
    let env = Env::default();
    let (client, admin) = setup(&env);
    fast_forward(&env, TIMELOCK_MIN_DELAY * 2);
    let past = env.ledger().timestamp() - 1;
    let target = Address::generate(&env);
    let treasury = Address::generate(&env);

    assert_eq!(
        client
            .mock_all_auths()
            .try_schedule_add_admin(&admin, &target, &past),
        Err(Ok(ContractError::InvalidInput))
    );
    assert_eq!(
        client
            .mock_all_auths()
            .try_schedule_set_fee(&admin, &None, &1u128, &1u128, &treasury, &past),
        Err(Ok(ContractError::InvalidInput))
    );
}
