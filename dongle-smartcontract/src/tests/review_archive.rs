//! Tests for issue #804: archive old reviews to cheaper storage with query access.
//!
//! Coverage:
//! - Admin can archive reviews older than 2 years
//! - Recent reviews are not archived
//! - Archived reviews are removed from active listing but queryable via get_archived_review
//! - list_archived_reviews paginates correctly
//! - Project stats (rating_sum, review_count) are preserved after archival
//! - User review index is cleaned up after archival
//! - Non-admin cannot call archive_old_reviews
//! - set_archived_review_arweave_tx records the Arweave TX id
//! - set_archived_review_arweave_tx requires admin
//! - get_archived_review returns None for non-archived reviews
//! - get_archived_review returns None for reviews that were never submitted
//! - batch_size capping: only up to MAX_ARCHIVE_BATCH_SIZE reviews per call
//! - Archiving a project that does not exist returns ProjectNotFound
//! - Multiple archive calls process remaining reviews
//! - Archived review's ArchivedReview fields match original Review fields

use crate::constants::REVIEW_ARCHIVE_AGE_SECONDS;
use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::DongleContractClient;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

// ── Helper ────────────────────────────────────────────────────────────────────

fn advance_time(env: &Env, seconds: u64) {
    let current = env.ledger().timestamp();
    env.ledger().set_timestamp(current + seconds);
}

/// Submit a review at the current ledger time and return the reviewer address.
fn submit_review(client: &DongleContractClient<'_>, project_id: u64, rating: u32) -> Address {
    let env = &client.env;
    let reviewer = Address::generate(env);
    client.add_review(&project_id, &reviewer, &rating, &None);
    reviewer
}

// ── archive_old_reviews ───────────────────────────────────────────────────────

#[test]
fn test_archive_reviews_older_than_two_years() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let project_id = create_test_project(&client, &admin, "OldReviewProject");

    // Submit a review at time T=0
    let reviewer = submit_review(&client, project_id, 4);

    // Advance past the 2-year threshold
    advance_time(&env, REVIEW_ARCHIVE_AGE_SECONDS + 1);

    // Active review should exist before archival
    assert!(client.get_review(&project_id, &reviewer).is_some());
    assert_eq!(client.list_reviews(&project_id, &0, &100).len(), 1);

    let archived_count = client.archive_old_reviews(&admin, &project_id, &50);
    assert_eq!(archived_count, 1);

    // Review should no longer appear in the active list
    assert_eq!(client.list_reviews(&project_id, &0, &100).len(), 0);
    // get_review on primary storage returns None after archival
    assert!(client.get_review(&project_id, &reviewer).is_none());
}

#[test]
fn test_recent_review_not_archived() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let project_id = create_test_project(&client, &admin, "NewReviewProject");

    // Submit a review — do NOT advance time past the threshold
    let reviewer = submit_review(&client, project_id, 5);
    advance_time(&env, REVIEW_ARCHIVE_AGE_SECONDS - 1); // just under threshold

    let archived_count = client.archive_old_reviews(&admin, &project_id, &50);
    assert_eq!(archived_count, 0);

    // Review should still be active
    assert!(client.get_review(&project_id, &reviewer).is_some());
    assert_eq!(client.list_reviews(&project_id, &0, &100).len(), 1);
}

#[test]
fn test_mixed_old_and_new_reviews_only_old_archived() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let project_id = create_test_project(&client, &admin, "MixedProject");

    // Old review: submitted at T=0
    let old_reviewer = submit_review(&client, project_id, 3);

    // Advance to just before the threshold and submit a recent review
    advance_time(&env, REVIEW_ARCHIVE_AGE_SECONDS - 1_000);
    let recent_reviewer = submit_review(&client, project_id, 5);

    // Advance past the threshold relative to T=0 so old_reviewer is eligible
    advance_time(&env, 2_000); // total now = REVIEW_ARCHIVE_AGE_SECONDS + 1_000

    let archived_count = client.archive_old_reviews(&admin, &project_id, &50);
    assert_eq!(archived_count, 1);

    // Old review is archived
    assert!(client.get_review(&project_id, &old_reviewer).is_none());
    assert!(client
        .get_archived_review(&project_id, &old_reviewer)
        .is_some());

    // Recent review is still active
    assert!(client.get_review(&project_id, &recent_reviewer).is_some());
    assert_eq!(client.list_reviews(&project_id, &0, &100).len(), 1);
}

#[test]
fn test_non_admin_cannot_archive() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let project_id = create_test_project(&client, &admin, "AuthGuardProject");

    submit_review(&client, project_id, 4);
    advance_time(&env, REVIEW_ARCHIVE_AGE_SECONDS + 1);

    let stranger = Address::generate(&env);
    let result = client.try_archive_old_reviews(&stranger, &project_id, &50);
    assert_eq!(result, Err(Ok(ContractError::AdminOnly)));
}

#[test]
fn test_archive_non_existent_project_returns_error() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    let result = client.try_archive_old_reviews(&admin, &9_999_999, &50);
    assert_eq!(result, Err(Ok(ContractError::ProjectNotFound)));
}

// ── get_archived_review ───────────────────────────────────────────────────────

#[test]
fn test_get_archived_review_returns_correct_data() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let project_id = create_test_project(&client, &admin, "ArchiveDataCheck");

    let reviewer = Address::generate(&env);
    let rating: u32 = 3;
    let submitted_at = env.ledger().timestamp();
    client.add_review(&project_id, &reviewer, &rating, &None);

    advance_time(&env, REVIEW_ARCHIVE_AGE_SECONDS + 1);
    let archived_at = env.ledger().timestamp();

    client.archive_old_reviews(&admin, &project_id, &50);

    let archived = client
        .get_archived_review(&project_id, &reviewer)
        .expect("archived review should exist");

    assert_eq!(archived.project_id, project_id);
    assert_eq!(archived.reviewer, reviewer);
    assert_eq!(archived.rating, rating);
    assert_eq!(archived.created_at, submitted_at);
    assert!(archived.arweave_tx_id.is_none());
    // archived_at should be approximately when archive_old_reviews was called
    assert!(archived.archived_at >= archived_at);
}

#[test]
fn test_get_archived_review_returns_none_for_active_review() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let project_id = create_test_project(&client, &admin, "ActiveNotArchived");

    let reviewer = submit_review(&client, project_id, 5);

    // Review is active — archived slot should be empty
    assert!(client.get_archived_review(&project_id, &reviewer).is_none());
}

#[test]
fn test_get_archived_review_returns_none_for_nonexistent_review() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let project_id = create_test_project(&client, &admin, "NoReviewProject");

    let nobody = Address::generate(&env);
    assert!(client.get_archived_review(&project_id, &nobody).is_none());
}

// ── list_archived_reviews ─────────────────────────────────────────────────────

#[test]
fn test_list_archived_reviews_returns_archived_entries() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let project_id = create_test_project(&client, &admin, "ListArchiveProject");

    let r1 = submit_review(&client, project_id, 4);
    let r2 = submit_review(&client, project_id, 2);

    advance_time(&env, REVIEW_ARCHIVE_AGE_SECONDS + 1);
    client.archive_old_reviews(&admin, &project_id, &50);

    let archived = client.list_archived_reviews(&project_id, &0, &100);
    assert_eq!(archived.len(), 2);

    // Both reviewers appear in the archived list
    let reviewers: soroban_sdk::Vec<Address> = {
        let mut v = soroban_sdk::Vec::new(&env);
        for i in 0..archived.len() {
            v.push_back(archived.get(i).unwrap().reviewer);
        }
        v
    };
    assert!(reviewers.contains(&r1));
    assert!(reviewers.contains(&r2));
}

#[test]
fn test_list_archived_reviews_pagination() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let project_id = create_test_project(&client, &admin, "PaginationArchiveProject");

    // Submit 3 old reviews
    for _ in 0..3 {
        submit_review(&client, project_id, 3);
    }

    advance_time(&env, REVIEW_ARCHIVE_AGE_SECONDS + 1);
    client.archive_old_reviews(&admin, &project_id, &50);

    // Page 1: first 2
    let page1 = client.list_archived_reviews(&project_id, &0, &2);
    assert_eq!(page1.len(), 2);

    // Page 2: remaining 1
    let page2 = client.list_archived_reviews(&project_id, &2, &2);
    assert_eq!(page2.len(), 1);

    // Out of range
    let page3 = client.list_archived_reviews(&project_id, &10, &10);
    assert_eq!(page3.len(), 0);
}

#[test]
fn test_list_archived_reviews_empty_when_none_archived() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let project_id = create_test_project(&client, &admin, "NoArchivesProject");

    let archived = client.list_archived_reviews(&project_id, &0, &100);
    assert_eq!(archived.len(), 0);
}

// ── Project stats preserved after archival ────────────────────────────────────

#[test]
fn test_project_stats_preserved_after_archival() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let project_id = create_test_project(&client, &admin, "StatsPreserveProject");

    // Submit two reviews with known ratings
    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    client.add_review(&project_id, &r1, &4, &None);
    client.add_review(&project_id, &r2, &2, &None);

    let stats_before = client.get_project_stats(&project_id);
    assert_eq!(stats_before.review_count, 2);
    assert_eq!(stats_before.rating_sum, 600); // (4+2)*100

    advance_time(&env, REVIEW_ARCHIVE_AGE_SECONDS + 1);
    client.archive_old_reviews(&admin, &project_id, &50);

    // Stats should be unchanged — archived reviews still count
    let stats_after = client.get_project_stats(&project_id);
    assert_eq!(stats_after.review_count, stats_before.review_count);
    assert_eq!(stats_after.rating_sum, stats_before.rating_sum);
    assert_eq!(stats_after.average_rating, stats_before.average_rating);
}

// ── set_archived_review_arweave_tx ────────────────────────────────────────────

#[test]
fn test_set_archived_review_arweave_tx_records_id() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let project_id = create_test_project(&client, &admin, "ArweaveProject");

    let reviewer = submit_review(&client, project_id, 5);
    advance_time(&env, REVIEW_ARCHIVE_AGE_SECONDS + 1);
    client.archive_old_reviews(&admin, &project_id, &50);

    let tx_id = String::from_str(&env, "abc123ArweaveTxId");
    client.set_archived_review_arweave_tx(&admin, &project_id, &reviewer, &tx_id);

    let archived = client
        .get_archived_review(&project_id, &reviewer)
        .expect("archived record should exist");
    assert_eq!(archived.arweave_tx_id, Some(tx_id));
}

#[test]
fn test_set_archived_review_arweave_tx_requires_admin() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let project_id = create_test_project(&client, &admin, "ArweaveAuthProject");

    let reviewer = submit_review(&client, project_id, 5);
    advance_time(&env, REVIEW_ARCHIVE_AGE_SECONDS + 1);
    client.archive_old_reviews(&admin, &project_id, &50);

    let tx_id = String::from_str(&env, "someArweaveTx");
    let stranger = Address::generate(&env);
    let result =
        client.try_set_archived_review_arweave_tx(&stranger, &project_id, &reviewer, &tx_id);
    assert_eq!(result, Err(Ok(ContractError::AdminOnly)));
}

#[test]
fn test_set_archived_review_arweave_tx_fails_for_non_archived_review() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let project_id = create_test_project(&client, &admin, "NonArchivedArweave");

    let reviewer = submit_review(&client, project_id, 5);
    // Do NOT archive — review is still active

    let tx_id = String::from_str(&env, "anyTxId");
    let result = client.try_set_archived_review_arweave_tx(&admin, &project_id, &reviewer, &tx_id);
    assert_eq!(result, Err(Ok(ContractError::ReviewNotArchived)));
}

// ── Batch size capping ────────────────────────────────────────────────────────

#[test]
fn test_batch_size_limits_per_call_count() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let project_id = create_test_project(&client, &admin, "BatchCapProject");

    // Submit 5 old reviews
    for _ in 0..5 {
        submit_review(&client, project_id, 3);
    }

    advance_time(&env, REVIEW_ARCHIVE_AGE_SECONDS + 1);

    // Archive at most 3 per call
    let count_first = client.archive_old_reviews(&admin, &project_id, &3);
    assert_eq!(count_first, 3);
    assert_eq!(client.list_archived_reviews(&project_id, &0, &100).len(), 3);
    assert_eq!(client.list_reviews(&project_id, &0, &100).len(), 2);

    // Archive the rest
    let count_second = client.archive_old_reviews(&admin, &project_id, &3);
    assert_eq!(count_second, 2);
    assert_eq!(client.list_archived_reviews(&project_id, &0, &100).len(), 5);
    assert_eq!(client.list_reviews(&project_id, &0, &100).len(), 0);
}

#[test]
fn test_zero_batch_size_uses_max() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let project_id = create_test_project(&client, &admin, "ZeroBatchProject");

    for _ in 0..3 {
        submit_review(&client, project_id, 4);
    }
    advance_time(&env, REVIEW_ARCHIVE_AGE_SECONDS + 1);

    // batch_size = 0 should default to MAX_ARCHIVE_BATCH_SIZE (50), archiving all 3
    let count = client.archive_old_reviews(&admin, &project_id, &0);
    assert_eq!(count, 3);
}

// ── Idempotency and second-call behaviour ────────────────────────────────────

#[test]
fn test_archive_already_archived_project_is_idempotent() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let project_id = create_test_project(&client, &admin, "IdempotentArchive");

    submit_review(&client, project_id, 4);
    advance_time(&env, REVIEW_ARCHIVE_AGE_SECONDS + 1);

    // First call archives the review
    let first = client.archive_old_reviews(&admin, &project_id, &50);
    assert_eq!(first, 1);

    // Second call — nothing left to archive
    let second = client.archive_old_reviews(&admin, &project_id, &50);
    assert_eq!(second, 0);

    // Archived record is still accessible
    assert_eq!(client.list_archived_reviews(&project_id, &0, &100).len(), 1);
}

// ── User review index cleanup ─────────────────────────────────────────────────

#[test]
fn test_archived_review_removed_from_user_can_still_review_new_project() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    // Reviewer leaves a review on project A
    let project_a = create_test_project(&client, &admin, "ProjectA804");
    let reviewer = Address::generate(&env);
    client.add_review(&project_a, &reviewer, &5, &None);

    // Advance past threshold and archive
    advance_time(&env, REVIEW_ARCHIVE_AGE_SECONDS + 1);
    client.archive_old_reviews(&admin, &project_a, &50);

    // The reviewer can now review a new project (slot freed in UserReviews index)
    let project_b = create_test_project(&client, &admin, "ProjectB804");
    client.add_review(&project_b, &reviewer, &3, &None);

    let review_b = client.get_review(&project_b, &reviewer);
    assert!(review_b.is_some());
    assert_eq!(review_b.unwrap().rating, 3);
}
