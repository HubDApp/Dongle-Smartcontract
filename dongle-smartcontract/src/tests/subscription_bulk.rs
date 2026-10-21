//! Bulk subscription tests (#932): batch subscribe, deduplication, export /
//! import, and admin cloning.

use crate::constants::{MAX_SUBSCRIPTION_BATCH_SIZE, SUBSCRIPTION_EXPORT_FORMAT_VERSION};
use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::EventSubscription;
use soroban_sdk::{testutils::Address as _, vec, Address, Env, Vec};

fn sub(subscriber: &Address, project_id: u64) -> EventSubscription {
    EventSubscription {
        subscriber: subscriber.clone(),
        project_id,
    }
}

#[test]
fn test_batch_subscribe_creates_all_subscriptions() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let p1 = create_test_project(&client, &admin, "BulkSubA");
    let p2 = create_test_project(&client, &admin, "BulkSubB");
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);

    let batch = vec![&env, sub(&alice, p1), sub(&alice, p2), sub(&bob, p1)];
    let result = client.subscribe_to_events_batch(&batch);

    assert_eq!(result.subscribed, batch);
    assert!(result.duplicates.is_empty());
    assert!(result.failed.is_empty());
    assert!(client.is_following(&p1, &alice));
    assert!(client.is_following(&p2, &alice));
    assert!(client.is_following(&p1, &bob));
    assert_eq!(client.get_follower_count(&p1), 2);
    assert_eq!(client.get_user_subscriptions(&alice, &0, &10), vec![&env, p1, p2]);
}

#[test]
fn test_batch_subscribe_dedups_within_batch_and_existing() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let p1 = create_test_project(&client, &admin, "DedupA");
    let p2 = create_test_project(&client, &admin, "DedupB");
    let alice = Address::generate(&env);
    client.follow_project(&p1, &alice);

    let batch = vec![&env, sub(&alice, p1), sub(&alice, p2), sub(&alice, p2)];
    let result = client.subscribe_to_events_batch(&batch);

    assert_eq!(result.subscribed, vec![&env, sub(&alice, p2)]);
    assert_eq!(result.duplicates, vec![&env, sub(&alice, p1), sub(&alice, p2)]);
    assert!(result.failed.is_empty());
    assert_eq!(client.get_follower_count(&p2), 1);
    assert_eq!(client.get_user_subscriptions(&alice, &0, &10), vec![&env, p1, p2]);
}

#[test]
fn test_batch_subscribe_reports_missing_project_without_aborting() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let p1 = create_test_project(&client, &admin, "PartialSub");
    let alice = Address::generate(&env);

    let result = client.subscribe_to_events_batch(&vec![&env, sub(&alice, 9_999), sub(&alice, p1)]);

    assert_eq!(result.subscribed, vec![&env, sub(&alice, p1)]);
    assert_eq!(result.failed.len(), 1);
    let failure = result.failed.get(0).unwrap();
    assert_eq!(failure.subscription, sub(&alice, 9_999));
    assert_eq!(failure.error_code, ContractError::ProjectNotFound as u32);
}

#[test]
fn test_batch_subscribe_rejects_empty_and_oversized() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let p1 = create_test_project(&client, &admin, "SizeSub");

    let empty: Vec<EventSubscription> = Vec::new(&env);
    assert_eq!(
        client.try_subscribe_to_events_batch(&empty),
        Err(Ok(ContractError::InvalidInput.into()))
    );

    let mut oversized = Vec::new(&env);
    for _ in 0..=MAX_SUBSCRIPTION_BATCH_SIZE {
        oversized.push_back(sub(&Address::generate(&env), p1));
    }
    assert_eq!(
        client.try_subscribe_to_events_batch(&oversized),
        Err(Ok(ContractError::InvalidInput.into()))
    );
    assert_eq!(client.get_follower_count(&p1), 0);
}

#[test]
fn test_export_then_import_to_new_wallet() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let p1 = create_test_project(&client, &admin, "ExportA");
    let p2 = create_test_project(&client, &admin, "ExportB");
    let old_wallet = Address::generate(&env);
    let new_wallet = Address::generate(&env);
    client.follow_project(&p1, &old_wallet);
    client.follow_project(&p2, &old_wallet);

    let export = client.export_subscriptions(&old_wallet);
    assert_eq!(export.format_version, SUBSCRIPTION_EXPORT_FORMAT_VERSION);
    assert_eq!(export.subscriber, old_wallet);
    assert_eq!(export.project_ids, vec![&env, p1, p2]);

    let result = client.import_subscriptions(&new_wallet, &export);
    assert_eq!(result.subscribed.len(), 2);
    assert_eq!(client.get_user_subscriptions(&new_wallet, &0, &10), vec![&env, p1, p2]);

    // Importing again is a no-op: everything is a duplicate.
    let again = client.import_subscriptions(&new_wallet, &export);
    assert!(again.subscribed.is_empty());
    assert_eq!(again.duplicates.len(), 2);
    assert_eq!(client.get_follower_count(&p1), 2);
}

#[test]
fn test_import_rejects_unknown_format_version() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let p1 = create_test_project(&client, &admin, "ImportVersion");
    let user = Address::generate(&env);
    client.follow_project(&p1, &user);

    let mut export = client.export_subscriptions(&user);
    export.format_version = SUBSCRIPTION_EXPORT_FORMAT_VERSION + 1;

    assert_eq!(
        client.try_import_subscriptions(&Address::generate(&env), &export),
        Err(Ok(ContractError::InvalidInput.into()))
    );
}

#[test]
fn test_admin_clones_subscriptions() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let p1 = create_test_project(&client, &admin, "CloneA");
    let p2 = create_test_project(&client, &admin, "CloneB");
    let source = Address::generate(&env);
    let target = Address::generate(&env);
    client.follow_project(&p1, &source);
    client.follow_project(&p2, &source);
    client.follow_project(&p2, &target);

    let result = client.clone_subscriptions_from_address(&admin, &source, &target);

    assert_eq!(result.subscribed, vec![&env, sub(&target, p1)]);
    assert_eq!(result.duplicates, vec![&env, sub(&target, p2)]);
    assert!(client.is_following(&p1, &target));
    // Source is unchanged.
    assert_eq!(client.get_user_subscriptions(&source, &0, &10), vec![&env, p1, p2]);
}

#[test]
fn test_clone_requires_admin_and_distinct_addresses() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let source = Address::generate(&env);
    let target = Address::generate(&env);
    let outsider = Address::generate(&env);

    assert_eq!(
        client.try_clone_subscriptions_from_address(&outsider, &source, &target),
        Err(Ok(ContractError::AdminOnly.into()))
    );
    assert_eq!(
        client.try_clone_subscriptions_from_address(&admin, &source, &source),
        Err(Ok(ContractError::InvalidInput.into()))
    );
}

#[test]
fn test_clone_from_address_with_no_subscriptions_is_empty() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    let result = client.clone_subscriptions_from_address(
        &admin,
        &Address::generate(&env),
        &Address::generate(&env),
    );

    assert!(result.subscribed.is_empty());
    assert!(result.duplicates.is_empty());
    assert!(result.failed.is_empty());
}
