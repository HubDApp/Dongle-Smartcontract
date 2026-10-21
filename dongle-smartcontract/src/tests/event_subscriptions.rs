//! Tests for on-chain contract event subscriptions (#931).

use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::{EventFilters, EventTopic};
use soroban_sdk::{testutils::Address as _, vec, Address, Env, Vec};

fn no_filters(env: &Env) -> EventFilters {
    EventFilters {
        project_ids: Vec::new(env),
        actor: None,
    }
}

#[test]
fn test_subscribe_and_query_by_address_and_topic() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let relay = Address::generate(&env);

    let id = client.subscribe_to_events(
        &relay,
        &vec![&env, EventTopic::Verification, EventTopic::Review],
        &no_filters(&env),
    );

    let sub = client.get_event_subscription(&id).unwrap();
    assert_eq!(sub.address, relay);
    assert_eq!(sub.event_topics.len(), 2);

    let by_address = client.get_subscriptions_for_address(&admin, &relay);
    assert_eq!(by_address.len(), 1);
    assert_eq!(by_address.get(0).unwrap().id, id);

    assert_eq!(client.get_subscriptions_for_topic(&EventTopic::Review, &0, &10).len(), 1);
    assert_eq!(client.get_subscriptions_for_topic(&EventTopic::Fee, &0, &10).len(), 0);
}

#[test]
fn test_get_subscriptions_for_address_is_admin_only() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let relay = Address::generate(&env);
    client.subscribe_to_events(&relay, &vec![&env, EventTopic::Fee], &no_filters(&env));

    assert_eq!(
        client.try_get_subscriptions_for_address(&relay, &relay),
        Err(Ok(ContractError::AdminOnly))
    );
}

#[test]
fn test_filters_by_project() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let relay = Address::generate(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Alpha");

    let filters = EventFilters {
        project_ids: vec![&env, project_id],
        actor: Some(owner.clone()),
    };
    let id = client.subscribe_to_events(&relay, &vec![&env, EventTopic::Project], &filters);
    assert_eq!(client.get_event_subscription(&id).unwrap().filters, filters);

    let missing = EventFilters {
        project_ids: vec![&env, 999],
        actor: None,
    };
    assert_eq!(
        client.try_subscribe_to_events(&relay, &vec![&env, EventTopic::Project], &missing),
        Err(Ok(ContractError::ProjectNotFound))
    );
}

#[test]
fn test_invalid_topics_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let relay = Address::generate(&env);

    assert_eq!(
        client.try_subscribe_to_events(&relay, &Vec::new(&env), &no_filters(&env)),
        Err(Ok(ContractError::InvalidEventSubscription))
    );
    assert_eq!(
        client.try_subscribe_to_events(
            &relay,
            &vec![&env, EventTopic::Fee, EventTopic::Fee],
            &no_filters(&env)
        ),
        Err(Ok(ContractError::InvalidEventSubscription))
    );
}

#[test]
fn test_update_moves_topic_indexes() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let relay = Address::generate(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Alpha");

    let id = client.subscribe_to_events(
        &relay,
        &vec![&env, EventTopic::Fee, EventTopic::Review],
        &no_filters(&env),
    );
    let filters = EventFilters {
        project_ids: vec![&env, project_id],
        actor: None,
    };
    client.update_event_subscription(
        &relay,
        &id,
        &vec![&env, EventTopic::Review, EventTopic::Changelog],
        &filters,
    );

    let sub = client.get_event_subscription(&id).unwrap();
    assert_eq!(sub.filters, filters);
    assert_eq!(client.get_subscriptions_for_topic(&EventTopic::Fee, &0, &10).len(), 0);
    assert_eq!(client.get_subscriptions_for_topic(&EventTopic::Review, &0, &10).len(), 1);
    assert_eq!(client.get_subscriptions_for_topic(&EventTopic::Changelog, &0, &10).len(), 1);
}

#[test]
fn test_unsubscribe_cleans_indexes() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let relay = Address::generate(&env);

    let id = client.subscribe_to_events(&relay, &vec![&env, EventTopic::Admin], &no_filters(&env));
    client.unsubscribe_from_events(&relay, &id);

    assert!(client.get_event_subscription(&id).is_none());
    assert_eq!(client.get_subscriptions_for_address(&admin, &relay).len(), 0);
    assert_eq!(client.get_subscriptions_for_topic(&EventTopic::Admin, &0, &10).len(), 0);
    assert_eq!(
        client.try_unsubscribe_from_events(&relay, &id),
        Err(Ok(ContractError::EventSubscriptionNotFound))
    );
}

#[test]
fn test_only_subscriber_can_modify() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let relay = Address::generate(&env);
    let other = Address::generate(&env);

    let id = client.subscribe_to_events(&relay, &vec![&env, EventTopic::Fee], &no_filters(&env));
    assert_eq!(
        client.try_unsubscribe_from_events(&other, &id),
        Err(Ok(ContractError::EventSubscriptionNotFound))
    );
    assert_eq!(
        client.try_update_event_subscription(
            &other,
            &id,
            &vec![&env, EventTopic::Review],
            &no_filters(&env)
        ),
        Err(Ok(ContractError::EventSubscriptionNotFound))
    );
}

#[test]
fn test_per_address_limit() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let relay = Address::generate(&env);

    for _ in 0..crate::constants::MAX_EVENT_SUBSCRIPTIONS_PER_ADDRESS {
        client.subscribe_to_events(&relay, &vec![&env, EventTopic::Fee], &no_filters(&env));
    }
    assert_eq!(
        client.try_subscribe_to_events(&relay, &vec![&env, EventTopic::Fee], &no_filters(&env)),
        Err(Ok(ContractError::EventSubscriptionLimitReached))
    );
}
