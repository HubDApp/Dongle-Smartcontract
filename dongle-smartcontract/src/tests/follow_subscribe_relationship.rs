//! Follow/subscribe relationship clarity tests (closes #663).
//!
//! ## Feature Overview
//!
//! The contract exposes two related but distinct concepts:
//!
//! ### Follow (`follow_project` / `unfollow_project`)
//!
//! A user **follows** a project to express interest.  Following:
//! - Adds the follower's `Address` to `ProjectFollowers(project_id)`.
//! - Adds the `project_id` to `UserSubscriptions(follower)`.
//! - Increments `FollowerCount(project_id)`.
//! - Emits a `ProjectFollowed` event.
//!
//! ### Subscribe (`UserSubscriptions`)
//!
//! The term *subscription* in this contract is **synonymous with following**.
//! `UserSubscriptions(address)` is the per-user index of project IDs that the
//! user follows.  There is no separate "subscribe" operation — calling
//! `follow_project` is the only way to add a project to a user's subscription
//! list.  Calling `unfollow_project` removes it.
//!
//! ### Relationship rules (consistency invariants)
//!
//! 1. `is_following(project_id, user) == true`  ⟺  `user ∈ ProjectFollowers(project_id)`
//! 2. `is_following(project_id, user) == true`  ⟺  `project_id ∈ UserSubscriptions(user)`
//! 3. `get_follower_count(project_id) == len(ProjectFollowers(project_id))`
//! 4. A user cannot follow the same project twice (`AlreadyFollowing` error).
//! 5. Unfollowing a project that is not followed returns `NotFollowing`.
//! 6. Following a non-existent project returns `ProjectNotFound`.
//!
//! ### User experience
//!
//! - "Follow" and "subscribe" refer to the **same action** in the UI.
//! - The follower count is the number of unique addresses that have followed.
//! - A user's subscription list (`get_user_subscriptions`) returns the IDs of
//!   all projects they follow, in the order they followed them.
//! - Following/unfollowing is permissionless (any address can follow any
//!   active project).
//! - The pause guard applies: `follow_project` and `unfollow_project` fail
//!   with `ContractPaused` when the contract is paused.

use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use soroban_sdk::{testutils::Address as _, Address, Env, Vec};

// ── Invariant 1 & 2: is_following mirrors both indexes ───────────────────────

/// `is_following` returns true iff the user is in `ProjectFollowers` AND the
/// project is in `UserSubscriptions`.  We verify both sides via the public
/// read functions.
#[test]
fn follow_sets_both_indexes_consistently() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let follower = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "ConsistencyProject");

    // Before following: no membership in either index.
    assert!(
        !client.is_following(&project_id, &follower),
        "should not be following before follow_project"
    );
    let subs_before = client.get_user_subscriptions(&follower, &0, &100);
    assert_eq!(
        subs_before.len(),
        0,
        "user subscriptions should be empty before follow"
    );

    // Follow.
    client.follow_project(&project_id, &follower);

    // After following: both indexes must be consistent.
    assert!(
        client.is_following(&project_id, &follower),
        "is_following must be true after follow_project"
    );

    let followers = client.get_project_followers(&project_id, &0, &100);
    assert!(
        followers.contains(follower.clone()),
        "ProjectFollowers must contain the follower"
    );

    let subs_after = client.get_user_subscriptions(&follower, &0, &100);
    let mut found_in_subs = false;
    for i in 0..subs_after.len() {
        if let Some(pid) = subs_after.get(i) {
            if pid == project_id {
                found_in_subs = true;
            }
        }
    }
    assert!(found_in_subs, "UserSubscriptions must contain the project_id");
}

/// `unfollow_project` removes the user from both indexes.
#[test]
fn unfollow_removes_both_indexes_consistently() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let follower = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "UnfollowConsistency");

    client.follow_project(&project_id, &follower);
    assert!(client.is_following(&project_id, &follower));

    client.unfollow_project(&project_id, &follower);

    // is_following must be false.
    assert!(
        !client.is_following(&project_id, &follower),
        "is_following must be false after unfollow"
    );

    // ProjectFollowers must not contain the follower.
    let followers = client.get_project_followers(&project_id, &0, &100);
    assert!(
        !followers.contains(follower.clone()),
        "ProjectFollowers must not contain follower after unfollow"
    );

    // UserSubscriptions must not contain the project.
    let subs = client.get_user_subscriptions(&follower, &0, &100);
    let mut found = false;
    for i in 0..subs.len() {
        if let Some(pid) = subs.get(i) {
            if pid == project_id {
                found = true;
            }
        }
    }
    assert!(
        !found,
        "UserSubscriptions must not contain project after unfollow"
    );
}

// ── Invariant 3: follower count matches index length ─────────────────────────

#[test]
fn follower_count_matches_followers_list_length() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "CountConsistency");

    let n = 6u32;
    for _ in 0..n {
        let f = Address::generate(&env);
        client.follow_project(&project_id, &f);
    }

    let count = client.get_follower_count(&project_id);
    let followers = client.get_project_followers(&project_id, &0, &100);

    assert_eq!(
        count,
        followers.len(),
        "get_follower_count must equal the length of get_project_followers"
    );
    assert_eq!(count, n);
}

#[test]
fn follower_count_decrements_on_unfollow() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "DecrementCount");

    let f1 = Address::generate(&env);
    let f2 = Address::generate(&env);
    client.follow_project(&project_id, &f1);
    client.follow_project(&project_id, &f2);
    assert_eq!(client.get_follower_count(&project_id), 2);

    client.unfollow_project(&project_id, &f1);
    assert_eq!(client.get_follower_count(&project_id), 1);

    client.unfollow_project(&project_id, &f2);
    assert_eq!(client.get_follower_count(&project_id), 0);
}

// ── Invariant 4: no duplicate follows ─────────────────────────────────────────

#[test]
fn duplicate_follow_returns_already_following() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let follower = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "NoDupFollow");

    client.follow_project(&project_id, &follower);

    let result = client.try_follow_project(&project_id, &follower);
    assert_eq!(result, Err(Ok(ContractError::AlreadyFollowing)));
    // Count must remain 1.
    assert_eq!(client.get_follower_count(&project_id), 1);
}

// ── Invariant 5: unfollow requires prior follow ───────────────────────────────

#[test]
fn unfollow_without_prior_follow_returns_not_following() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let follower = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "NotFollowingProject");

    let result = client.try_unfollow_project(&project_id, &follower);
    assert_eq!(result, Err(Ok(ContractError::NotFollowing)));
}

#[test]
fn double_unfollow_returns_not_following() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let follower = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "DoubleUnfollow");

    client.follow_project(&project_id, &follower);
    client.unfollow_project(&project_id, &follower);

    let result = client.try_unfollow_project(&project_id, &follower);
    assert_eq!(result, Err(Ok(ContractError::NotFollowing)));
}

// ── Invariant 6: non-existent project ────────────────────────────────────────

#[test]
fn follow_nonexistent_project_returns_project_not_found() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let follower = Address::generate(&env);
    let result = client.try_follow_project(&999_999u64, &follower);
    assert_eq!(result, Err(Ok(ContractError::ProjectNotFound)));
}

// ── Re-follow after unfollow ──────────────────────────────────────────────────

#[test]
fn re_follow_after_unfollow_restores_all_indexes() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let follower = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "ReFollowProject");

    // Follow → unfollow → re-follow.
    client.follow_project(&project_id, &follower);
    client.unfollow_project(&project_id, &follower);
    client.follow_project(&project_id, &follower);

    // All invariants must hold after re-follow.
    assert!(client.is_following(&project_id, &follower));
    assert_eq!(client.get_follower_count(&project_id), 1);

    let subs = client.get_user_subscriptions(&follower, &0, &100);
    let mut found = false;
    for i in 0..subs.len() {
        if let Some(pid) = subs.get(i) {
            if pid == project_id {
                found = true;
            }
        }
    }
    assert!(found, "project must appear in subscriptions after re-follow");
}

// ── Subscribe = follow (documentation test) ───────────────────────────────────

/// `get_user_subscriptions` returns exactly the set of projects the user follows.
/// This test documents that "subscribe" and "follow" are synonymous in this contract.
#[test]
fn user_subscriptions_equals_followed_projects() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let follower = Address::generate(&env);

    let p1 = create_test_project(&client, &owner, "Sub-A");
    let p2 = create_test_project(&client, &owner, "Sub-B");
    let p3 = create_test_project(&client, &owner, "Sub-C");

    // Follow p1 and p3 but NOT p2.
    client.follow_project(&p1, &follower);
    client.follow_project(&p3, &follower);

    // Subscription list must contain exactly p1 and p3.
    let subs = client.get_user_subscriptions(&follower, &0, &100);
    assert_eq!(subs.len(), 2, "subscriptions should mirror followed projects");

    let mut ids: Vec<u64> = Vec::new(&env);
    for i in 0..subs.len() {
        if let Some(id) = subs.get(i) {
            ids.push_back(id);
        }
    }
    assert!(ids.contains(p1), "p1 must be in subscriptions");
    assert!(ids.contains(p3), "p3 must be in subscriptions");
    assert!(!ids.contains(p2), "p2 must NOT be in subscriptions (not followed)");

    // `is_following` must agree.
    assert!(client.is_following(&p1, &follower));
    assert!(!client.is_following(&p2, &follower));
    assert!(client.is_following(&p3, &follower));
}

/// Unfollowing a project removes it from subscriptions.
#[test]
fn unfollow_removes_from_subscriptions() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let follower = Address::generate(&env);

    let p1 = create_test_project(&client, &owner, "UnsubA");
    let p2 = create_test_project(&client, &owner, "UnsubB");

    client.follow_project(&p1, &follower);
    client.follow_project(&p2, &follower);

    client.unfollow_project(&p1, &follower);

    let subs = client.get_user_subscriptions(&follower, &0, &100);
    assert_eq!(subs.len(), 1, "only p2 should remain in subscriptions");
    assert_eq!(subs.get(0), Some(p2));
}

// ── Multiple projects / multiple users ───────────────────────────────────────

#[test]
fn multiple_users_can_follow_same_project() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "MultiFollowProject");

    let users: Vec<Address> = {
        let mut v = Vec::new(&env);
        for _ in 0..4 {
            v.push_back(Address::generate(&env));
        }
        v
    };

    for i in 0..users.len() {
        if let Some(u) = users.get(i) {
            client.follow_project(&project_id, &u);
        }
    }

    assert_eq!(client.get_follower_count(&project_id), 4);

    // Each user must have the project in their subscriptions.
    for i in 0..users.len() {
        if let Some(u) = users.get(i) {
            let subs = client.get_user_subscriptions(&u, &0, &100);
            assert_eq!(subs.len(), 1);
            assert_eq!(subs.get(0), Some(project_id));
        }
    }
}

#[test]
fn one_user_can_follow_multiple_projects() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let follower = Address::generate(&env);

    let names = ["MultiA", "MultiB", "MultiC", "MultiD", "MultiE"];
    let mut project_ids: Vec<u64> = Vec::new(&env);

    for name in &names {
        let pid = create_test_project(&client, &owner, name);
        project_ids.push_back(pid);
        client.follow_project(&pid, &follower);
    }

    let subs = client.get_user_subscriptions(&follower, &0, &100);
    assert_eq!(subs.len(), 5, "user should have 5 subscriptions");

    // Each project's follower count is 1.
    for i in 0..project_ids.len() {
        if let Some(pid) = project_ids.get(i) {
            assert_eq!(
                client.get_follower_count(&pid),
                1,
                "each project should have exactly 1 follower"
            );
        }
    }
}

// ── Pause guard applies to follow/unfollow ────────────────────────────────────

#[test]
fn follow_fails_when_contract_is_paused() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let follower = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "PauseFollow");

    client.pause(&admin);

    let result = client.try_follow_project(&project_id, &follower);
    assert_eq!(result, Err(Ok(ContractError::ContractPaused)));
}

#[test]
fn unfollow_fails_when_contract_is_paused() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let follower = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "PauseUnfollow");

    client.follow_project(&project_id, &follower);

    client.pause(&admin);

    let result = client.try_unfollow_project(&project_id, &follower);
    assert_eq!(result, Err(Ok(ContractError::ContractPaused)));
}

#[test]
fn follow_works_again_after_unpause() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let follower = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "UnpausedFollow");

    client.pause(&admin);
    client.unpause(&admin);

    // Should work now.
    let result = client.try_follow_project(&project_id, &follower);
    assert!(result.is_ok(), "follow_project must work after unpause");
    assert!(client.is_following(&project_id, &follower));
}
