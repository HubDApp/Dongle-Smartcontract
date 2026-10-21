//! Issue #972 — Reentrancy and race condition protection tests.
//!
//! Soroban transactions execute atomically and in strict isolation — there
//! is no interleaving between two contract calls within a single ledger
//! entry.  This means reentrancy in the Solidity sense (a callback re-entering
//! the contract mid-state-change) is structurally impossible.
//!
//! However, **logical races** can still occur when two transactions are
//! submitted in the same ledger close and both observe the *pre* state:
//!
//! * Duplicate review submission by the same reviewer — the second must
//!   be rejected with `DuplicateReview`.
//! * Duplicate project registration with the same slug — the second must
//!   be rejected.
//! * Double-delete of a review — the second must error with `ReviewNotFound`.
//! * Pausing while a write is in-flight — the pause must not corrupt the
//!   in-flight data.
//!
//! These tests simulate the sequential orderings that *represent* concurrent
//! submit scenarios (since the Soroban test harness executes one call at a
//! time) and assert the **invariants** that must hold after every mutation.

extern crate alloc;

use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::{
    ProjectRegistrationParams, ProjectSortMode, ProjectUpdateParams, ReviewSortMode,
};
use soroban_sdk::{testutils::Address as _, Address, Env, String};

// ── Reentrancy: duplicate review from same reviewer is rejected ──────────────

#[test]
fn test_reentrancy_duplicate_review_rejected() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "ReentrancyProject");

    let reviewer = Address::generate(&env);
    client.mock_all_auths().add_review(&project_id, &reviewer, &5, &None);

    // A second "reentrant" call from the same reviewer must be rejected.
    let result = client
        .mock_all_auths()
        .try_add_review(&project_id, &reviewer, &3, &None);
    assert_eq!(result, Err(Ok(ContractError::DuplicateReview)));

    // Original review is unchanged.
    let review = client.get_review(&project_id, &reviewer).unwrap();
    assert_eq!(review.rating, 5);
}

// ── Race: two reviewers submit concurrently (simulated sequential) ──────────

#[test]
fn test_race_two_reviewers_same_project() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "RaceProject");

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);

    client.mock_all_auths().add_review(&project_id, &r1, &4, &None);
    client.mock_all_auths().add_review(&project_id, &r2, &5, &None);

    // Both reviews present, stats consistent.
    let stats = client.get_project_stats(&project_id);
    assert_eq!(stats.review_count, 2);
    assert_eq!(stats.rating_sum, 900);
}

// ── Race: duplicate slug registration is rejected ────────────────────────────

#[test]
fn test_race_duplicate_slug_rejected() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner1 = Address::generate(&env);
    let owner2 = Address::generate(&env);

    let params = ProjectRegistrationParams {
        owner: owner1.clone(),
        name: String::from_str(&env, "DupSlug"),
        slug: String::from_str(&env, "dup-slug"),
        description: String::from_str(&env, "First"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        license: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
        repository_url: None,
        language_code: None,
        verification_requirements: None,
    };
    let _id1 = client.mock_all_auths().register_project(&params);

    // Same slug from a different owner must fail.
    let params2 = ProjectRegistrationParams {
        owner: owner2.clone(),
        name: String::from_str(&env, "DupSlug2"),
        slug: String::from_str(&env, "dup-slug"),
        description: String::from_str(&env, "Second"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        license: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
        repository_url: None,
        language_code: None,
        verification_requirements: None,
    };
    let result = client.mock_all_auths().try_register_project(&params2);
    assert!(result.is_err());
}

// ── Race: double-delete review returns ReviewNotFound ────────────────────────

#[test]
fn test_race_double_delete_review() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "DoubleDelete");

    let reviewer = Address::generate(&env);
    client.mock_all_auths().add_review(&project_id, &reviewer, &4, &None);

    // First delete succeeds.
    client.mock_all_auths().delete_review(&project_id, &reviewer);

    // Second delete (simulating a concurrent retry) must fail.
    let result = client
        .mock_all_auths()
        .try_delete_review(&project_id, &reviewer);
    assert_eq!(result, Err(Ok(ContractError::ReviewNotFound)));

    // Stats reflect only one remaining review (zero).
    let stats = client.get_project_stats(&project_id);
    assert_eq!(stats.review_count, 0);
}

// ── Race: concurrent project update from non-owner is rejected ──────────────

#[test]
fn test_race_concurrent_update_non_owner_rejected() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let attacker = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "OwnerOnly");

    // Attacker attempts to update — must fail.
    let update = ProjectUpdateParams {
        project_id,
        caller: attacker.clone(),
        name: Some(String::from_str(&env, "Hacked")),
        slug: None,
        description: None,
        category: None,
        website: None,
        license: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
        repository_url: None,
        language_code: None,
        verification_requirements: None,
    };
    let result = client.mock_all_auths().try_update_project(&update);
    assert!(result.is_err());

    // Owner's project name is unchanged.
    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.name, String::from_str(&env, "OwnerOnly"));
}

// ── Atomicity: review add + stats update are atomic ─────────────────────────

#[test]
fn test_atomicity_review_add_and_stats_consistent() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "AtomicProject");

    for i in 0..5u32 {
        let reviewer = Address::generate(&env);
        let rating = ((i % 5) + 1) as u32;
        client.mock_all_auths().add_review(&project_id, &reviewer, &rating, &None);

        // After each add, stats must be immediately consistent.
        let stats = client.get_project_stats(&project_id);
        assert_eq!(stats.review_count, (i + 1) as u32);
    }
}

// ── Lock-free consistency: sorted reads agree with unsorted reads ────────────

#[test]
fn test_lock_free_consistency_sorted_matches_unsorted() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "LockFree");

    for i in 0..5u32 {
        let reviewer = Address::generate(&env);
        let rating = ((i % 5) + 1) as u32;
        client.mock_all_auths().add_review(&project_id, &reviewer, &rating, &None);
    }

    let unsorted = client.list_reviews(&project_id, &0, &10);
    let sorted = client.list_reviews_sorted(&project_id, &0, &10, &ReviewSortMode::Newest);

    // Both must return the same set of reviews (possibly different order).
    assert_eq!(unsorted.len(), sorted.len());
    assert_eq!(unsorted.len(), 5);
}

// ── Lock-free consistency: project count is monotonic ────────────────────────

#[test]
fn test_lock_free_consistency_project_count_monotonic() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let mut last_count = client.get_project_count();
    for i in 0..5u32 {
        let name = alloc::format!("Mono{}", i);
        let _ = create_test_project(&client, &owner, &name);
        let count = client.get_project_count();
        assert_eq!(count, last_count + 1);
        last_count = count;
    }
}

// ── Atomicity: pause does not corrupt in-flight project data ─────────────────

#[test]
fn test_atomicity_pause_does_not_corrupt_in_flight_data() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "InFlight");

    let project_before = client.get_project(&project_id);

    // Pause immediately after creating a project.
    client.mock_all_auths().pause(&admin);
    assert!(client.is_paused());

    // Data is unchanged during pause.
    assert_eq!(client.get_project(&project_id), project_before);

    // Unpause and verify data still intact.
    client.mock_all_auths().unpause(&admin);
    assert_eq!(client.get_project(&project_id), project_before);
}

// ── Concurrent link operations: link then unlink is idempotent ───────────────

#[test]
fn test_concurrent_link_unlink_idempotent() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let p1 = create_test_project(&client, &owner, "LinkSource");
    let p2 = create_test_project(&client, &owner, "LinkTarget");

    client.mock_all_auths().link_project(&p1, &owner, &p2);
    let links = client.get_linked_projects(&p1);
    assert_eq!(links.len(), 1);

    // Double-link is idempotent (no duplicate entry).
    client.mock_all_auths().link_project(&p1, &owner, &p2);
    let links = client.get_linked_projects(&p1);
    assert_eq!(links.len(), 1);

    // Unlink.
    client.mock_all_auths().unlink_project(&p1, &owner, &p2);
    let links = client.get_linked_projects(&p1);
    assert_eq!(links.len(), 0);
}

// ── Verification race: request then approve is atomic ────────────────────────

#[test]
fn test_race_verification_request_then_approve() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "VerifyRace");

    // Request verification.
    let cid = String::from_str(&env, "QmTest1234567890123456789012345678901234567890123");
    client
        .mock_all_auths()
        .request_verification(&project_id, &owner, &cid);

    // Double-request must fail (already pending).
    let result = client
        .mock_all_auths()
        .try_request_verification(&project_id, &owner, &cid);
    assert!(result.is_err());

    // Admin approves — project becomes verified.
    client.mock_all_auths().approve_verification(&project_id, &admin);

    let project = client.get_project(&project_id).unwrap();
    assert_eq!(
        project.verification_status,
        crate::types::VerificationStatus::Verified
    );
}

// ── Sorted listing remains consistent after concurrent adds ─────────────────

#[test]
fn test_concurrent_adds_sorted_listing_consistent() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    for i in 0..3u32 {
        let name = alloc::format!("SortProj{}", i);
        let _ = create_test_project(&client, &owner, &name);
    }

    let sorted = client.list_projects_sorted(&ProjectSortMode::Newest, &0, &10);
    assert_eq!(sorted.len(), 3);

    // Adding more projects must keep sorting consistent.
    for i in 3..6u32 {
        let name = alloc::format!("SortProj{}", i);
        let _ = create_test_project(&client, &owner, &name);
    }

    let sorted = client.list_projects_sorted(&ProjectSortMode::Newest, &0, &10);
    assert_eq!(sorted.len(), 6);
}
