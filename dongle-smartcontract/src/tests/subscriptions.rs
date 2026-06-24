#![cfg(test)]

use crate::tests::fixtures::{create_test_project, setup_contract};
use soroban_sdk::{testutils::Address as _, Address, Env, Vec};

#[test]
fn test_follow_project() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let follower = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "FollowableProject");

    client.follow_project(&project_id, &follower);

    let count = client.get_follower_count(&project_id);
    assert_eq!(count, 1);

    let is_following = client.is_following(&project_id, &follower);
    assert!(is_following);
}

#[test]
fn test_unfollow_project() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let follower = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "UnfollowableProject");

    client.follow_project(&project_id, &follower);
    assert_eq!(client.get_follower_count(&project_id), 1);

    client.unfollow_project(&project_id, &follower);
    assert_eq!(client.get_follower_count(&project_id), 0);

    let is_following = client.is_following(&project_id, &follower);
    assert!(!is_following);
}

#[test]
fn test_duplicate_follow_returns_error() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let follower = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "DupFollowProject");

    client.follow_project(&project_id, &follower);

    let result = client.try_follow_project(&project_id, &follower);
    assert!(result.is_err());
}

#[test]
fn test_unfollow_missing_returns_error() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let follower = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "UnfollowMissing");

    let result = client.try_unfollow_project(&project_id, &follower);
    assert!(result.is_err());
}

#[test]
fn test_follower_count_multiple_followers() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "MultiFollowProject");

    let follower_count = 5u32;
    let mut followers = Vec::new(&env);
    for _ in 0..follower_count {
        let f = Address::generate(&env);
        followers.push_back(f.clone());
        client.follow_project(&project_id, &f);
    }

    assert_eq!(
        client.get_follower_count(&project_id),
        follower_count,
        "follower count should match total unique followers"
    );
}

#[test]
fn test_get_project_followers_pagination() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "PagedFollowers");

    let total = 5u32;
    for _ in 0..total {
        let f = Address::generate(&env);
        client.follow_project(&project_id, &f);
    }

    let page1 = client.get_project_followers(&project_id, &0, &2);
    assert_eq!(page1.len(), 2, "first page should have 2 followers");

    let page2 = client.get_project_followers(&project_id, &2, &2);
    assert_eq!(page2.len(), 2, "second page should have 2 followers");

    let page3 = client.get_project_followers(&project_id, &4, &2);
    assert_eq!(page3.len(), 1, "third page should have 1 follower");

    let empty = client.get_project_followers(&project_id, &10, &2);
    assert_eq!(empty.len(), 0, "start beyond length should be empty");
}

#[test]
fn test_get_user_subscriptions() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let follower = Address::generate(&env);

    let p1 = create_test_project(&client, &owner, "ProjectA");
    let p2 = create_test_project(&client, &owner, "ProjectB");
    let p3 = create_test_project(&client, &owner, "ProjectC");

    client.follow_project(&p1, &follower);
    client.follow_project(&p2, &follower);
    client.follow_project(&p3, &follower);

    let subs = client.get_user_subscriptions(&follower, &0, &10);
    assert_eq!(subs.len(), 3, "user should have 3 subscriptions");

    let mut ids = Vec::new(&env);
    for i in 0..subs.len() {
        if let Some(id) = subs.get(i) {
            ids.push_back(id);
        }
    }
    assert!(ids.contains(p1), "should contain project A");
    assert!(ids.contains(p2), "should contain project B");
    assert!(ids.contains(p3), "should contain project C");
}

#[test]
fn test_get_user_subscriptions_pagination() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let follower = Address::generate(&env);

    extern crate alloc;
    use alloc::string::ToString;

    let names = ["Proj_0", "Proj_1", "Proj_2", "Proj_3"];
    for name in names {
        let project_id = create_test_project(&client, &owner, name);
        client.follow_project(&project_id, &follower);
    }

    let page1 = client.get_user_subscriptions(&follower, &0, &2);
    assert_eq!(page1.len(), 2);

    let page2 = client.get_user_subscriptions(&follower, &2, &2);
    assert_eq!(page2.len(), 2);

    let empty = client.get_user_subscriptions(&follower, &10, &2);
    assert_eq!(empty.len(), 0);
}

#[test]
fn test_follow_nonexistent_project_returns_error() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let follower = Address::generate(&env);

    let result = client.try_follow_project(&999u64, &follower);
    assert!(result.is_err());
}

#[test]
fn test_unfollow_after_unfollow_is_error() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let follower = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "DoubleUnfollow");

    client.follow_project(&project_id, &follower);
    client.unfollow_project(&project_id, &follower);

    let result = client.try_unfollow_project(&project_id, &follower);
    assert!(result.is_err(), "second unfollow should fail");
}

#[test]
fn test_follow_after_unfollow_allows_refollow() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let follower = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "RefollowProject");

    client.follow_project(&project_id, &follower);
    client.unfollow_project(&project_id, &follower);
    assert_eq!(client.get_follower_count(&project_id), 0);

    client.follow_project(&project_id, &follower);
    assert_eq!(client.get_follower_count(&project_id), 1);

    let is_following = client.is_following(&project_id, &follower);
    assert!(is_following);
}

#[test]
fn test_multiple_projects_multiple_followers() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let project_a = create_test_project(&client, &owner, "ProjectA");
    let project_b = create_test_project(&client, &owner, "ProjectB");

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);

    client.follow_project(&project_a, &user1);
    client.follow_project(&project_a, &user2);
    client.follow_project(&project_b, &user1);

    assert_eq!(client.get_follower_count(&project_a), 2);
    assert_eq!(client.get_follower_count(&project_b), 1);

    let subs_user1 = client.get_user_subscriptions(&user1, &0, &10);
    assert_eq!(subs_user1.len(), 2);

    let subs_user2 = client.get_user_subscriptions(&user2, &0, &10);
    assert_eq!(subs_user2.len(), 1);
}
