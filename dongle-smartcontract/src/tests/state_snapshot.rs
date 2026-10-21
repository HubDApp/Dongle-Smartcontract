//! State snapshot tests (#933): scoped exports, paging, hashing, recording,
//! and drift detection.

use crate::constants::{MAX_SNAPSHOT_PAGE_SIZE, STATE_SNAPSHOT_FORMAT_VERSION};
use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::SnapshotScope;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, BytesN, Env, String,
};

const CID: &str = "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG";

#[test]
fn test_full_snapshot_includes_projects_reviews_and_config() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let p1 = create_test_project(&client, &admin, "SnapA");
    let p2 = create_test_project(&client, &admin, "SnapB");
    client.add_review(&p1, &Address::generate(&env), &4, &None);
    client.add_review(&p2, &Address::generate(&env), &5, &None);

    let snap = client.export_contract_state_snapshot(&admin, &SnapshotScope::Full, &0, &0);

    assert_eq!(snap.format_version, STATE_SNAPSHOT_FORMAT_VERSION);
    assert_eq!(snap.project_count, 2);
    assert_eq!(snap.start_project_id, 1);
    assert_eq!(snap.end_project_id, 2);
    assert_eq!(snap.projects.len(), 2);
    assert_eq!(snap.reviews.len(), 2);
    let config = snap.config.unwrap();
    assert!(config.admins.contains(&admin));
    assert!(!config.paused);
}

#[test]
fn test_selective_scopes_only_fill_their_lists() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let p1 = create_test_project(&client, &admin, "Selective");
    client.add_review(&p1, &Address::generate(&env), &3, &None);

    let projects = client.export_contract_state_snapshot(&admin, &SnapshotScope::Projects, &1, &10);
    assert_eq!(projects.projects.len(), 1);
    assert!(projects.reviews.is_empty());
    assert!(projects.config.is_none());

    let reviews = client.export_contract_state_snapshot(&admin, &SnapshotScope::Reviews, &1, &10);
    assert!(reviews.projects.is_empty());
    assert_eq!(reviews.reviews.len(), 1);

    let configs = client.export_contract_state_snapshot(&admin, &SnapshotScope::Configs, &1, &10);
    assert!(configs.projects.is_empty());
    assert!(configs.reviews.is_empty());
    assert!(configs.config.is_some());
    assert_eq!(configs.start_project_id, 0);
    assert_eq!(configs.end_project_id, 0);
}

#[test]
fn test_snapshot_pages_by_project_id() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    for name in ["PageA", "PageB", "PageC"] {
        create_test_project(&client, &admin, name);
    }

    let first = client.export_contract_state_snapshot(&admin, &SnapshotScope::Projects, &1, &2);
    assert_eq!(first.projects.len(), 2);
    assert_eq!(first.end_project_id, 2);

    let second = client.export_contract_state_snapshot(&admin, &SnapshotScope::Projects, &3, &2);
    assert_eq!(second.projects.len(), 1);
    assert_eq!(second.projects.get(0).unwrap().id, 3);

    let past_end = client.export_contract_state_snapshot(&admin, &SnapshotScope::Projects, &10, &2);
    assert!(past_end.projects.is_empty());
    assert!(past_end.end_project_id < past_end.start_project_id);

    let capped = client.export_contract_state_snapshot(
        &admin,
        &SnapshotScope::Projects,
        &1,
        &(MAX_SNAPSHOT_PAGE_SIZE + 50),
    );
    assert_eq!(capped.projects.len(), 3);
}

#[test]
fn test_state_hash_is_stable_and_ignores_timestamp() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    create_test_project(&client, &admin, "HashStable");

    let a = client.export_contract_state_snapshot(&admin, &SnapshotScope::Full, &1, &10);
    env.ledger().with_mut(|l| l.timestamp += 1_000);
    let b = client.export_contract_state_snapshot(&admin, &SnapshotScope::Full, &1, &10);

    assert_ne!(a.generated_at, b.generated_at);
    assert_eq!(a.state_hash, b.state_hash);

    let other_scope = client.export_contract_state_snapshot(&admin, &SnapshotScope::Projects, &1, &10);
    assert_ne!(a.state_hash, other_scope.state_hash);
}

#[test]
fn test_record_and_verify_snapshot_detects_drift() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let p1 = create_test_project(&client, &admin, "Drift");
    let export_sha = BytesN::from_array(&env, &[7u8; 32]);
    let cid = String::from_str(&env, CID);

    let expected = client.export_contract_state_snapshot(&admin, &SnapshotScope::Full, &1, &10);
    let id = client.record_state_snapshot(&admin, &SnapshotScope::Full, &1, &10, &cid, &export_sha);
    assert_eq!(id, 1);
    assert_eq!(client.get_state_snapshot_count(), 1);

    let record = client.get_state_snapshot(&id).unwrap();
    assert_eq!(record.state_hash, expected.state_hash);
    assert_eq!(record.export_sha256, export_sha);
    assert_eq!(record.ipfs_cid, cid);
    assert_eq!(record.published_by, admin);
    assert_eq!(record.published_at, env.ledger().timestamp());

    assert!(client.verify_state_snapshot(&id).matches);

    client.add_review(&p1, &Address::generate(&env), &2, &None);
    let after = client.verify_state_snapshot(&id);
    assert!(!after.matches);
    assert_eq!(after.recorded_hash, expected.state_hash);
}

#[test]
fn test_snapshot_endpoints_require_admin() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let outsider = Address::generate(&env);

    assert_eq!(
        client.try_export_contract_state_snapshot(&outsider, &SnapshotScope::Full, &1, &10),
        Err(Ok(ContractError::AdminOnly.into()))
    );
    assert_eq!(
        client.try_record_state_snapshot(
            &outsider,
            &SnapshotScope::Full,
            &1,
            &10,
            &String::from_str(&env, CID),
            &BytesN::from_array(&env, &[0u8; 32]),
        ),
        Err(Ok(ContractError::AdminOnly.into()))
    );
}

#[test]
fn test_record_rejects_invalid_cid_and_unknown_id() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    assert_eq!(
        client.try_record_state_snapshot(
            &admin,
            &SnapshotScope::Configs,
            &0,
            &0,
            &String::from_str(&env, "not-a-cid"),
            &BytesN::from_array(&env, &[0u8; 32]),
        ),
        Err(Ok(ContractError::InvalidCid.into()))
    );
    assert_eq!(client.get_state_snapshot_count(), 0);
    assert_eq!(
        client.try_verify_state_snapshot(&99),
        Err(Ok(ContractError::SnapshotNotFound.into()))
    );
}
