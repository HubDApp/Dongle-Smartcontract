//! TTL extension and expiration edge cases (closes #953).
//!
//! - TTL extension at maximum batch size (`MAX_TTL_BATCH_SIZE`).
//! - TTL calls for expired/nonexistent entries do not corrupt state.
//! - Batch TTL extension with partial failures reports `skipped_ids`.
//! - TTL refresh still works after pause/unpause cycles.
//! - TTL state persists across sequential calls (contract upgrades keep
//!   ledger entries; re-extending an entry is idempotent).

use crate::constants::MAX_TTL_BATCH_SIZE;
use crate::tests::fixtures::{create_test_project, setup_contract};
use soroban_sdk::{testutils::Address as _, Address, Env, Vec};

extern crate alloc;
use alloc::format;

#[test]
fn ttl_extension_at_maximum_batch_size() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let mut ids = Vec::new(&env);
    for i in 0..MAX_TTL_BATCH_SIZE {
        let pid = create_test_project(&client, &owner, &format!("MaxTTL{i}"));
        ids.push_back(pid);
    }

    let result = client.extend_projects_ttl(&ids);
    assert_eq!(result.refreshed, MAX_TTL_BATCH_SIZE);
    assert_eq!(result.skipped_ids.len(), 0);
}

#[test]
fn ttl_for_expired_entries_reports_skipped() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);

    // Never-created ids behave like expired/evicted entries: skipped, no panic.
    let mut ids = Vec::new(&env);
    ids.push_back(u64::MAX);
    ids.push_back(u64::MAX - 1);

    let result = client.extend_projects_ttl(&ids);
    assert_eq!(result.refreshed, 0);
    assert_eq!(result.skipped_ids.len(), 2);
}

#[test]
fn batch_ttl_extension_with_partial_failures() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let real = create_test_project(&client, &owner, "PartialTTL");

    let mut ids = Vec::new(&env);
    ids.push_back(real);
    ids.push_back(999_001);
    ids.push_back(999_002);

    let result = client.extend_projects_ttl(&ids);
    assert_eq!(result.refreshed, 1);
    assert_eq!(result.skipped_ids.len(), 2);
    assert_eq!(result.skipped_ids.get(0), Some(999_001u64));
    assert_eq!(result.skipped_ids.get(1), Some(999_002u64));

    // Real project data untouched by the partial failure.
    let project = client.get_project(&real).unwrap();
    assert_eq!(project.id, real);
}

#[test]
fn ttl_restoration_after_pause_unpause() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let pid = create_test_project(&client, &owner, "PauseTTL");
    client.mock_all_auths().pause(&admin);
    assert!(client.is_paused());
    client.mock_all_auths().unpause(&admin);
    assert!(!client.is_paused());

    // TTL extension works after the pause/unpause cycle.
    client.extend_project_ttl(&pid);
    let project = client.get_project(&pid).unwrap();
    assert_eq!(project.id, pid);
}

#[test]
fn ttl_persists_across_sequential_extensions() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let pid = create_test_project(&client, &owner, "PersistTTL");
    // Re-extending is idempotent: repeated calls keep working and keep data.
    client.extend_project_ttl(&pid);
    client.extend_project_ttl(&pid);

    let mut ids = Vec::new(&env);
    ids.push_back(pid);
    let result = client.extend_projects_ttl(&ids);
    assert_eq!(result.refreshed, 1);

    let project = client.get_project(&pid).unwrap();
    assert_eq!(project.id, pid);
}
