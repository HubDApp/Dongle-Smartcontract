//! Tests for the project archive lifecycle (#873): reasons, audit log,
//! bulk archive, and exclusion of archived projects from listings.

use crate::errors::ContractError;
use crate::types::{ArchiveAction, ArchiveReason};
use soroban_sdk::{testutils::Address as _, vec, Address, Env, Vec};

use super::fixtures::{create_test_project, setup_contract};

#[test]
fn test_archive_with_reason_tracks_reason_and_audit() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Reason Project");

    client.mock_all_auths().archive_project_with_reason(
        &project_id,
        &owner,
        &ArchiveReason::Migration,
    );

    assert_eq!(
        client.get_archive_reason(&project_id),
        Some(ArchiveReason::Migration)
    );
    let log = client.get_archive_audit_log(&project_id, &0, &10);
    assert_eq!(log.len(), 1);
    let entry = log.get(0).unwrap();
    assert_eq!(entry.action, ArchiveAction::Archived);
    assert_eq!(entry.caller, owner);
    assert_eq!(entry.reason, Some(ArchiveReason::Migration));
}

#[test]
fn test_reactivate_clears_reason_and_appends_audit() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Reactivate Project");

    client.mock_all_auths().archive_project_with_reason(
        &project_id,
        &owner,
        &ArchiveReason::Maintenance,
    );
    client
        .mock_all_auths()
        .reactivate_project(&project_id, &owner);

    assert_eq!(client.get_archive_reason(&project_id), None);
    let log = client.get_archive_audit_log(&project_id, &0, &10);
    assert_eq!(log.len(), 2);
    assert_eq!(log.get(1).unwrap().action, ArchiveAction::Reactivated);
    assert_eq!(log.get(1).unwrap().reason, None);
}

#[test]
fn test_plain_archive_records_audit_without_reason() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Plain Project");

    client.mock_all_auths().archive_project(&project_id, &owner);

    let log = client.get_archive_audit_log(&project_id, &0, &10);
    assert_eq!(log.len(), 1);
    assert_eq!(log.get(0).unwrap().reason, None);
    assert_eq!(client.get_archive_reason(&project_id), None);
}

#[test]
fn test_bulk_archive_projects() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let a = create_test_project(&client, &owner, "Bulk A");
    let b = create_test_project(&client, &owner, "Bulk B");

    let archived = client.mock_all_auths().bulk_archive_projects(
        &owner,
        &vec![&env, a, b],
        &ArchiveReason::Discontinuation,
    );

    assert_eq!(archived, 2);
    assert!(client.get_project(&a).unwrap().archived);
    assert!(client.get_project(&b).unwrap().archived);
    assert_eq!(
        client.get_archive_reason(&b),
        Some(ArchiveReason::Discontinuation)
    );
}

#[test]
fn test_bulk_archive_is_atomic_on_unauthorized_project() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let other = Address::generate(&env);
    let mine = create_test_project(&client, &owner, "Mine");
    let theirs = create_test_project(&client, &other, "Theirs");

    let result = client.mock_all_auths().try_bulk_archive_projects(
        &owner,
        &vec![&env, mine, theirs],
        &ArchiveReason::Maintenance,
    );

    assert_eq!(result, Err(Ok(ContractError::Unauthorized)));
    assert!(!client.get_project(&mine).unwrap().archived);
}

#[test]
fn test_bulk_archive_rejects_empty_batch() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let result = client.mock_all_auths().try_bulk_archive_projects(
        &owner,
        &Vec::new(&env),
        &ArchiveReason::Maintenance,
    );

    assert_eq!(result, Err(Ok(ContractError::InvalidInput)));
}

#[test]
fn test_archived_projects_excluded_from_listings() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let kept = create_test_project(&client, &owner, "Kept");
    let gone = create_test_project(&client, &owner, "Gone");

    client.mock_all_auths().archive_project_with_reason(
        &gone,
        &owner,
        &ArchiveReason::Discontinuation,
    );

    let listed = client.list_projects(&1, &10);
    assert_eq!(listed.len(), 1);
    assert_eq!(listed.get(0).unwrap().id, kept);
}
