//! Security invariant tests for critical contract properties.
//!
//! Each test verifies a global consistency property that must hold after any
//! sequence of operations, not just a single "happy path" scenario.

use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::VerificationStatus;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

// ── Invariant: Stats consistency ─────────────────────────────────────────────
//
// After every review operation the stored ProjectStats must agree with the
// set of live reviews: rating_sum == Σ(ratings), review_count == |reviews|,
// average_rating == rating_sum / review_count (integer division).

#[test]
fn invariant_stats_sum_and_count_match_submitted_reviews() {
    let env = Env::default();
    let (client, _) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "StatsInvariant");

    let ratings = [5u32, 3, 4, 2, 5];
    let mut expected_sum: u64 = 0;
    for &r in &ratings {
        let reviewer = Address::generate(&env);
        client
            .mock_all_auths()
            .add_review(&project_id, &reviewer, &r, &None);
        expected_sum += r as u64;
    }

    let stats = client.get_project_stats(&project_id);
    assert_eq!(
        stats.review_count,
        ratings.len() as u32,
        "review_count must equal the number of reviews submitted"
    );
    assert_eq!(
        stats.rating_sum, expected_sum,
        "rating_sum must equal the sum of all submitted ratings"
    );
    let expected_avg = (expected_sum / ratings.len() as u64) as u32;
    assert_eq!(
        stats.average_rating, expected_avg,
        "average_rating must be rating_sum / review_count (integer division)"
    );
}

#[test]
fn invariant_stats_remain_consistent_after_review_deletion() {
    let env = Env::default();
    let (client, _) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "StatsDeletionInvariant");

    let reviewer1 = Address::generate(&env);
    let reviewer2 = Address::generate(&env);

    client
        .mock_all_auths()
        .add_review(&project_id, &reviewer1, &5, &None);
    client
        .mock_all_auths()
        .add_review(&project_id, &reviewer2, &3, &None);

    client
        .mock_all_auths()
        .delete_review(&project_id, &reviewer1);

    let stats = client.get_project_stats(&project_id);
    assert_eq!(
        stats.review_count, 1,
        "review_count must decrement after a review is deleted"
    );
    assert_eq!(
        stats.rating_sum, 3,
        "rating_sum must decrease by the deleted review's rating"
    );
    assert_eq!(
        stats.average_rating, 3,
        "average_rating must reflect only the remaining reviews"
    );
}

#[test]
fn invariant_stats_zero_for_project_with_no_reviews() {
    let env = Env::default();
    let (client, _) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "EmptyStatsInvariant");

    let stats = client.get_project_stats(&project_id);
    assert_eq!(stats.review_count, 0, "new project must have zero reviews");
    assert_eq!(stats.rating_sum, 0, "new project must have zero rating_sum");
    assert_eq!(
        stats.average_rating, 0,
        "new project must have zero average_rating"
    );
}

// ── Invariant: Owner index consistency ───────────────────────────────────────
//
// Every project registered by an owner must appear in get_projects_by_owner,
// and projects from different owners must not cross-contaminate each other's
// index.

#[test]
fn invariant_owner_index_contains_all_registered_projects() {
    let env = Env::default();
    let (client, _) = setup_contract(&env);
    let owner = Address::generate(&env);

    let id1 = create_test_project(&client, &owner, "IndexA");
    let id2 = create_test_project(&client, &owner, "IndexB");
    let id3 = create_test_project(&client, &owner, "IndexC");

    let owner_projects = client.get_projects_by_owner(&owner);
    assert_eq!(
        owner_projects.len(),
        3,
        "owner index must contain all 3 registered projects"
    );

    let mut found1 = false;
    let mut found2 = false;
    let mut found3 = false;
    for p in owner_projects.iter() {
        if p.id == id1 {
            found1 = true;
        }
        if p.id == id2 {
            found2 = true;
        }
        if p.id == id3 {
            found3 = true;
        }
    }
    assert!(found1, "project {} missing from owner index", id1);
    assert!(found2, "project {} missing from owner index", id2);
    assert!(found3, "project {} missing from owner index", id3);
}

#[test]
fn invariant_owner_indexes_are_isolated_per_owner() {
    let env = Env::default();
    let (client, _) = setup_contract(&env);
    let owner1 = Address::generate(&env);
    let owner2 = Address::generate(&env);

    let id1 = create_test_project(&client, &owner1, "Owner1Project");
    let id2 = create_test_project(&client, &owner2, "Owner2Project");

    let projects1 = client.get_projects_by_owner(&owner1);
    let projects2 = client.get_projects_by_owner(&owner2);

    assert_eq!(projects1.len(), 1, "owner1 index must contain exactly 1 project");
    assert_eq!(projects2.len(), 1, "owner2 index must contain exactly 1 project");

    assert_eq!(
        projects1.get(0).unwrap().id,
        id1,
        "owner1 index must contain only owner1's project"
    );
    assert_eq!(
        projects2.get(0).unwrap().id,
        id2,
        "owner2 index must contain only owner2's project"
    );
}

#[test]
fn invariant_owner_index_count_matches_get_projects_by_owner() {
    let env = Env::default();
    let (client, _) = setup_contract(&env);
    let owner = Address::generate(&env);

    create_test_project(&client, &owner, "CountA");
    create_test_project(&client, &owner, "CountB");

    let count = client.get_owner_project_count(&owner);
    let projects = client.get_projects_by_owner(&owner);

    assert_eq!(
        count,
        projects.len(),
        "get_owner_project_count must agree with get_projects_by_owner length"
    );
}

// ── Invariant: Verification status / record consistency ──────────────────────
//
// After any verification state transition the project's verification_status
// field must match the outcome of that transition; calling get_verification
// must return a record consistent with that status.

#[test]
fn invariant_verification_status_is_pending_after_request() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "VerifyPendingInvariant");

    client
        .mock_all_auths()
        .set_min_project_age(&admin, &0);

    let evidence = String::from_str(&env, "QmTestEvidenceCid123456789012345678901234567890");
    client
        .mock_all_auths()
        .request_verification(&project_id, &owner, &evidence);

    let project = client.get_project(&project_id).expect("project must exist");
    assert_eq!(
        project.verification_status,
        VerificationStatus::Pending,
        "verification_status must be Pending immediately after request"
    );

    let record = client
        .get_verification(&project_id)
        .expect("verification record must exist after request");
    assert_eq!(
        record.status,
        VerificationStatus::Pending,
        "verification record status must agree with project status"
    );
    assert_eq!(
        record.project_id, project_id,
        "verification record must reference the correct project"
    );
}

#[test]
fn invariant_verification_status_is_verified_after_approval() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "VerifyApprovedInvariant");

    client
        .mock_all_auths()
        .set_min_project_age(&admin, &0);

    let evidence = String::from_str(&env, "QmTestEvidenceCid123456789012345678901234567890");
    client
        .mock_all_auths()
        .request_verification(&project_id, &owner, &evidence);
    client
        .mock_all_auths()
        .approve_verification(&project_id, &admin);

    let project = client.get_project(&project_id).expect("project must exist");
    assert_eq!(
        project.verification_status,
        VerificationStatus::Verified,
        "verification_status must be Verified after admin approval"
    );

    let record = client
        .get_verification(&project_id)
        .expect("verification record must persist after approval");
    assert_eq!(
        record.status,
        VerificationStatus::Verified,
        "verification record status must agree with project status after approval"
    );
}

#[test]
fn invariant_verification_status_is_rejected_after_rejection() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "VerifyRejectedInvariant");

    client
        .mock_all_auths()
        .set_min_project_age(&admin, &0);

    let evidence = String::from_str(&env, "QmTestEvidenceCid123456789012345678901234567890");
    client
        .mock_all_auths()
        .request_verification(&project_id, &owner, &evidence);
    client
        .mock_all_auths()
        .reject_verification(&project_id, &admin);

    let project = client.get_project(&project_id).expect("project must exist");
    assert_eq!(
        project.verification_status,
        VerificationStatus::Rejected,
        "verification_status must be Rejected after admin rejection"
    );

    let record = client
        .get_verification(&project_id)
        .expect("verification record must persist after rejection");
    assert_eq!(
        record.status,
        VerificationStatus::Rejected,
        "verification record status must agree with project status after rejection"
    );
}

// ── Invariant: Admin count consistency ───────────────────────────────────────
//
// get_admin_list().len() must always equal get_admin_count(), regardless of
// how many admins have been added or removed.

#[test]
fn invariant_admin_list_length_matches_count_at_initialization() {
    let env = Env::default();
    let (client, _) = setup_contract(&env);

    let list = client.get_admin_list();
    let count = client.get_admin_count();
    assert_eq!(
        list.len(),
        count,
        "admin list length must equal admin count at initialization"
    );
    assert_eq!(count, 1, "contract starts with exactly one admin");
}

#[test]
fn invariant_admin_count_stays_in_sync_after_add() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let new_admin = Address::generate(&env);

    client
        .mock_all_auths()
        .add_admin(&admin, &new_admin);

    let list = client.get_admin_list();
    let count = client.get_admin_count();
    assert_eq!(
        list.len(),
        count,
        "admin list and count must stay in sync after add_admin"
    );
    assert_eq!(count, 2, "two admins after initialization + add_admin");
}

#[test]
fn invariant_admin_count_stays_in_sync_after_removal() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let new_admin = Address::generate(&env);

    client
        .mock_all_auths()
        .add_admin(&admin, &new_admin);
    client
        .mock_all_auths()
        .remove_admin(&admin, &new_admin);

    let list = client.get_admin_list();
    let count = client.get_admin_count();
    assert_eq!(
        list.len(),
        count,
        "admin list and count must stay in sync after remove_admin"
    );
    assert_eq!(count, 1, "only the original admin remains after removal");
}

#[test]
fn invariant_cannot_remove_last_admin() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let result = client
        .mock_all_auths()
        .try_remove_admin(&admin, &admin);

    assert!(
        result.is_err(),
        "removing the last admin must return an error"
    );

    let count = client.get_admin_count();
    assert_eq!(
        count, 1,
        "admin count must remain 1 after failed last-admin removal"
    );
}
