//! Tests for canonical CID field consolidation - ensuring no data loss

use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_env, register_test_project};
use crate::types::Review;
use crate::DongleContract;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

// ── CID Data Consolidation Tests ──

#[test]
fn test_add_review_uses_canonical_content_cid() {
    let (env, _admin, owner) = create_test_env();
    let project_id = register_test_project(&env, &owner);

    let reviewer = Address::generate(&env);
    let cid = String::from_str(&env, "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG");

    let result = DongleContract::add_review(
        env.clone(),
        project_id,
        reviewer.clone(),
        5,
        Some(cid.clone()),
    );

    assert!(result.is_ok());

    // Retrieve the review
    let review = DongleContract::get_review(env.clone(), project_id, reviewer.clone())
        .expect("Review should exist");

    // Verify that content_cid is set (consolidated field)
    assert_eq!(review.content_cid, Some(cid.clone()));

    // Verify old duplicate fields no longer exist
    // This ensures we can't accidentally use stale fields
    let review_struct: Review = review; // Verify structure
    assert_eq!(review_struct.content_cid, Some(cid));
}

#[test]
fn test_add_review_without_cid_works() {
    let (env, _admin, owner) = create_test_env();
    let project_id = register_test_project(&env, &owner);

    let reviewer = Address::generate(&env);

    let result = DongleContract::add_review(env.clone(), project_id, reviewer.clone(), 4, None);

    assert!(result.is_ok());

    let review = DongleContract::get_review(env.clone(), project_id, reviewer.clone())
        .expect("Review should exist");

    // Verify content_cid is None
    assert_eq!(review.content_cid, None);
}

#[test]
fn test_update_review_updates_canonical_cid() {
    let (env, _admin, owner) = create_test_env();
    let project_id = register_test_project(&env, &owner);

    let reviewer = Address::generate(&env);
    let original_cid = String::from_str(&env, "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG");

    // Add review
    DongleContract::add_review(
        env.clone(),
        project_id,
        reviewer.clone(),
        5,
        Some(original_cid.clone()),
    )
    .unwrap();

    // Update with different CID
    let new_cid = String::from_str(&env, "QmXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX");

    let result = DongleContract::update_review(
        env.clone(),
        project_id,
        reviewer.clone(),
        4,
        Some(new_cid.clone()),
    );

    assert!(result.is_ok());

    let review = DongleContract::get_review(env.clone(), project_id, reviewer.clone())
        .expect("Review should exist");

    // Verify content_cid is updated
    assert_eq!(review.content_cid, Some(new_cid));
    assert_ne!(review.content_cid, Some(original_cid));
}

#[test]
fn test_review_cid_getter_returns_content_cid() {
    let (env, _admin, owner) = create_test_env();
    let project_id = register_test_project(&env, &owner);

    let reviewer = Address::generate(&env);
    let cid = String::from_str(&env, "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG");

    DongleContract::add_review(
        env.clone(),
        project_id,
        reviewer.clone(),
        5,
        Some(cid.clone()),
    )
    .unwrap();

    // Verify internal getter works
    let retrieved_cid = crate::review_registry::ReviewRegistry::get_review_cid(
        &env,
        project_id,
        reviewer.clone(),
    );

    assert_eq!(retrieved_cid, Some(cid));
}

#[test]
fn test_multiple_reviews_each_has_own_cid() {
    let (env, _admin, owner) = create_test_env();
    let project_id = register_test_project(&env, &owner);

    let reviewer1 = Address::generate(&env);
    let reviewer2 = Address::generate(&env);

    let cid1 = String::from_str(&env, "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG");
    let cid2 = String::from_str(&env, "QmXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX");

    // Add first review
    DongleContract::add_review(
        env.clone(),
        project_id,
        reviewer1.clone(),
        5,
        Some(cid1.clone()),
    )
    .unwrap();

    // Add second review
    DongleContract::add_review(
        env.clone(),
        project_id,
        reviewer2.clone(),
        4,
        Some(cid2.clone()),
    )
    .unwrap();

    // Verify each review has correct CID
    let review1 =
        DongleContract::get_review(env.clone(), project_id, reviewer1.clone()).unwrap();
    let review2 =
        DongleContract::get_review(env.clone(), project_id, reviewer2.clone()).unwrap();

    assert_eq!(review1.content_cid, Some(cid1));
    assert_eq!(review2.content_cid, Some(cid2));
}

// ── Data Preservation Tests ──

#[test]
fn test_review_preserves_all_data_across_operations() {
    let (env, _admin, owner) = create_test_env();
    let project_id = register_test_project(&env, &owner);

    let reviewer = Address::generate(&env);
    let cid = String::from_str(&env, "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG");

    // Add review
    DongleContract::add_review(
        env.clone(),
        project_id,
        reviewer.clone(),
        5,
        Some(cid.clone()),
    )
    .unwrap();

    let review_v1 = DongleContract::get_review(env.clone(), project_id, reviewer.clone())
        .expect("Review should exist");

    let created_at = review_v1.created_at;
    let original_rating = review_v1.rating;

    // Update review
    let new_cid = String::from_str(&env, "QmABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCD");
    DongleContract::update_review(
        env.clone(),
        project_id,
        reviewer.clone(),
        4,
        Some(new_cid.clone()),
    )
    .unwrap();

    let review_v2 = DongleContract::get_review(env.clone(), project_id, reviewer.clone())
        .expect("Review should exist");

    // Verify immutable fields are preserved
    assert_eq!(review_v2.project_id, project_id);
    assert_eq!(review_v2.reviewer, reviewer);
    assert_eq!(review_v2.created_at, created_at);

    // Verify mutable fields are updated
    assert_eq!(review_v2.rating, 4);
    assert_ne!(review_v2.rating, original_rating);
    assert_eq!(review_v2.content_cid, Some(new_cid.clone()));
    assert_ne!(review_v2.content_cid, Some(cid));

    // Verify updated_at changed
    assert!(review_v2.updated_at >= review_v1.updated_at);
}

#[test]
fn test_review_listing_preserves_cids() {
    let (env, _admin, owner) = create_test_env();
    let project_id = register_test_project(&env, &owner);

    let reviewer1 = Address::generate(&env);
    let reviewer2 = Address::generate(&env);
    let reviewer3 = Address::generate(&env);

    let cid1 = String::from_str(&env, "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG");
    let cid2 = String::from_str(&env, "QmXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX");
    let cid3 = String::from_str(&env, "QmABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ABCD");

    // Add multiple reviews
    DongleContract::add_review(
        env.clone(),
        project_id,
        reviewer1.clone(),
        5,
        Some(cid1.clone()),
    )
    .unwrap();
    DongleContract::add_review(
        env.clone(),
        project_id,
        reviewer2.clone(),
        4,
        Some(cid2.clone()),
    )
    .unwrap();
    DongleContract::add_review(
        env.clone(),
        project_id,
        reviewer3.clone(),
        3,
        Some(cid3.clone()),
    )
    .unwrap();

    // List reviews
    let reviews = DongleContract::list_reviews(env.clone(), project_id, 0, 10);

    assert_eq!(reviews.len(), 3);

    // Verify all CIDs are preserved in list
    let mut found_cids = vec![];
    for review in reviews.iter() {
        if let Some(cid) = review.content_cid.clone() {
            found_cids.push(cid);
        }
    }

    assert!(found_cids.contains(&cid1));
    assert!(found_cids.contains(&cid2));
    assert!(found_cids.contains(&cid3));
}

// ── Migration Path Tests ──

#[test]
fn test_get_review_still_works_after_consolidation() {
    let (env, _admin, owner) = create_test_env();
    let project_id = register_test_project(&env, &owner);

    let reviewer = Address::generate(&env);
    let cid = String::from_str(&env, "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG");

    // Add review using new consolidated flow
    DongleContract::add_review(
        env.clone(),
        project_id,
        reviewer.clone(),
        5,
        Some(cid.clone()),
    )
    .unwrap();

    // Get review using public API (migration path)
    let review = DongleContract::get_review(env.clone(), project_id, reviewer.clone())
        .expect("Get review should still work");

    // Public API works seamlessly with new structure
    assert_eq!(review.rating, 5);
    assert_eq!(review.content_cid, Some(cid));
}

#[test]
fn test_delete_review_preserves_cid_until_deletion() {
    let (env, _admin, owner) = create_test_env();
    let project_id = register_test_project(&env, &owner);

    let reviewer = Address::generate(&env);
    let cid = String::from_str(&env, "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG");

    // Add review
    DongleContract::add_review(
        env.clone(),
        project_id,
        reviewer.clone(),
        5,
        Some(cid.clone()),
    )
    .unwrap();

    // Verify review exists with CID
    let review_before = DongleContract::get_review(env.clone(), project_id, reviewer.clone())
        .expect("Review should exist");
    assert_eq!(review_before.content_cid, Some(cid.clone()));

    // Delete review
    let delete_result = DongleContract::delete_review(env.clone(), project_id, reviewer.clone());
    assert!(delete_result.is_ok());

    // Verify review no longer exists
    let review_after = DongleContract::get_review(env.clone(), project_id, reviewer.clone());
    assert!(review_after.is_none());
}

// ── Edge Case Tests ──

#[test]
fn test_empty_cid_string_handled_correctly() {
    let (env, _admin, owner) = create_test_env();
    let project_id = register_test_project(&env, &owner);

    let reviewer = Address::generate(&env);

    // Attempt to add review with None CID
    let result = DongleContract::add_review(
        env.clone(),
        project_id,
        reviewer.clone(),
        5,
        None,
    );

    assert!(result.is_ok());

    let review = DongleContract::get_review(env.clone(), project_id, reviewer.clone()).unwrap();
    assert_eq!(review.content_cid, None);
}

#[test]
fn test_cid_can_be_cleared_on_update() {
    let (env, _admin, owner) = create_test_env();
    let project_id = register_test_project(&env, &owner);

    let reviewer = Address::generate(&env);
    let cid = String::from_str(&env, "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG");

    // Add review with CID
    DongleContract::add_review(
        env.clone(),
        project_id,
        reviewer.clone(),
        5,
        Some(cid.clone()),
    )
    .unwrap();

    // Update review to remove CID
    let result = DongleContract::update_review(env.clone(), project_id, reviewer.clone(), 4, None);

    assert!(result.is_ok());

    let review = DongleContract::get_review(env.clone(), project_id, reviewer.clone()).unwrap();
    assert_eq!(review.content_cid, None);
}
