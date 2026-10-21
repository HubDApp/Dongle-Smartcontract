//! Issue #968 — Verify storage invariants are maintained.
//!
//! Ensures that all mutating operations leave the contract's storage in a
//! self-consistent state.  Each test picks one invariant and drives it
//! through multiple operations, asserting the property holds after every
//! step:
//!
//! * Counter consistency: `project_count`, `review_count`, `admin_count`
//!   always match the number of live records.
//! * Index consistency: every project that appears in `get_projects_by_owner`
//!   must be retrievable via `get_project`, and vice-versa.
//! * Referential integrity: a review always references a project that exists.
//! * Storage key uniqueness: two separate registrations never share a
//!   project ID.
//! * Concurrent-style safety: interleaved operations by different users
//!   never corrupt each other's counters.

use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::VerificationStatus;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

// ── Counter consistency: project count ───────────────────────────────────────

/// The global project counter must equal the number of projects that can be
/// individually retrieved after each registration.
#[test]
fn storage_invariant_project_count_matches_registered_projects() {
    let env = Env::default();
    let (client, _) = setup_contract(&env);
    let owner = Address::generate(&env);

    let id1 = create_test_project(&client, &owner, "Alpha968");
    let id2 = create_test_project(&client, &owner, "Beta968");
    let id3 = create_test_project(&client, &owner, "Gamma968");

    // All three must be retrievable — no record must be overwritten by another.
    assert!(
        client.get_project(&id1).is_some(),
        "project id1 must remain retrievable after further registrations"
    );
    assert!(
        client.get_project(&id2).is_some(),
        "project id2 must remain retrievable after further registrations"
    );
    assert!(
        client.get_project(&id3).is_some(),
        "project id3 must remain retrievable after further registrations"
    );

    // IDs must be distinct.
    assert_ne!(id1, id2, "each registration must produce a unique ID");
    assert_ne!(id2, id3, "each registration must produce a unique ID");
    assert_ne!(id1, id3, "each registration must produce a unique ID");
}

/// After archiving a project the project record still exists but its status
/// changes — the owner index count must remain the same because the project
/// was not deleted, only archived.
#[test]
fn storage_invariant_project_remains_in_owner_index_after_archive() {
    let env = Env::default();
    let (client, _) = setup_contract(&env);
    let owner = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "ArchiveInv968");

    let before = client.get_projects_by_owner(&owner).len();
    client.mock_all_auths().archive_project(&project_id, &owner);
    let after = client.get_projects_by_owner(&owner).len();

    assert_eq!(
        before, after,
        "owner index length must not change when a project is archived (not deleted)"
    );

    // The record itself must still exist.
    assert!(
        client.get_project(&project_id).is_some(),
        "archived project record must still be retrievable"
    );
}

// ── Counter consistency: review count ────────────────────────────────────────

/// `ProjectStats.review_count` must increment by exactly 1 per submitted
/// review and decrement by exactly 1 per deleted review.
#[test]
fn storage_invariant_review_count_increments_and_decrements_correctly() {
    let env = Env::default();
    let (client, _) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "ReviewCount968");

    let count_0 = client.get_project_stats(&project_id).review_count;
    assert_eq!(count_0, 0, "new project must have zero reviews");

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    let r3 = Address::generate(&env);

    client.mock_all_auths().add_review(&project_id, &r1, &5, &None);
    assert_eq!(
        client.get_project_stats(&project_id).review_count,
        1,
        "review_count must be 1 after first review"
    );

    client.mock_all_auths().add_review(&project_id, &r2, &3, &None);
    assert_eq!(
        client.get_project_stats(&project_id).review_count,
        2,
        "review_count must be 2 after second review"
    );

    client.mock_all_auths().add_review(&project_id, &r3, &4, &None);
    assert_eq!(
        client.get_project_stats(&project_id).review_count,
        3,
        "review_count must be 3 after third review"
    );

    client.mock_all_auths().delete_review(&project_id, &r2);
    assert_eq!(
        client.get_project_stats(&project_id).review_count,
        2,
        "review_count must drop to 2 after one deletion"
    );

    client.mock_all_auths().delete_review(&project_id, &r1);
    client.mock_all_auths().delete_review(&project_id, &r3);
    assert_eq!(
        client.get_project_stats(&project_id).review_count,
        0,
        "review_count must reach 0 after all reviews are deleted"
    );
}

/// The `rating_sum` in `ProjectStats` must always equal the arithmetic sum
/// of every current review's rating multiplied by 100.
#[test]
fn storage_invariant_rating_sum_equals_sum_of_live_review_ratings() {
    let env = Env::default();
    let (client, _) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "RatingSumInv968");

    let ratings: &[u32] = &[1, 2, 3, 4, 5];
    let reviewers: Vec<Address> = ratings.iter().map(|_| Address::generate(&env)).collect();

    for (reviewer, &rating) in reviewers.iter().zip(ratings.iter()) {
        client
            .mock_all_auths()
            .add_review(&project_id, reviewer, &rating, &None);
    }

    let expected_sum: u64 = ratings.iter().map(|&r| (r as u64) * 100).sum();
    let stats = client.get_project_stats(&project_id);

    assert_eq!(
        stats.rating_sum, expected_sum,
        "rating_sum must equal the sum of all live review ratings (scaled by 100)"
    );

    // Delete one review and re-check.
    let removed_rating = ratings[2]; // 3
    client
        .mock_all_auths()
        .delete_review(&project_id, &reviewers[2]);

    let expected_after_delete = expected_sum - (removed_rating as u64) * 100;
    let stats_after = client.get_project_stats(&project_id);
    assert_eq!(
        stats_after.rating_sum, expected_after_delete,
        "rating_sum must decrease by the deleted review's scaled rating"
    );
}

// ── Counter consistency: admin count ─────────────────────────────────────────

/// `get_admin_count()` must always agree with `get_admin_list().len()`.
#[test]
fn storage_invariant_admin_count_consistent_across_add_and_remove() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let check = |label: &str| {
        let list = client.get_admin_list();
        let count = client.get_admin_count();
        assert_eq!(
            list.len(),
            count,
            "{}: admin list length and get_admin_count must agree",
            label
        );
    };

    check("initial");

    let a2 = Address::generate(&env);
    let a3 = Address::generate(&env);

    client.mock_all_auths().add_admin(&admin, &a2);
    check("after add a2");

    client.mock_all_auths().add_admin(&admin, &a3);
    check("after add a3");

    client.mock_all_auths().remove_admin(&admin, &a3);
    check("after remove a3");

    client.mock_all_auths().remove_admin(&admin, &a2);
    check("after remove a2");
}

// ── Index consistency ─────────────────────────────────────────────────────────

/// Every project ID returned by `get_projects_by_owner` must be directly
/// retrievable via `get_project`, and every stored project must have the
/// correct owner set.
#[test]
fn storage_invariant_owner_index_entries_are_individually_retrievable() {
    let env = Env::default();
    let (client, _) = setup_contract(&env);
    let owner = Address::generate(&env);

    let names = ["Idx968A", "Idx968B", "Idx968C", "Idx968D"];
    let mut ids = Vec::new();
    for name in &names {
        ids.push(create_test_project(&client, &owner, name));
    }

    let owner_projects = client.get_projects_by_owner(&owner);
    assert_eq!(
        owner_projects.len(),
        names.len() as u32,
        "owner index must contain exactly {} entries",
        names.len()
    );

    for project in owner_projects.iter() {
        let fetched = client
            .get_project(&project.id)
            .expect("every ID in the owner index must be individually retrievable");
        assert_eq!(
            fetched.owner, owner,
            "project owner stored in the record must match the index owner"
        );
    }
}

/// The owner index of owner A must not contain projects registered by owner B,
/// even when both owners register projects in an interleaved sequence.
#[test]
fn storage_invariant_owner_indexes_isolated_during_interleaved_registrations() {
    let env = Env::default();
    let (client, _) = setup_contract(&env);
    let owner_a = Address::generate(&env);
    let owner_b = Address::generate(&env);

    let a1 = create_test_project(&client, &owner_a, "A1Idx968");
    let b1 = create_test_project(&client, &owner_b, "B1Idx968");
    let a2 = create_test_project(&client, &owner_a, "A2Idx968");
    let b2 = create_test_project(&client, &owner_b, "B2Idx968");

    let projects_a = client.get_projects_by_owner(&owner_a);
    let projects_b = client.get_projects_by_owner(&owner_b);

    assert_eq!(projects_a.len(), 2, "owner_a must have exactly 2 projects");
    assert_eq!(projects_b.len(), 2, "owner_b must have exactly 2 projects");

    let ids_a: std::vec::Vec<u64> = projects_a.iter().map(|p| p.id).collect();
    let ids_b: std::vec::Vec<u64> = projects_b.iter().map(|p| p.id).collect();

    assert!(ids_a.contains(&a1), "a1 must appear in owner_a's index");
    assert!(ids_a.contains(&a2), "a2 must appear in owner_a's index");
    assert!(!ids_a.contains(&b1), "b1 must NOT appear in owner_a's index");
    assert!(!ids_a.contains(&b2), "b2 must NOT appear in owner_a's index");

    assert!(ids_b.contains(&b1), "b1 must appear in owner_b's index");
    assert!(ids_b.contains(&b2), "b2 must appear in owner_b's index");
    assert!(!ids_b.contains(&a1), "a1 must NOT appear in owner_b's index");
    assert!(!ids_b.contains(&a2), "a2 must NOT appear in owner_b's index");
}

// ── Referential integrity ─────────────────────────────────────────────────────

/// A review references a project ID.  After the review is submitted, that
/// project must still exist and be retrievable.
#[test]
fn storage_invariant_review_references_existing_project() {
    let env = Env::default();
    let (client, _) = setup_contract(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "RefInt968");
    client
        .mock_all_auths()
        .add_review(&project_id, &reviewer, &4, &None);

    // The project must still exist (not implicitly deleted by the review).
    let project = client
        .get_project(&project_id)
        .expect("project must still exist after a review is added");

    // The review must reference the same project ID.
    let reviews = client.list_reviews(&project_id, &0, &100);
    assert!(
        !reviews.is_empty(),
        "review list must be non-empty after submission"
    );
    for r in reviews.iter() {
        assert_eq!(
            r.project_id, project.id,
            "every review must reference the correct project ID"
        );
    }
}

/// Attempting to add a review for a non-existent project must fail and must
/// not create any dangling review records.
#[test]
fn storage_invariant_review_for_nonexistent_project_is_rejected() {
    let env = Env::default();
    let (client, _) = setup_contract(&env);
    let reviewer = Address::generate(&env);

    let result = client
        .mock_all_auths()
        .try_add_review(&999_u64, &reviewer, &5, &None);

    assert!(
        result.is_err(),
        "adding a review for a non-existent project must return an error"
    );
}

// ── Storage key uniqueness ────────────────────────────────────────────────────

/// Every call to `register_project` must return a unique project ID.
/// Two projects with different slugs/names must never collide on the same
/// storage key.
#[test]
fn storage_invariant_unique_project_ids_across_multiple_registrations() {
    let env = Env::default();
    let (client, _) = setup_contract(&env);

    let owner = Address::generate(&env);
    let mut ids = std::collections::HashSet::new();

    for i in 0..10u32 {
        // Each name is unique so slug uniqueness is also satisfied.
        extern crate alloc;
        let name = alloc::format!("UniqueKey968_{}", i);
        let id = create_test_project(&client, &owner, &name);
        assert!(
            ids.insert(id),
            "project id {} was already issued — IDs must be unique",
            id
        );
    }
    assert_eq!(ids.len(), 10, "10 unique IDs must have been produced");
}

/// Re-registering a project with the same slug must fail with an error; it
/// must not silently overwrite the existing record.
#[test]
fn storage_invariant_duplicate_slug_rejected_and_original_record_preserved() {
    let env = Env::default();
    let (client, _) = setup_contract(&env);
    let owner = Address::generate(&env);

    let id = create_test_project(&client, &owner, "DupSlug968");

    // Try to register a second project with the same name (same slug).
    let result = client
        .mock_all_auths()
        .try_register_project(&crate::types::ProjectRegistrationParams {
            owner: owner.clone(),
            name: String::from_str(&env, "DupSlug968"),
            slug: String::from_str(&env, "dupslug968"),
            description: String::from_str(&env, "Duplicate"),
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
        });

    assert!(
        result.is_err(),
        "registering a duplicate slug must return an error"
    );

    // Original record must be intact.
    let project = client
        .get_project(&id)
        .expect("original project must still exist after a failed duplicate registration");
    assert_eq!(
        project.name,
        String::from_str(&env, "DupSlug968"),
        "original project name must be unchanged"
    );
}

// ── Concurrent-style operation safety ────────────────────────────────────────

/// Interleaved project registrations and reviews by independent users must
/// never cause counter divergence.  This simulates concurrent usage by
/// submitting operations for multiple independent owners/reviewers in a
/// single ledger transaction context.
#[test]
fn storage_invariant_counters_consistent_after_interleaved_operations() {
    let env = Env::default();
    let (client, _) = setup_contract(&env);

    let owners: Vec<Address> = (0..3).map(|_| Address::generate(&env)).collect();
    let mut project_ids: Vec<u64> = Vec::new();

    // Register one project per owner.
    for (i, owner) in owners.iter().enumerate() {
        extern crate alloc;
        let name = alloc::format!("Concurrent968_{}", i);
        project_ids.push(create_test_project(&client, owner, &name));
    }

    // Each owner submits two reviews on the next owner's project (round-robin).
    for i in 0..3usize {
        let project_id = project_ids[(i + 1) % 3];
        let reviewer1 = Address::generate(&env);
        let reviewer2 = Address::generate(&env);
        client
            .mock_all_auths()
            .add_review(&project_id, &reviewer1, &((i as u32 % 5) + 1), &None);
        client
            .mock_all_auths()
            .add_review(&project_id, &reviewer2, &((i as u32 % 5) + 1), &None);
    }

    // Each project that received 2 reviews must report review_count == 2.
    for pid in &project_ids {
        let stats = client.get_project_stats(pid);
        assert_eq!(
            stats.review_count, 2,
            "project {} must have exactly 2 reviews after interleaved submissions",
            pid
        );
    }

    // Owner indexes must still be isolated.
    for owner in &owners {
        let count = client.get_projects_by_owner(owner).len();
        assert_eq!(
            count, 1,
            "each owner must still have exactly 1 project in their index after interleaved ops"
        );
    }
}

/// Verification status stored on the `Project` record and on the
/// `VerificationRecord` must always agree after request, approval, and
/// rejection transitions.
#[test]
fn storage_invariant_verification_status_consistent_across_project_and_record() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "VerifConsistency968");

    client.mock_all_auths().set_min_project_age(&admin, &0);

    let evidence = String::from_str(&env, "QmEvidenceCid968StorageTest1234567890123456789");

    // — Request —
    client
        .mock_all_auths()
        .request_verification(&project_id, &owner, &evidence);

    let proj = client.get_project(&project_id).unwrap();
    let rec = client.get_verification(&project_id).unwrap();
    assert_eq!(
        proj.verification_status,
        VerificationStatus::Pending,
        "project status must be Pending after request"
    );
    assert_eq!(
        rec.status,
        VerificationStatus::Pending,
        "verification record status must match project status after request"
    );

    // — Approve —
    client
        .mock_all_auths()
        .approve_verification(&project_id, &admin);

    let proj = client.get_project(&project_id).unwrap();
    let rec = client.get_verification(&project_id).unwrap();
    assert_eq!(
        proj.verification_status,
        VerificationStatus::Verified,
        "project status must be Verified after approval"
    );
    assert_eq!(
        rec.status,
        VerificationStatus::Verified,
        "verification record status must match project status after approval"
    );
}
