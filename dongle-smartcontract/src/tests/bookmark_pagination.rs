//! Bookmark registry pagination and isolation tests (issue #495).
//!
//! `bookmarks.rs` already covers the happy paths, duplicate/missing errors and
//! basic paging. This module pins the boundary behaviour of
//! `get_user_bookmarks` that nothing else exercises — the limit clamping in
//! particular, which silently rewrites the caller's argument:
//!
//! ```ignore
//! let effective_limit = if limit == 0 || limit > MAX_PAGE_LIMIT {
//!     MAX_PAGE_LIMIT
//! } else { limit };
//! ```
//!
//! A caller passing `limit = 0` gets a *full page*, not an empty one.

#![cfg(test)]

use crate::bookmark_registry::MAX_PAGE_LIMIT;
use crate::tests::fixtures::{create_test_project, setup_contract};
use soroban_sdk::{testutils::Address as _, Address, Env};

extern crate alloc;
use alloc::format;
use alloc::vec::Vec as AllocVec;

/// Bookmark `count` fresh projects for `user`, returning the ids in order.
fn bookmark_n(
    client: &crate::DongleContractClient<'_>,
    owner: &Address,
    user: &Address,
    prefix: &str,
    count: u32,
) -> AllocVec<u64> {
    (0..count)
        .map(|i| {
            let id = create_test_project(client, owner, &format!("{prefix}-{i}"));
            client.bookmark_project(&id, user);
            id
        })
        .collect()
}

// ─── Limit clamping ──────────────────────────────────────────────────────────

#[test]
fn test_limit_zero_returns_a_full_page_not_an_empty_one() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    bookmark_n(&client, &owner, &user, "LimitZero", 3);

    // `limit == 0` is treated as "unspecified" and clamped up to MAX_PAGE_LIMIT.
    let page = client.get_user_bookmarks(&user, &0, &0);
    assert_eq!(page.len(), 3);
}

#[test]
fn test_limit_above_the_maximum_is_clamped() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    bookmark_n(&client, &owner, &user, "LimitHuge", 3);

    let page = client.get_user_bookmarks(&user, &0, &u32::MAX);
    assert_eq!(page.len(), 3, "clamping must not truncate a short list");

    let clamped = client.get_user_bookmarks(&user, &0, &(MAX_PAGE_LIMIT + 1));
    assert_eq!(clamped.len(), 3);
}

#[test]
fn test_limit_of_one_returns_a_single_entry() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    let ids = bookmark_n(&client, &owner, &user, "LimitOne", 3);

    let page = client.get_user_bookmarks(&user, &0, &1);
    assert_eq!(page.len(), 1);
    assert_eq!(page.get(0).unwrap(), ids[0]);
}

// ─── Offset boundaries ───────────────────────────────────────────────────────

#[test]
fn test_start_at_the_length_returns_empty() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    bookmark_n(&client, &owner, &user, "StartAtLen", 3);

    // start == len is the first out-of-range offset.
    assert_eq!(client.get_user_bookmarks(&user, &3, &10).len(), 0);
    assert_eq!(client.get_user_bookmarks(&user, &2, &10).len(), 1);
}

#[test]
fn test_start_beyond_the_length_returns_empty() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    bookmark_n(&client, &owner, &user, "StartBeyond", 2);

    assert_eq!(client.get_user_bookmarks(&user, &99, &10).len(), 0);
    assert_eq!(client.get_user_bookmarks(&user, &u32::MAX, &10).len(), 0);
}

#[test]
fn test_start_plus_limit_overflow_does_not_panic() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    bookmark_n(&client, &owner, &user, "Overflow", 2);

    // `start + limit` is computed with saturating arithmetic; without it this
    // would overflow in a release build.
    let page = client.get_user_bookmarks(&user, &1, &u32::MAX);
    assert_eq!(page.len(), 1);
}

#[test]
fn test_paging_covers_every_bookmark_exactly_once() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    let ids = bookmark_n(&client, &owner, &user, "FullSweep", 5);

    let mut seen = AllocVec::new();
    let mut start = 0u32;
    loop {
        let page = client.get_user_bookmarks(&user, &start, &2);
        if page.len() == 0 {
            break;
        }
        for i in 0..page.len() {
            seen.push(page.get(i).unwrap());
        }
        start += 2;
    }

    assert_eq!(seen.len(), ids.len());
    for id in ids.iter() {
        assert!(seen.contains(id), "bookmark {id} missing from the sweep");
    }
}

// ─── Empty and isolated state ────────────────────────────────────────────────

#[test]
fn test_user_with_no_bookmarks_returns_empty() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let user = Address::generate(&env);

    assert_eq!(client.get_user_bookmarks(&user, &0, &10).len(), 0);
    assert_eq!(client.get_user_bookmarks(&user, &0, &0).len(), 0);
}

#[test]
fn test_bookmark_lists_are_isolated_per_user() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let alice = Address::generate(&env);
    let bob = Address::generate(&env);

    let alice_ids = bookmark_n(&client, &owner, &alice, "AliceOnly", 3);
    let bob_ids = bookmark_n(&client, &owner, &bob, "BobOnly", 1);

    assert_eq!(client.get_user_bookmarks(&alice, &0, &10).len(), 3);
    assert_eq!(client.get_user_bookmarks(&bob, &0, &10).len(), 1);
    assert_eq!(
        client.get_user_bookmarks(&bob, &0, &10).get(0).unwrap(),
        bob_ids[0]
    );
    assert!(!client.is_bookmarked(&alice_ids[0], &bob));
}

#[test]
fn test_unbookmarking_shrinks_the_page() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    let ids = bookmark_n(&client, &owner, &user, "Shrinking", 3);
    assert_eq!(client.get_user_bookmarks(&user, &0, &10).len(), 3);

    client.unbookmark_project(&ids[1], &user);

    let page = client.get_user_bookmarks(&user, &0, &10);
    assert_eq!(page.len(), 2);
    // The removed id must be gone, and the survivors intact.
    for i in 0..page.len() {
        assert_ne!(page.get(i).unwrap(), ids[1]);
    }
    assert!(client.is_bookmarked(&ids[0], &user));
    assert!(client.is_bookmarked(&ids[2], &user));
}

#[test]
fn test_unbookmarking_everything_returns_to_empty() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    let ids = bookmark_n(&client, &owner, &user, "Drained", 2);
    for id in ids.iter() {
        client.unbookmark_project(id, &user);
    }

    assert_eq!(client.get_user_bookmarks(&user, &0, &10).len(), 0);
    for id in ids.iter() {
        assert!(!client.is_bookmarked(id, &user));
    }
}
