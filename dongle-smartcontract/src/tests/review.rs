//! Review lifecycle tests: add, update, delete, rating invariants, and ownership.

use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::DongleContractClient;
use soroban_sdk::{testutils::Address as _, Address, Env};

fn setup(env: &Env) -> (DongleContractClient, Address) {
    setup_contract(env)
}

// ---------------------------------------------------------------------------
// add_review
// ---------------------------------------------------------------------------

#[test]
fn test_add_review_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectA");

    let reviewer = Address::generate(&env);
    client.add_review(&project_id, &reviewer, &5, &None);

    let review = client.get_review(&project_id, &reviewer).unwrap();
    assert_eq!(review.rating, 5);
    assert_eq!(review.reviewer, reviewer);
    assert_eq!(review.project_id, project_id);
}

#[test]
fn test_add_review_updates_stats() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectB");

    let reviewer = Address::generate(&env);
    client.add_review(&project_id, &reviewer, &4, &None);

    let stats = client.get_project_stats(&project_id);
    assert_eq!(stats.review_count, 1);
    assert_eq!(stats.rating_sum, 400); // 4 * 100
    assert_eq!(stats.average_rating, 400); // 4.00
}

#[test]
fn test_add_two_reviews_correct_average() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectC");

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    client.add_review(&project_id, &r1, &4, &None);
    client.add_review(&project_id, &r2, &2, &None);

    let stats = client.get_project_stats(&project_id);
    assert_eq!(stats.review_count, 2);
    assert_eq!(stats.rating_sum, 600); // (4+2)*100
    assert_eq!(stats.average_rating, 300); // 3.00
}

#[test]
fn test_add_review_duplicate_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectD");

    let reviewer = Address::generate(&env);
    client.add_review(&project_id, &reviewer, &3, &None);

    let result = client.try_add_review(&project_id, &reviewer, &4, &None);
    assert_eq!(result, Err(Ok(ContractError::DuplicateReview.into())));
}

#[test]
fn test_add_review_rating_zero_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectE");

    let reviewer = Address::generate(&env);
    let result = client.try_add_review(&project_id, &reviewer, &0, &None);
    assert_eq!(result, Err(Ok(ContractError::InvalidRating.into())));
}

#[test]
fn test_add_review_rating_too_high_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectF");

    let reviewer = Address::generate(&env);
    let result = client.try_add_review(&project_id, &reviewer, &6, &None);
    assert_eq!(result, Err(Ok(ContractError::InvalidRating.into())));
}

#[test]
fn test_add_review_boundary_ratings() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectG");

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    client.add_review(&project_id, &r1, &1, &None); // min valid
    client.add_review(&project_id, &r2, &5, &None); // max valid

    let stats = client.get_project_stats(&project_id);
    assert_eq!(stats.review_count, 2);
    assert_eq!(stats.rating_sum, 600); // (1+5)*100
    assert_eq!(stats.average_rating, 300); // 3.00
}

// ---------------------------------------------------------------------------
// update_review
// ---------------------------------------------------------------------------

#[test]
fn test_update_review_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectH");

    let reviewer = Address::generate(&env);
    client.add_review(&project_id, &reviewer, &3, &None);
    client.update_review(&project_id, &reviewer, &5, &None);

    let review = client.get_review(&project_id, &reviewer).unwrap();
    assert_eq!(review.rating, 5);
}

#[test]
fn test_update_review_corrects_stats() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectI");

    let reviewer = Address::generate(&env);
    client.add_review(&project_id, &reviewer, &3, &None);

    // Update 3 → 5
    client.update_review(&project_id, &reviewer, &5, &None);

    let stats = client.get_project_stats(&project_id);
    assert_eq!(stats.review_count, 1); // count unchanged
    assert_eq!(stats.rating_sum, 500); // 5 * 100
    assert_eq!(stats.average_rating, 500); // 5.00
}

#[test]
fn test_update_review_not_found_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectJ");

    let reviewer = Address::generate(&env);
    let result = client.try_update_review(&project_id, &reviewer, &4, &None);
    assert_eq!(result, Err(Ok(ContractError::ReviewNotFound.into())));
}

#[test]
fn test_update_review_invalid_rating_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectK");

    let reviewer = Address::generate(&env);
    client.add_review(&project_id, &reviewer, &3, &None);

    let result = client.try_update_review(&project_id, &reviewer, &0, &None);
    assert_eq!(result, Err(Ok(ContractError::InvalidRating.into())));

    let result2 = client.try_update_review(&project_id, &reviewer, &6, &None);
    assert_eq!(result2, Err(Ok(ContractError::InvalidRating.into())));
}

#[test]
fn test_update_review_no_change_preserves_stats() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectL");

    let reviewer = Address::generate(&env);
    client.add_review(&project_id, &reviewer, &4, &None);
    client.update_review(&project_id, &reviewer, &4, &None); // same rating

    let stats = client.get_project_stats(&project_id);
    assert_eq!(stats.review_count, 1);
    assert_eq!(stats.rating_sum, 400);
    assert_eq!(stats.average_rating, 400);
}

// ---------------------------------------------------------------------------
// delete_review
// ---------------------------------------------------------------------------

#[test]
fn test_delete_review_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectM");

    let reviewer = Address::generate(&env);
    client.add_review(&project_id, &reviewer, &5, &None);
    client.delete_review(&project_id, &reviewer);

    let review = client.get_review(&project_id, &reviewer);
    assert!(review.is_none());
}

#[test]
fn test_delete_review_updates_stats() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectN");

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    client.add_review(&project_id, &r1, &4, &None);
    client.add_review(&project_id, &r2, &2, &None);
    client.delete_review(&project_id, &r1); // remove rating=4

    let stats = client.get_project_stats(&project_id);
    assert_eq!(stats.review_count, 1);
    assert_eq!(stats.rating_sum, 200); // only r2's rating remains
    assert_eq!(stats.average_rating, 200); // 2.00
}

#[test]
fn test_delete_last_review_zeroes_stats() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectO");

    let reviewer = Address::generate(&env);
    client.add_review(&project_id, &reviewer, &5, &None);
    client.delete_review(&project_id, &reviewer);

    // Deleting the last review must produce zero stats with no division-by-zero
    let stats = client.get_project_stats(&project_id);
    assert_eq!(stats.review_count, 0);
    assert_eq!(stats.rating_sum, 0);
    assert_eq!(stats.average_rating, 0);
}

#[test]
fn test_delete_review_not_found_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectP");

    let reviewer = Address::generate(&env);
    let result = client.try_delete_review(&project_id, &reviewer);
    assert_eq!(result, Err(Ok(ContractError::ReviewNotFound.into())));
}

// ---------------------------------------------------------------------------
// Ownership: only the original reviewer can modify/delete their review
// ---------------------------------------------------------------------------

#[test]
fn test_other_reviewer_cannot_update_another_review() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectQ");

    let reviewer_a = Address::generate(&env);
    let reviewer_b = Address::generate(&env);

    client.add_review(&project_id, &reviewer_a, &5, &None);

    // B has not submitted a review, so updating with B's address yields ReviewNotFound.
    // B cannot touch A's review because it is stored under A's address key and
    // would require A's auth signature to call update_review(reviewer=A).
    let result = client.try_update_review(&project_id, &reviewer_b, &1, &None);
    assert_eq!(result, Err(Ok(ContractError::ReviewNotFound.into())));
}

#[test]
fn test_other_reviewer_cannot_delete_another_review() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectR");

    let reviewer_a = Address::generate(&env);
    let reviewer_b = Address::generate(&env);

    client.add_review(&project_id, &reviewer_a, &5, &None);

    // B has not reviewed, so deleting with B's address yields ReviewNotFound.
    let result = client.try_delete_review(&project_id, &reviewer_b);
    assert_eq!(result, Err(Ok(ContractError::ReviewNotFound.into())));

    // A's review is untouched
    let review = client.get_review(&project_id, &reviewer_a);
    assert!(review.is_some());
}

#[test]
fn test_reviewer_can_only_touch_own_review() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectS");

    let reviewer_a = Address::generate(&env);
    let reviewer_b = Address::generate(&env);

    client.add_review(&project_id, &reviewer_a, &5, &None);
    client.add_review(&project_id, &reviewer_b, &1, &None);

    // A updates their own review — succeeds
    client.update_review(&project_id, &reviewer_a, &3, &None);

    // A's review changed; B's review unchanged
    let ra = client.get_review(&project_id, &reviewer_a).unwrap();
    let rb = client.get_review(&project_id, &reviewer_b).unwrap();
    assert_eq!(ra.rating, 3);
    assert_eq!(rb.rating, 1);
}

// ---------------------------------------------------------------------------
// Full lifecycle: add → update → delete, rating invariants hold throughout
// ---------------------------------------------------------------------------

#[test]
fn test_full_review_lifecycle_stats_invariant() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectT");

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    let r3 = Address::generate(&env);

    // Phase 1: add three reviews (ratings 5, 3, 1)
    client.add_review(&project_id, &r1, &5, &None);
    client.add_review(&project_id, &r2, &3, &None);
    client.add_review(&project_id, &r3, &1, &None);

    let stats = client.get_project_stats(&project_id);
    assert_eq!(stats.review_count, 3);
    assert_eq!(stats.rating_sum, 900); // (5+3+1)*100
    assert_eq!(stats.average_rating, 300); // 3.00

    // Phase 2: update r2's rating from 3 → 5
    client.update_review(&project_id, &r2, &5, &None);

    let stats = client.get_project_stats(&project_id);
    assert_eq!(stats.review_count, 3); // count unchanged
    assert_eq!(stats.rating_sum, 1100); // (5+5+1)*100
    assert_eq!(stats.average_rating, 366); // floor(1100/3) = 366

    // Phase 3: delete r3's review (rating=1)
    client.delete_review(&project_id, &r3);

    let stats = client.get_project_stats(&project_id);
    assert_eq!(stats.review_count, 2);
    assert_eq!(stats.rating_sum, 1000); // (5+5)*100
    assert_eq!(stats.average_rating, 500); // 5.00

    // Phase 4: delete remaining reviews — stats must be zero, no div-by-zero
    client.delete_review(&project_id, &r1);
    client.delete_review(&project_id, &r2);

    let stats = client.get_project_stats(&project_id);
    assert_eq!(stats.review_count, 0);
    assert_eq!(stats.rating_sum, 0);
    assert_eq!(stats.average_rating, 0);
}

#[test]
fn test_stats_default_to_zero_with_no_reviews() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectU");

    let stats = client.get_project_stats(&project_id);
    assert_eq!(stats.review_count, 0);
    assert_eq!(stats.rating_sum, 0);
    assert_eq!(stats.average_rating, 0);
}

#[test]
fn test_multiple_reviewers_average_precision() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectV");

    // Three reviewers: ratings 4, 5, 3 → avg = 12/3 = 4.00
    for (rating, _) in [(4u32, 0), (5, 1), (3, 2)] {
        let reviewer = Address::generate(&env);
        client.add_review(&project_id, &reviewer, &rating, &None);
    }

    let stats = client.get_project_stats(&project_id);
    assert_eq!(stats.review_count, 3);
    assert_eq!(stats.rating_sum, 1200);
    assert_eq!(stats.average_rating, 400); // exactly 4.00
}

#[test]
fn test_re_review_after_delete() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectW");

    let reviewer = Address::generate(&env);
    client.add_review(&project_id, &reviewer, &2, &None);
    client.delete_review(&project_id, &reviewer);

    // After deletion the reviewer can submit a new review
    client.add_review(&project_id, &reviewer, &4, &None);

    let stats = client.get_project_stats(&project_id);
    assert_eq!(stats.review_count, 1);
    assert_eq!(stats.rating_sum, 400);
    assert_eq!(stats.average_rating, 400);
}
