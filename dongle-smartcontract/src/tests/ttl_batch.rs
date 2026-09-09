//! Tests for batch TTL extension (closes #666).
//!
//! ## Semantics under test
//!
//! - **Continue-on-missing**: missing project/review IDs are reported in
//!   `BatchTtlResult::skipped_ids`; the rest of the batch still runs.
//! - **Fail-fast on oversized batch**: returns `InvalidInput` before touching
//!   storage when the input exceeds `MAX_TTL_BATCH_SIZE`.
//! - **All-or-nothing detection**: callers can confirm full success by
//!   asserting `result.skipped_ids.len() == 0`.
//! - **Error reporting**: each skipped project ID appears in `skipped_ids`.

use crate::constants::MAX_TTL_BATCH_SIZE;
use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use soroban_sdk::{testutils::Address as _, Address, Env, Vec};

// ── Project batch ────────────────────────────────────────────────────────────

#[test]
fn batch_project_ttl_refreshes_existing_and_skips_missing() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let first = create_test_project(&client, &owner, "BulkTTLProjectA");
    let second = create_test_project(&client, &owner, "BulkTTLProjectB");

    let mut ids = Vec::new(&env);
    ids.push_back(first);
    ids.push_back(999_999); // does not exist
    ids.push_back(second);

    let result = client.extend_projects_ttl(&ids);

    // Two real projects refreshed.
    assert_eq!(result.refreshed, 2);
    // One missing project in skipped_ids.
    assert_eq!(result.skipped_ids.len(), 1);
    assert_eq!(result.skipped_ids.get(0), Some(999_999u64));
}

#[test]
fn batch_project_ttl_all_success_has_empty_skipped() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let p1 = create_test_project(&client, &owner, "BatchSuccessA");
    let p2 = create_test_project(&client, &owner, "BatchSuccessB");

    let mut ids = Vec::new(&env);
    ids.push_back(p1);
    ids.push_back(p2);

    let result = client.extend_projects_ttl(&ids);

    assert_eq!(result.refreshed, 2);
    // No skipped IDs — caller can use this to confirm all-or-nothing success.
    assert_eq!(result.skipped_ids.len(), 0);
}

#[test]
fn batch_project_ttl_all_missing_reports_all_skipped() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);

    let mut ids = Vec::new(&env);
    ids.push_back(100u64);
    ids.push_back(200u64);
    ids.push_back(300u64);

    let result = client.extend_projects_ttl(&ids);

    // Nothing refreshed.
    assert_eq!(result.refreshed, 0);
    // All three IDs reported as skipped.
    assert_eq!(result.skipped_ids.len(), 3);
}

#[test]
fn batch_project_ttl_rejects_oversized_batches() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let mut ids = Vec::new(&env);

    for i in 0..=MAX_TTL_BATCH_SIZE {
        ids.push_back(i as u64);
    }

    // Fail-fast: no storage work done, returns InvalidInput.
    let result = client.try_extend_projects_ttl(&ids);
    assert_eq!(result, Err(Ok(ContractError::InvalidInput)));
}

#[test]
fn batch_project_ttl_empty_input_succeeds_with_zero_refreshed() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);

    let ids: Vec<u64> = Vec::new(&env);
    let result = client.extend_projects_ttl(&ids);

    assert_eq!(result.refreshed, 0);
    assert_eq!(result.skipped_ids.len(), 0);
}

#[test]
fn batch_project_ttl_skipped_ids_match_missing_ids() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let existing = create_test_project(&client, &owner, "ExistingForSkipTest");
    let missing_a = 77_001u64;
    let missing_b = 77_002u64;

    let mut ids = Vec::new(&env);
    ids.push_back(missing_a);
    ids.push_back(existing);
    ids.push_back(missing_b);

    let result = client.extend_projects_ttl(&ids);

    assert_eq!(result.refreshed, 1);
    assert_eq!(result.skipped_ids.len(), 2);

    // Both missing IDs must appear in skipped_ids.
    assert!(
        result.skipped_ids.contains(missing_a),
        "missing_a should be in skipped_ids"
    );
    assert!(
        result.skipped_ids.contains(missing_b),
        "missing_b should be in skipped_ids"
    );
}

// ── Review batch ─────────────────────────────────────────────────────────────

#[test]
fn batch_review_ttl_refreshes_existing_and_skips_missing() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let missing_reviewer = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "BulkTTLReviews");

    client
        .mock_all_auths()
        .add_review(&project_id, &reviewer, &5u32, &None);

    let mut review_ids = Vec::new(&env);
    review_ids.push_back((project_id, reviewer.clone()));
    review_ids.push_back((project_id, missing_reviewer)); // does not exist

    let result = client.extend_reviews_ttl(&review_ids);

    // One real review refreshed.
    assert_eq!(result.refreshed, 1);
    // One missing review's project_id in skipped_ids.
    assert_eq!(result.skipped_ids.len(), 1);
    assert_eq!(result.skipped_ids.get(0), Some(project_id));
}

#[test]
fn batch_review_ttl_all_success_has_empty_skipped() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let reviewer1 = Address::generate(&env);
    let reviewer2 = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "BulkTTLReviewsSuccess");

    client
        .mock_all_auths()
        .add_review(&project_id, &reviewer1, &4u32, &None);
    client
        .mock_all_auths()
        .add_review(&project_id, &reviewer2, &3u32, &None);

    let mut review_ids = Vec::new(&env);
    review_ids.push_back((project_id, reviewer1.clone()));
    review_ids.push_back((project_id, reviewer2.clone()));

    let result = client.extend_reviews_ttl(&review_ids);

    assert_eq!(result.refreshed, 2);
    assert_eq!(result.skipped_ids.len(), 0);
}

#[test]
fn batch_review_ttl_rejects_oversized_batches() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let reviewer = Address::generate(&env);
    let mut review_ids = Vec::new(&env);

    for i in 0..=MAX_TTL_BATCH_SIZE {
        review_ids.push_back((i as u64, reviewer.clone()));
    }

    let result = client.try_extend_reviews_ttl(&review_ids);
    assert_eq!(result, Err(Ok(ContractError::InvalidInput)));
}

#[test]
fn batch_review_ttl_empty_input_succeeds_with_zero_refreshed() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);

    let review_ids: Vec<(u64, Address)> = Vec::new(&env);
    let result = client.extend_reviews_ttl(&review_ids);

    assert_eq!(result.refreshed, 0);
    assert_eq!(result.skipped_ids.len(), 0);
}

// ── All-or-nothing detection ─────────────────────────────────────────────────

#[test]
fn all_or_nothing_detection_via_skipped_ids() {
    // Callers that need all-or-nothing semantics can validate all IDs first,
    // or simply assert skipped_ids is empty after the call.
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let p1 = create_test_project(&client, &owner, "AON-Project-A");
    let p2 = create_test_project(&client, &owner, "AON-Project-B");

    let mut ids = Vec::new(&env);
    ids.push_back(p1);
    ids.push_back(p2);

    let result = client.extend_projects_ttl(&ids);

    // All-or-nothing check: no skips means complete success.
    let all_succeeded = result.skipped_ids.len() == 0;
    assert!(all_succeeded, "expected all IDs to be refreshed");
    assert_eq!(result.refreshed, ids.len());
}
