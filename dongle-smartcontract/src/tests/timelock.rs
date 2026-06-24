use crate::constants::TIMELOCK_MIN_DELAY;
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
#[should_panic(expected = "Timelock: action already cancelled")]
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
    client
        .mock_all_auths()
        .cancel_scheduled_action(&admin, &action_id);
}

#[test]
#[should_panic(expected = "Timelock: action cannot execute before timelock expires")]
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

    client
        .mock_all_auths()
        .execute_scheduled_set_fee(&admin, &action_id);
}

#[test]
#[should_panic(expected = "Timelock: action already executed")]
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

    client
        .mock_all_auths()
        .execute_scheduled_set_fee(&admin, &action_id);
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
