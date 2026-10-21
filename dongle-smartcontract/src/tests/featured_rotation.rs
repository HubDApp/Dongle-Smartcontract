//! Tests for scheduled featured-project rotation (issue #927).

use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use soroban_sdk::{testutils::Address as _, testutils::Ledger, vec, Address, Env};

const DAY: u64 = 24 * 60 * 60;
const START: u64 = 1_000_000;

fn set_time(env: &Env, timestamp: u64) {
    env.ledger().with_mut(|li| li.timestamp = timestamp);
}

#[test]
fn test_schedule_rotation_admin_only() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let id = create_test_project(&client, &owner, "Alpha");
    let non_admin = Address::generate(&env);

    let result = client.try_schedule_featured_rotation(&non_admin, &vec![&env, id], &7);
    assert_eq!(result, Err(Ok(ContractError::AdminOnly)));
}

#[test]
fn test_schedule_rotation_validates_input() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let id = create_test_project(&client, &owner, "Alpha");

    // Empty project list.
    let result = client.try_schedule_featured_rotation(&admin, &vec![&env], &7);
    assert_eq!(result, Err(Ok(ContractError::InvalidInput)));

    // Zero and over-long periods.
    let result = client.try_schedule_featured_rotation(&admin, &vec![&env, id], &0);
    assert_eq!(result, Err(Ok(ContractError::InvalidInput)));
    let result = client.try_schedule_featured_rotation(&admin, &vec![&env, id], &366);
    assert_eq!(result, Err(Ok(ContractError::InvalidInput)));

    // Duplicate project.
    let result = client.try_schedule_featured_rotation(&admin, &vec![&env, id, id], &7);
    assert_eq!(result, Err(Ok(ContractError::InvalidInput)));

    // Unknown project.
    let result = client.try_schedule_featured_rotation(&admin, &vec![&env, id, 999], &7);
    assert_eq!(result, Err(Ok(ContractError::ProjectNotFound)));
}

#[test]
fn test_schedule_rotation_rejects_archived_project() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let id = create_test_project(&client, &owner, "Alpha");
    client.archive_project(&id, &owner);

    let result = client.try_schedule_featured_rotation(&admin, &vec![&env, id], &7);
    assert_eq!(result, Err(Ok(ContractError::InvalidInput)));
}

#[test]
fn test_no_schedule_returns_none() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);

    assert_eq!(client.get_featured_rotation_schedule(), None);
    assert_eq!(client.get_current_rotation_project(), None);
    assert_eq!(
        client.try_advance_featured_rotation(),
        Err(Ok(ContractError::InvalidStatus))
    );
}

#[test]
fn test_first_project_featured_immediately() {
    let env = Env::default();
    env.mock_all_auths();
    set_time(&env, START);
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let a = create_test_project(&client, &owner, "Alpha");
    let b = create_test_project(&client, &owner, "Beta");

    let schedule = client.schedule_featured_rotation(&admin, &vec![&env, a, b], &7);
    assert_eq!(schedule.started_at, START);
    assert_eq!(schedule.current_index, 0);
    assert_eq!(schedule.current_project_id, a);
    assert_eq!(schedule.next_rotation_at, START + 7 * DAY);
    assert_eq!(schedule.rotations_completed, 0);
    assert_eq!(client.get_current_rotation_project(), Some(a));
}

#[test]
fn test_rotation_timing_boundaries() {
    let env = Env::default();
    env.mock_all_auths();
    set_time(&env, START);
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let a = create_test_project(&client, &owner, "Alpha");
    let b = create_test_project(&client, &owner, "Beta");

    client.schedule_featured_rotation(&admin, &vec![&env, a, b], &3);

    // One second before the boundary the first project is still featured.
    set_time(&env, START + 3 * DAY - 1);
    assert_eq!(client.get_current_rotation_project(), Some(a));

    // Exactly at the boundary the rotation advances.
    set_time(&env, START + 3 * DAY);
    let schedule = client.get_featured_rotation_schedule().unwrap();
    assert_eq!(schedule.current_project_id, b);
    assert_eq!(schedule.current_index, 1);
    assert_eq!(schedule.rotations_completed, 1);
    assert_eq!(schedule.next_rotation_at, START + 6 * DAY);
}

#[test]
fn test_rotation_order_wraps_around() {
    let env = Env::default();
    env.mock_all_auths();
    set_time(&env, START);
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let a = create_test_project(&client, &owner, "Alpha");
    let b = create_test_project(&client, &owner, "Beta");
    let c = create_test_project(&client, &owner, "Gamma");

    client.schedule_featured_rotation(&admin, &vec![&env, a, b, c], &1);

    let expected = [a, b, c, a, b, c, a];
    for (day, project) in expected.iter().enumerate() {
        set_time(&env, START + day as u64 * DAY);
        assert_eq!(client.get_current_rotation_project(), Some(*project));
    }
}

#[test]
fn test_lazy_evaluation_skips_missed_periods() {
    let env = Env::default();
    env.mock_all_auths();
    set_time(&env, START);
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let a = create_test_project(&client, &owner, "Alpha");
    let b = create_test_project(&client, &owner, "Beta");
    let c = create_test_project(&client, &owner, "Gamma");

    client.schedule_featured_rotation(&admin, &vec![&env, a, b, c], &2);

    // Nobody touches the contract for 11 days (5 full 2-day periods).
    set_time(&env, START + 11 * DAY);
    let schedule = client.get_featured_rotation_schedule().unwrap();
    assert_eq!(schedule.rotations_completed, 5);
    assert_eq!(schedule.current_index, 2); // 5 % 3
    assert_eq!(schedule.current_project_id, c);
    assert_eq!(schedule.next_rotation_at, START + 12 * DAY);
}

#[test]
fn test_advance_persists_current_slot() {
    let env = Env::default();
    env.mock_all_auths();
    set_time(&env, START);
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let a = create_test_project(&client, &owner, "Alpha");
    let b = create_test_project(&client, &owner, "Beta");

    client.schedule_featured_rotation(&admin, &vec![&env, a, b], &7);

    // Before the boundary, advancing is a no-op.
    set_time(&env, START + DAY);
    let schedule = client.advance_featured_rotation();
    assert_eq!(schedule.current_project_id, a);

    set_time(&env, START + 7 * DAY);
    let schedule = client.advance_featured_rotation();
    assert_eq!(schedule.current_project_id, b);
    assert_eq!(schedule.rotations_completed, 1);

    // The evaluated read agrees with the persisted state.
    assert_eq!(client.get_featured_rotation_schedule(), Some(schedule));
}

#[test]
fn test_reschedule_restarts_rotation() {
    let env = Env::default();
    env.mock_all_auths();
    set_time(&env, START);
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let a = create_test_project(&client, &owner, "Alpha");
    let b = create_test_project(&client, &owner, "Beta");
    let c = create_test_project(&client, &owner, "Gamma");

    client.schedule_featured_rotation(&admin, &vec![&env, a, b], &1);
    set_time(&env, START + DAY);
    assert_eq!(client.get_current_rotation_project(), Some(b));

    let schedule = client.schedule_featured_rotation(&admin, &vec![&env, c, a], &5);
    assert_eq!(schedule.started_at, START + DAY);
    assert_eq!(schedule.current_project_id, c);
    assert_eq!(schedule.next_rotation_at, START + 6 * DAY);
}

#[test]
fn test_clear_rotation() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let a = create_test_project(&client, &owner, "Alpha");

    client.schedule_featured_rotation(&admin, &vec![&env, a], &7);

    let non_admin = Address::generate(&env);
    assert_eq!(
        client.try_clear_featured_rotation(&non_admin),
        Err(Ok(ContractError::AdminOnly))
    );

    client.clear_featured_rotation(&admin);
    assert_eq!(client.get_featured_rotation_schedule(), None);
}
