//! Tests for changelog generation from project events (#929).
//!
//! Each test drives real contract flows, then checks that the generated
//! changelog matches the recorded event stream one-to-one.

use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::{ChangelogEventType, NotificationKind};
use crate::DongleContractClient;
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{Address, Env, String};

const DAY: u64 = 86_400;

fn approve_verification(env: &Env, client: &DongleContractClient<'_>, admin: &Address, owner: &Address, id: u64) {
    let token = env
        .register_stellar_asset_contract_v2(Address::generate(env))
        .address();
    client.set_fee(admin, &Some(token.clone()), &100, &0, admin);
    soroban_sdk::token::StellarAssetClient::new(env, &token).mint(owner, &1000);
    client.pay_fee(owner, &id, &Some(token));
    client.request_verification(
        &id,
        owner,
        &String::from_str(env, "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG"),
    );
    client.approve_verification(&id, admin);
}

fn set_time(env: &Env, timestamp: u64) {
    env.ledger().with_mut(|l| l.timestamp = timestamp);
}

#[test]
fn test_events_are_recorded_for_project_notifications() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let id = create_test_project(&client, &owner, "Alpha");

    assert_eq!(client.get_project_event_count(&id), 0);
    client.archive_project(&id, &owner);
    client.reactivate_project(&id, &owner);

    assert_eq!(client.get_project_event_count(&id), 2);
    assert_eq!(client.get_project_event(&id, &0).unwrap().kind, NotificationKind::ProjectArchived);
    assert_eq!(client.get_project_event(&id, &1).unwrap().kind, NotificationKind::ProjectReactivated);
}

#[test]
fn test_generated_changelog_matches_event_stream() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let id = create_test_project(&client, &owner, "Alpha");

    set_time(&env, 10 * DAY);
    approve_verification(&env, &client, &admin, &owner, id);
    set_time(&env, 11 * DAY);
    client.archive_project(&id, &owner);
    set_time(&env, 12 * DAY);
    client.reactivate_project(&id, &owner);

    let generated = client.generate_changelog_from_events(&id, &0);
    let event_count = client.get_project_event_count(&id);
    assert_eq!(generated as u64, event_count);

    let entries = client.get_generated_changelog(&id, &0, &50);
    assert_eq!(entries.len() as u64, event_count);
    for entry in entries.iter() {
        let source = client.get_project_event(&id, &entry.source_event_seq).unwrap();
        assert_eq!(entry.occurred_at, source.timestamp);
        assert_eq!(entry.project_id, id);
    }
    assert_eq!(entries.get(0).unwrap().event_type, ChangelogEventType::VerificationApproved);
    assert_eq!(entries.get(1).unwrap().event_type, ChangelogEventType::Archived);
    assert_eq!(entries.get(2).unwrap().event_type, ChangelogEventType::Reactivated);
}

#[test]
fn test_generation_is_idempotent_and_batched() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let id = create_test_project(&client, &owner, "Alpha");

    client.archive_project(&id, &owner);
    client.reactivate_project(&id, &owner);
    client.archive_project(&id, &owner);

    assert_eq!(client.generate_changelog_from_events(&id, &2), 2);
    assert_eq!(client.generate_changelog_from_events(&id, &2), 1);
    assert_eq!(client.generate_changelog_from_events(&id, &2), 0);
    assert_eq!(client.get_generated_changelog(&id, &0, &50).len(), 3);
}

#[test]
fn test_summary_groups_by_type_and_window() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let id = create_test_project(&client, &owner, "Alpha");

    set_time(&env, DAY);
    client.archive_project(&id, &owner);
    set_time(&env, 2 * DAY);
    client.reactivate_project(&id, &owner);
    set_time(&env, 3 * DAY);
    client.archive_project(&id, &owner);
    client.generate_changelog_from_events(&id, &0);

    let summary = client.generate_changelog_summary(&id, &0, &(10 * DAY));
    assert_eq!(summary.total_entries, 3);
    assert_eq!(summary.unprocessed_events, 0);
    assert_eq!(summary.by_type.len(), 2);
    let archived = summary.by_type.get(0).unwrap();
    assert_eq!(archived.event_type, ChangelogEventType::Archived);
    assert_eq!(archived.count, 2);
    assert_eq!(summary.by_type.get(1).unwrap().count, 1);

    // Window excludes the event at exactly `end`.
    let narrow = client.generate_changelog_summary(&id, &DAY, &(3 * DAY));
    assert_eq!(narrow.total_entries, 2);

    // A new event shows up as unprocessed until the generator runs again.
    client.reactivate_project(&id, &owner);
    let stale = client.generate_changelog_summary(&id, &0, &(10 * DAY));
    assert_eq!(stale.unprocessed_events, 1);
    assert_eq!(stale.total_entries, 3);
}

#[test]
fn test_group_by_period() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let id = create_test_project(&client, &owner, "Alpha");

    set_time(&env, DAY);
    client.archive_project(&id, &owner);
    set_time(&env, DAY + 60);
    client.reactivate_project(&id, &owner);
    set_time(&env, 3 * DAY);
    client.archive_project(&id, &owner);
    client.generate_changelog_from_events(&id, &0);

    let groups = client.group_changelog_by_period(&id, &0, &(4 * DAY), &DAY);
    assert_eq!(groups.len(), 2);
    assert_eq!(groups.get(0).unwrap().period_start, DAY);
    assert_eq!(groups.get(0).unwrap().count, 2);
    assert_eq!(groups.get(1).unwrap().period_start, 3 * DAY);
    assert_eq!(groups.get(1).unwrap().count, 1);
}

#[test]
fn test_invalid_windows_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let id = create_test_project(&client, &owner, "Alpha");

    assert_eq!(
        client.try_generate_changelog_summary(&id, &5, &5),
        Err(Ok(ContractError::InvalidChangelogWindow))
    );
    assert_eq!(
        client.try_group_changelog_by_period(&id, &0, &DAY, &0),
        Err(Ok(ContractError::InvalidChangelogWindow))
    );
    assert_eq!(
        client.try_generate_changelog_from_events(&999, &0),
        Err(Ok(ContractError::ProjectNotFound))
    );
}
