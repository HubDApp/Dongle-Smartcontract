//! Contract upgrade and data migration paths (issue #967).
//!
//! An upgrade replaces the contract code while persistent storage stays at the
//! same contract address. These tests simulate a v1.0 -> v2.0 upgrade by
//! re-registering the contract code at the existing address (the test-env
//! equivalent of `update_current_contract_wasm`) and then asserting that data
//! written before the upgrade is still readable, writable and evented the same
//! way afterwards. See `docs/STORAGE_MIGRATION_GUIDE.md` for the procedure.

use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::VerificationStatus;
use crate::{DongleContract, DongleContractClient};
use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events},
    Address, Env, IntoVal, Val, Vec,
};

/// Swap in the "v2.0" code at the same address, keeping all storage.
fn upgrade<'a>(env: &'a Env, client: &DongleContractClient<'a>) -> DongleContractClient<'a> {
    let id = client.address.clone();
    env.register_at(&id, DongleContract, ());
    DongleContractClient::new(env, &id)
}

#[test]
fn upgrade_preserves_projects_reviews_and_admins() {
    let env = Env::default();
    let (v1, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);

    let id = create_test_project(&v1, &owner, "Upgrade Project");
    v1.mock_all_auths().add_review(&id, &reviewer, &4, &None);
    let project_before = v1.get_project(&id).unwrap();
    let stats_before = v1.get_project_stats(&id);

    let v2 = upgrade(&env, &v1);

    assert_eq!(v2.get_project(&id).unwrap(), project_before);
    assert_eq!(v2.get_project_stats(&id), stats_before);
    assert_eq!(v2.get_project_count(), 1);
    assert_eq!(v2.get_projects_by_owner(&owner).len(), 1);
    assert!(v2.is_admin(&admin));
    assert_eq!(
        v2.get_project(&id).unwrap().verification_status,
        VerificationStatus::Unverified
    );
}

#[test]
fn data_written_before_upgrade_remains_mutable_after_upgrade() {
    let env = Env::default();
    let (v1, _) = setup_contract(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let id = create_test_project(&v1, &owner, "Upgrade Mutable");
    v1.mock_all_auths().add_review(&id, &reviewer, &5, &None);

    let v2 = upgrade(&env, &v1);

    // Records created by v1 can be updated by v2.
    v2.mock_all_auths()
        .update_review(&id, &reviewer, &3, &None, &None);
    assert_eq!(v2.get_project_stats(&id).rating_sum, 300);

    // Ids keep incrementing from the pre-upgrade counter (no collisions).
    let next = create_test_project(&v2, &owner, "Upgrade Next");
    assert_eq!(next, id + 1);
}

#[test]
fn storage_schema_migration_new_fields_default_for_existing_projects() {
    // v2.0 introduced soft-deletion fields on `Project` (#875). Projects that
    // existed before the upgrade must surface the "live" defaults and be able
    // to use the new code paths.
    let env = Env::default();
    let (v1, _) = setup_contract(&env);
    let owner = Address::generate(&env);
    let id = create_test_project(&v1, &owner, "Upgrade Schema");

    let v2 = upgrade(&env, &v1);
    let project = v2.get_project(&id).unwrap();
    assert_eq!(project.deleted_at, None);
    assert_eq!(project.recoverable_until, 0);

    v2.mock_all_auths().delete_project(&id, &owner);
    v2.mock_all_auths().recover_deleted_project(&id, &owner);
    assert_eq!(v2.get_project(&id).unwrap().deleted_at, None);
}

#[test]
fn event_topics_are_compatible_across_upgrade() {
    let env = Env::default();
    let (v1, _) = setup_contract(&env);
    let owner = Address::generate(&env);

    let created_topics = |id: u64| -> Vec<Val> {
        (symbol_short!("PROJECT"), symbol_short!("CREATED"), id).into_val(&env)
    };
    let emitted = |topics: Vec<Val>| env.events().all().iter().any(|(_, t, _)| t == topics);

    let before = create_test_project(&v1, &owner, "Upgrade Events A");
    assert!(emitted(created_topics(before)));

    let v2 = upgrade(&env, &v1);
    let after = create_test_project(&v2, &owner, "Upgrade Events B");
    assert!(emitted(created_topics(after)));
}
