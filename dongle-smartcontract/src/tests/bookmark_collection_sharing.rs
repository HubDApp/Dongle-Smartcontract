//! Tests for shareable public bookmark collections (#916).

#![cfg(test)]

use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::ContractError;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

fn name(env: &Env, s: &str) -> String {
    String::from_str(env, s)
}

#[test]
fn new_collection_is_private_with_empty_share_url() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let id = client.create_bookmark_collection(&owner, &name(&env, "DeFi picks"));
    let collection = client.get_bookmark_collection(&owner, &id);

    assert!(!collection.public);
    assert_eq!(collection.share_url, name(&env, ""));
    assert_eq!(collection.follower_count, 0);
    assert_eq!(collection.published_at, None);
}

#[test]
fn private_collection_hidden_from_other_viewers() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let stranger = Address::generate(&env);

    let id = client.create_bookmark_collection(&owner, &name(&env, "Secret list"));

    let result = client.try_get_bookmark_collection(&stranger, &id);
    assert_eq!(result, Err(Ok(ContractError::BookmarkCollectionNotPublic)));
}

#[test]
fn make_public_sets_share_url_and_lists_collection() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let viewer = Address::generate(&env);

    let id = client.create_bookmark_collection(&owner, &name(&env, "Shared list"));
    let url = client.make_bookmark_collection_public(&owner, &id);

    assert_eq!(url, name(&env, "https://dongle.hub/b/1"));
    let collection = client.get_bookmark_collection(&viewer, &id);
    assert!(collection.public);
    assert_eq!(collection.share_url, url);
    assert!(collection.published_at.is_some());
    assert_eq!(client.list_public_bookmark_collections(&0, &10).len(), 1);
}

#[test]
fn only_owner_can_publish_or_edit() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let other = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Collected Project");

    let id = client.create_bookmark_collection(&owner, &name(&env, "Mine"));

    assert_eq!(
        client.try_make_bookmark_collection_public(&other, &id),
        Err(Ok(ContractError::Unauthorized))
    );
    assert_eq!(
        client.try_add_project_to_bookmark_collection(&other, &id, &project_id),
        Err(Ok(ContractError::Unauthorized))
    );

    client.add_project_to_bookmark_collection(&owner, &id, &project_id);
    assert_eq!(
        client.try_remove_project_from_bookmark_collection(&other, &id, &project_id),
        Err(Ok(ContractError::Unauthorized))
    );
}

#[test]
fn publishing_twice_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let id = client.create_bookmark_collection(&owner, &name(&env, "Twice"));
    client.make_bookmark_collection_public(&owner, &id);

    assert_eq!(
        client.try_make_bookmark_collection_public(&owner, &id),
        Err(Ok(ContractError::BookmarkCollectionAlreadyPublic))
    );
}

#[test]
fn follow_requires_public_collection() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let follower = Address::generate(&env);

    let id = client.create_bookmark_collection(&owner, &name(&env, "Private"));

    assert_eq!(
        client.try_follow_bookmark_collection(&follower, &id),
        Err(Ok(ContractError::BookmarkCollectionNotPublic))
    );
}

#[test]
fn follow_and_unfollow_public_collection() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let follower = Address::generate(&env);

    let id = client.create_bookmark_collection(&owner, &name(&env, "Followable"));
    client.make_bookmark_collection_public(&owner, &id);

    client.follow_bookmark_collection(&follower, &id);
    assert!(client.is_following_bookmark_collection(&follower, &id));
    assert_eq!(client.get_bookmark_collection(&follower, &id).follower_count, 1);
    assert_eq!(client.get_followed_bookmark_collections(&follower).len(), 1);
    assert_eq!(
        client.try_follow_bookmark_collection(&follower, &id),
        Err(Ok(ContractError::AlreadyFollowingBookmarkCollection))
    );

    client.unfollow_bookmark_collection(&follower, &id);
    assert!(!client.is_following_bookmark_collection(&follower, &id));
    assert_eq!(client.get_bookmark_collection(&follower, &id).follower_count, 0);
    assert_eq!(
        client.try_unfollow_bookmark_collection(&follower, &id),
        Err(Ok(ContractError::NotFollowingBookmarkCollection))
    );
}

#[test]
fn owner_cannot_follow_own_collection() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let id = client.create_bookmark_collection(&owner, &name(&env, "Self"));
    client.make_bookmark_collection_public(&owner, &id);

    assert_eq!(
        client.try_follow_bookmark_collection(&owner, &id),
        Err(Ok(ContractError::CannotFollowOwnBookmarkCollection))
    );
}

#[test]
fn making_private_revokes_follower_access() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let follower = Address::generate(&env);

    let id = client.create_bookmark_collection(&owner, &name(&env, "Revocable"));
    client.make_bookmark_collection_public(&owner, &id);
    client.follow_bookmark_collection(&follower, &id);

    client.make_bookmark_collection_private(&owner, &id);

    assert_eq!(
        client.try_get_bookmark_collection(&follower, &id),
        Err(Ok(ContractError::BookmarkCollectionNotPublic))
    );
    assert_eq!(client.get_followed_bookmark_collections(&follower).len(), 0);
    assert_eq!(client.list_public_bookmark_collections(&0, &10).len(), 0);
    assert_eq!(client.get_bookmark_collection(&owner, &id).share_url, name(&env, ""));

    // Followers can still clean up after the collection goes private.
    client.unfollow_bookmark_collection(&follower, &id);
}

#[test]
fn missing_collection_returns_not_found() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let viewer = Address::generate(&env);

    assert_eq!(
        client.try_get_bookmark_collection(&viewer, &42),
        Err(Ok(ContractError::BookmarkCollectionNotFound))
    );
}
