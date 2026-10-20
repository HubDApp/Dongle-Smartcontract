//! Soft deletion with a time-limited recovery window (issue #875).

use crate::constants::PROJECT_RECOVERY_WINDOW_SECONDS;
use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env,
};

#[test]
fn delete_sets_deleted_at_and_recovery_deadline() {
    let env = Env::default();
    env.ledger().set_timestamp(1_000);
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let id = create_test_project(&client, &owner, "SoftDeleteA");

    client.mock_all_auths().delete_project(&id, &owner);

    let project = client.get_project(&id).unwrap();
    assert_eq!(project.deleted_at, Some(1_000));
    assert_eq!(
        project.recoverable_until,
        1_000 + PROJECT_RECOVERY_WINDOW_SECONDS
    );
    let deleted = client.get_deleted_projects(&admin);
    assert_eq!(deleted.len(), 1);
    assert_eq!(deleted.get(0).unwrap().id, id);
}

#[test]
fn double_delete_is_rejected() {
    let env = Env::default();
    let (client, _) = setup_contract(&env);
    let owner = Address::generate(&env);
    let id = create_test_project(&client, &owner, "SoftDeleteB");

    client.mock_all_auths().delete_project(&id, &owner);
    let res = client.mock_all_auths().try_delete_project(&id, &owner);
    assert_eq!(res, Err(Ok(ContractError::ProjectAlreadyDeleted)));
}

#[test]
fn non_owner_cannot_delete_or_recover() {
    let env = Env::default();
    let (client, _) = setup_contract(&env);
    let owner = Address::generate(&env);
    let stranger = Address::generate(&env);
    let id = create_test_project(&client, &owner, "SoftDeleteC");

    let res = client.mock_all_auths().try_delete_project(&id, &stranger);
    assert_eq!(res, Err(Ok(ContractError::Unauthorized)));

    client.mock_all_auths().delete_project(&id, &owner);
    let res = client
        .mock_all_auths()
        .try_recover_deleted_project(&id, &stranger);
    assert_eq!(res, Err(Ok(ContractError::Unauthorized)));
}

#[test]
fn recover_within_window_restores_project() {
    let env = Env::default();
    env.ledger().set_timestamp(1_000);
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let id = create_test_project(&client, &owner, "SoftDeleteD");

    client.mock_all_auths().delete_project(&id, &owner);
    env.ledger()
        .set_timestamp(1_000 + PROJECT_RECOVERY_WINDOW_SECONDS);
    client.mock_all_auths().recover_deleted_project(&id, &owner);

    let project = client.get_project(&id).unwrap();
    assert_eq!(project.deleted_at, None);
    assert_eq!(project.recoverable_until, 0);
    assert_eq!(client.get_deleted_projects(&admin).len(), 0);
}

#[test]
fn recover_live_project_is_rejected() {
    let env = Env::default();
    let (client, _) = setup_contract(&env);
    let owner = Address::generate(&env);
    let id = create_test_project(&client, &owner, "SoftDeleteE");

    let res = client
        .mock_all_auths()
        .try_recover_deleted_project(&id, &owner);
    assert_eq!(res, Err(Ok(ContractError::ProjectNotDeleted)));
}

#[test]
fn recover_after_window_expires_is_rejected() {
    let env = Env::default();
    env.ledger().set_timestamp(1_000);
    let (client, _) = setup_contract(&env);
    let owner = Address::generate(&env);
    let id = create_test_project(&client, &owner, "SoftDeleteF");

    client.mock_all_auths().delete_project(&id, &owner);
    env.ledger()
        .set_timestamp(1_000 + PROJECT_RECOVERY_WINDOW_SECONDS + 1);
    let res = client
        .mock_all_auths()
        .try_recover_deleted_project(&id, &owner);
    assert_eq!(res, Err(Ok(ContractError::RecoveryWindowExpired)));
}

#[test]
fn purge_removes_only_expired_projects() {
    let env = Env::default();
    env.ledger().set_timestamp(1_000);
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let expired = create_test_project(&client, &owner, "SoftDeleteG");
    let fresh = create_test_project(&client, &owner, "SoftDeleteH");

    client.mock_all_auths().delete_project(&expired, &owner);
    env.ledger()
        .set_timestamp(1_000 + PROJECT_RECOVERY_WINDOW_SECONDS);
    client.mock_all_auths().delete_project(&fresh, &owner);

    // Not yet expired: nothing purged.
    assert_eq!(client.purge_expired_projects(&10), 0);

    env.ledger()
        .set_timestamp(1_000 + PROJECT_RECOVERY_WINDOW_SECONDS + 1);
    assert_eq!(client.purge_expired_projects(&10), 1);
    assert!(client.get_project(&expired).is_none());
    assert!(client.get_project(&fresh).is_some());

    let deleted = client.get_deleted_projects(&admin);
    assert_eq!(deleted.len(), 1);
    assert_eq!(deleted.get(0).unwrap().id, fresh);
}

#[test]
fn purge_respects_limit() {
    let env = Env::default();
    env.ledger().set_timestamp(1_000);
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let a = create_test_project(&client, &owner, "SoftDeleteI");
    let b = create_test_project(&client, &owner, "SoftDeleteJ");
    client.mock_all_auths().delete_project(&a, &owner);
    client.mock_all_auths().delete_project(&b, &owner);

    env.ledger()
        .set_timestamp(1_000 + PROJECT_RECOVERY_WINDOW_SECONDS + 1);
    assert_eq!(client.purge_expired_projects(&1), 1);
    assert_eq!(client.get_deleted_projects(&admin).len(), 1);
    assert_eq!(client.purge_expired_projects(&1), 1);
    assert_eq!(client.get_deleted_projects(&admin).len(), 0);
}

#[test]
fn get_deleted_projects_requires_admin() {
    let env = Env::default();
    let (client, _) = setup_contract(&env);
    let stranger = Address::generate(&env);
    assert!(client.try_get_deleted_projects(&stranger).is_err());
}
