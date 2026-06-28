//! Tests for review update cooldown (#246) and delete tombstones (#240).

use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::ReviewSortMode;
use crate::DongleContractClient;
use soroban_sdk::{testutils::{Address as _, Ledger as _}, Address, Env};

fn setup(env: &Env) -> (DongleContractClient<'_>, Address) {
    setup_contract(env)
}

// ─── Cooldown (#246) ─────────────────────────────────────────────────────────

#[test]
fn test_immediate_review_update_fails_with_cooldown() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "CooldownProject");
    let reviewer = Address::generate(&env);
    client.add_review(&project_id, &reviewer, &3, &None);

    // First update succeeds (no previous update timestamp stored).
    client.update_review(&project_id, &reviewer, &4, &None);

    // Immediate second update must fail — cooldown not elapsed.
    let result = client.try_update_review(&project_id, &reviewer, &5, &None);
    assert_eq!(result, Err(Ok(ContractError::InvalidStatus)));
}

#[test]
fn test_review_update_succeeds_after_cooldown() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "CooldownPassedProject");
    let reviewer = Address::generate(&env);
    client.add_review(&project_id, &reviewer, &3, &None);

    // First update records the cooldown timestamp.
    client.update_review(&project_id, &reviewer, &4, &None);

    // Advance ledger time past the cooldown window (3601 seconds > 3600s cooldown).
    env.ledger().with_mut(|l| {
        l.timestamp += 3601;
    });

    // Update after cooldown must succeed.
    client.update_review(&project_id, &reviewer, &5, &None);
    let review = client.get_review(&project_id, &reviewer).unwrap();
    assert_eq!(review.rating, 5);
}

#[test]
fn test_first_update_always_succeeds_no_previous_timestamp() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "FirstUpdateProject");
    let reviewer = Address::generate(&env);
    client.add_review(&project_id, &reviewer, &2, &None);

    // No prior update timestamp — first update must always succeed.
    client.update_review(&project_id, &reviewer, &3, &None);
    let review = client.get_review(&project_id, &reviewer).unwrap();
    assert_eq!(review.rating, 3);
}

// ─── Tombstones (#240) ───────────────────────────────────────────────────────

#[test]
fn test_delete_review_leaves_tombstone() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "TombstoneProject");
    let reviewer = Address::generate(&env);
    client.add_review(&project_id, &reviewer, &4, &None);

    // Review exists; no tombstone yet.
    assert!(client.get_review(&project_id, &reviewer).is_some());
    assert!(client.get_review_tombstone(&project_id, &reviewer).is_none());

    client.delete_review(&project_id, &reviewer);

    // Review gone; tombstone present.
    assert!(client.get_review(&project_id, &reviewer).is_none());
    let tombstone = client.get_review_tombstone(&project_id, &reviewer).unwrap();
    assert_eq!(tombstone.project_id, project_id);
    assert_eq!(tombstone.reviewer, reviewer);
}

#[test]
fn test_admin_delete_review_leaves_tombstone() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "AdminTombstoneProject");
    let reviewer = Address::generate(&env);
    client.add_review(&project_id, &reviewer, &3, &None);

    client.admin_delete_review(&project_id, &reviewer, &admin);

    assert!(client.get_review(&project_id, &reviewer).is_none());
    assert!(client.get_review_tombstone(&project_id, &reviewer).is_some());
}

#[test]
fn test_tombstone_not_present_for_never_reviewed() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "NoReviewProject");
    let stranger = Address::generate(&env);

    // Neither a review nor a tombstone should exist.
    assert!(client.get_review(&project_id, &stranger).is_none());
    assert!(client.get_review_tombstone(&project_id, &stranger).is_none());
}

// ─── Sorting (#241) ──────────────────────────────────────────────────────────

#[test]
fn test_list_reviews_sorted_newest_first() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "SortNewestProject");

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    client.add_review(&project_id, &r1, &3, &None);
    env.ledger().with_mut(|l| l.timestamp += 100);
    client.add_review(&project_id, &r2, &5, &None);

    let reviews = client.list_reviews_sorted(&project_id, &0, &10, &ReviewSortMode::Newest);
    assert_eq!(reviews.len(), 2);
    // r2 was added later, so created_at is higher — should come first.
    assert_eq!(reviews.get(0).unwrap().reviewer, r2);
    assert_eq!(reviews.get(1).unwrap().reviewer, r1);
}

#[test]
fn test_list_reviews_sorted_rating_high_to_low() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "SortRatingProject");

    let r1 = Address::generate(&env);
    let r2 = Address::generate(&env);
    let r3 = Address::generate(&env);
    client.add_review(&project_id, &r1, &2, &None);
    client.add_review(&project_id, &r2, &5, &None);
    client.add_review(&project_id, &r3, &3, &None);

    let reviews = client.list_reviews_sorted(&project_id, &0, &10, &ReviewSortMode::RatingHigh);
    assert_eq!(reviews.len(), 3);
    assert_eq!(reviews.get(0).unwrap().rating, 5);
    assert_eq!(reviews.get(1).unwrap().rating, 3);
    assert_eq!(reviews.get(2).unwrap().rating, 2);
}
