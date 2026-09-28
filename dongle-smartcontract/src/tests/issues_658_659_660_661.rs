//! Tests for issues #658, #659, #660, #661.
//! Tests for issues #658, #659, #660, #661.
//!
//! - #661: Featured projects max limit enforcement and FIFO eviction policy.
//! - #660: Maintainer permission model — allowed and denied operations.
//! - #659: Admin action log coverage for all loggable operations.
//! - #658: Hidden review filtering across all listing / lookup functions.

#![cfg(test)]

use crate::constants::MAX_FEATURED_PROJECTS;
use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::{AdminActionType, ProjectUpdateParams};
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};

// ── Issue #661: Featured projects limit & eviction ────────────────────────────

/// Verify that the constant is set to a reasonable value.
#[test]
fn test_max_featured_projects_constant() {
    assert!(MAX_FEATURED_PROJECTS > 0, "MAX_FEATURED_PROJECTS must be positive");
    assert!(
        MAX_FEATURED_PROJECTS <= 100,
        "MAX_FEATURED_PROJECTS should be ≤ 100 to remain manageable"
    );
}

/// When the featured list is below the limit, all set_featured(true) calls succeed
/// and the list grows up to exactly MAX_FEATURED_PROJECTS entries.
#[test]
fn test_featured_list_grows_up_to_limit() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let limit = MAX_FEATURED_PROJECTS;

    // Register exactly MAX_FEATURED_PROJECTS projects and feature them all.
    for i in 0..limit {
        let name_str = alloc_string(i);
        let id = create_test_project(&client, &owner, &name_str);
        client.mock_all_auths().set_featured(&admin, &id, &true);
    }

    let count = client.get_featured_count();
    assert_eq!(count, limit, "Featured count should equal the limit");

    let featured = client.list_featured_projects(&0, &(limit + 10));
    assert_eq!(featured.len(), limit, "List should contain exactly MAX_FEATURED_PROJECTS projects");
}

/// When the limit is already reached, featuring a new project evicts the
/// oldest one (FIFO) and keeps the count at MAX_FEATURED_PROJECTS.
#[test]
fn test_featured_eviction_when_limit_reached() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let limit = MAX_FEATURED_PROJECTS;

    // Feature exactly MAX_FEATURED_PROJECTS projects.
    let mut ids = soroban_sdk::Vec::new(&env);
    for i in 0..limit {
        let name_str = alloc_string(i);
        let id = create_test_project(&client, &owner, &name_str);
        client.mock_all_auths().set_featured(&admin, &id, &true);
        ids.push_back(id);
    }

    let first_id = ids.get(0).unwrap();

    // Feature one more project — this should evict the oldest (first_id).
    let extra_name = alloc_string(limit);
    let extra_id = create_test_project(&client, &owner, &extra_name);
    client.mock_all_auths().set_featured(&admin, &extra_id, &true);

    // Count should still be MAX_FEATURED_PROJECTS.
    let count = client.get_featured_count();
    assert_eq!(count, limit, "Count must stay at the limit after eviction");

    // The newly featured project must appear in the list.
    let featured = client.list_featured_projects(&0, &(limit + 10));
    let has_extra = featured.iter().any(|p| p.id == extra_id);
    assert!(has_extra, "Newly featured project must be in the list");

    // The oldest project must no longer appear in the list.
    let has_first = featured.iter().any(|p| p.id == first_id);
    assert!(!has_first, "Oldest featured project must have been evicted");
}

/// Featuring an already-featured project is a no-op (no duplicate, no eviction).
#[test]
fn test_featured_idempotent_does_not_evict() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let limit = MAX_FEATURED_PROJECTS;

    // Feature exactly MAX_FEATURED_PROJECTS projects.
    let mut ids = soroban_sdk::Vec::new(&env);
    for i in 0..limit {
        let name_str = alloc_string(i);
        let id = create_test_project(&client, &owner, &name_str);
        client.mock_all_auths().set_featured(&admin, &id, &true);
        ids.push_back(id);
    }

    let first_id = ids.get(0).unwrap();

    // Feature the LAST project again (already featured) — must be a no-op.
    let last_id = ids.get(limit - 1).unwrap();
    client.mock_all_auths().set_featured(&admin, &last_id, &true);

    // Count must not have grown.
    let count = client.get_featured_count();
    assert_eq!(count, limit, "Count must not grow on idempotent feature");

    // First project must still be in the list (not evicted by the no-op).
    let featured = client.list_featured_projects(&0, &(limit + 10));
    let has_first = featured.iter().any(|p| p.id == first_id);
    assert!(has_first, "Idempotent feature must not evict existing projects");
}

/// list_featured_projects respects insertion order (oldest first).
#[test]
fn test_featured_insertion_order() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let id1 = create_test_project(&client, &owner, "First");
    let id2 = create_test_project(&client, &owner, "Second");
    let id3 = create_test_project(&client, &owner, "Third");

    client.mock_all_auths().set_featured(&admin, &id1, &true);
    client.mock_all_auths().set_featured(&admin, &id2, &true);
    client.mock_all_auths().set_featured(&admin, &id3, &true);

    let featured = client.list_featured_projects(&0, &10);
    assert_eq!(featured.len(), 3);
    assert_eq!(featured.get(0).unwrap().id, id1);
    assert_eq!(featured.get(1).unwrap().id, id2);
    assert_eq!(featured.get(2).unwrap().id, id3);
}

// ── Issue #660: Maintainer permission model ───────────────────────────────────

/// Maintainers CAN update project metadata.
#[test]
fn test_maintainer_can_update_project() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "MyProject");
    let maintainer = Address::generate(&env);

    client
        .mock_all_auths()
        .add_maintainer(&project_id, &owner, &maintainer);

    let new_desc = String::from_str(&env, "Updated by maintainer");
    let params = ProjectUpdateParams {
        project_id,
        caller: maintainer.clone(),
        name: None,
        slug: None,
        description: Some(new_desc.clone()),
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
    };

    let updated = client.mock_all_auths().update_project(&params);
    assert_eq!(updated.description, new_desc);
}

/// Maintainers CAN update the security contact.
#[test]
fn test_maintainer_can_update_security_contact() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "MyProject");
    let maintainer = Address::generate(&env);

    client
        .mock_all_auths()
        .add_maintainer(&project_id, &owner, &maintainer);

    let contact = String::from_str(&env, "security@example.com");
    let result = client
        .mock_all_auths()
        .update_security_contact(&project_id, &maintainer, &Some(contact));
    assert!(result.is_ok());
}

/// Maintainers CANNOT add other maintainers (owner-only).
#[test]
fn test_maintainer_cannot_add_maintainer() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "MyProject");
    let maintainer = Address::generate(&env);
    let other = Address::generate(&env);

    client
        .mock_all_auths()
        .add_maintainer(&project_id, &owner, &maintainer);

    let result = client
        .mock_all_auths()
        .try_add_maintainer(&project_id, &maintainer, &other);
    assert_eq!(result, Err(Ok(ContractError::Unauthorized)));
}

/// Maintainers CANNOT remove maintainers (owner-only).
#[test]
fn test_maintainer_cannot_remove_maintainer() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "MyProject");
    let maintainer = Address::generate(&env);

    client
        .mock_all_auths()
        .add_maintainer(&project_id, &owner, &maintainer);

    let result = client
        .mock_all_auths()
        .try_remove_maintainer(&project_id, &maintainer, &maintainer);
    assert_eq!(result, Err(Ok(ContractError::Unauthorized)));
}

/// Maintainers CANNOT initiate ownership transfer (owner-only).
#[test]
fn test_maintainer_cannot_transfer_ownership() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "MyProject");
    let maintainer = Address::generate(&env);
    let new_owner = Address::generate(&env);

    client
        .mock_all_auths()
        .add_maintainer(&project_id, &owner, &maintainer);

    let result = client
        .mock_all_auths()
        .try_initiate_transfer(&project_id, &maintainer, &new_owner);
    assert_eq!(result, Err(Ok(ContractError::Unauthorized)));
}

/// Maintainers CANNOT archive the project (owner/admin only).
#[test]
fn test_maintainer_cannot_archive_project() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "MyProject");
    let maintainer = Address::generate(&env);

    client
        .mock_all_auths()
        .add_maintainer(&project_id, &owner, &maintainer);

    let result = client
        .mock_all_auths()
        .try_archive_project(&project_id, &maintainer);
    assert_eq!(result, Err(Ok(ContractError::Unauthorized)));
}

/// Maintainers CANNOT set the claimable flag (owner/admin only).
#[test]
fn test_maintainer_cannot_set_claimable() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "MyProject");
    let maintainer = Address::generate(&env);

    client
        .mock_all_auths()
        .add_maintainer(&project_id, &owner, &maintainer);

    let result = client
        .mock_all_auths()
        .try_set_project_claimable(&project_id, &maintainer, &true);
    assert_eq!(result, Err(Ok(ContractError::Unauthorized)));
}

/// Owner retains all privileges after adding maintainers.
#[test]
fn test_owner_retains_all_privileges_with_maintainer() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "MyProject");
    let maintainer = Address::generate(&env);

    client
        .mock_all_auths()
        .add_maintainer(&project_id, &owner, &maintainer);

    // Owner can still archive, add/remove maintainers, transfer, etc.
    let other = Address::generate(&env);
    client
        .mock_all_auths()
        .add_maintainer(&project_id, &owner, &other);
    client
        .mock_all_auths()
        .remove_maintainer(&project_id, &owner, &other);

    let result = client
        .mock_all_auths()
        .try_archive_project(&project_id, &owner);
    assert!(result.is_ok());
}

/// Non-owner/non-admin/non-maintainer cannot update project.
#[test]
fn test_stranger_cannot_update_project() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "MyProject");
    let stranger = Address::generate(&env);

    let params = ProjectUpdateParams {
        project_id,
        caller: stranger.clone(),
        name: None,
        slug: None,
        description: Some(String::from_str(&env, "hacked")),
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
    };

    let result = client.mock_all_auths().try_update_project(&params);
    assert_eq!(result, Err(Ok(ContractError::Unauthorized)));
}

// ── Issue #659: Admin action log coverage ─────────────────────────────────────

/// Approving a claim request is logged with ClaimRequestApproved.
#[test]
fn test_log_claim_request_approved() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| li.timestamp = 1_000_000);
    let (client, admin) = setup_contract(&env);

    let original_owner = Address::generate(&env);
    let claimant = Address::generate(&env);
    let project_id = create_test_project(&client, &original_owner, "ClaimableProj");

    // Make project claimable.
    client.set_project_claimable(&project_id, &original_owner, &true);

    let proof = String::from_str(
        &env,
        "QmClaimProofCid12345678901234567890123456789012345",
    );
    let claim_id = client.submit_claim_request(&project_id, &claimant, &proof);

    let log_count_before = client.get_admin_action_log_count();
    client.approve_claim_request(&claim_id, &admin);
    let log_count_after = client.get_admin_action_log_count();

    assert_eq!(
        log_count_after,
        log_count_before + 1,
        "approve_claim_request must produce one new log entry"
    );

    let entry = client.get_admin_action_log_entry(&log_count_after).unwrap();
    assert_eq!(entry.action_type, AdminActionType::ClaimRequestApproved);
    assert_eq!(entry.target_id, Some(project_id));
    assert_eq!(entry.admin, admin);
}

/// Rejecting a claim request is logged with ClaimRequestRejected.
#[test]
fn test_log_claim_request_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| li.timestamp = 1_000_000);
    let (client, admin) = setup_contract(&env);

    let original_owner = Address::generate(&env);
    let claimant = Address::generate(&env);
    let project_id = create_test_project(&client, &original_owner, "ClaimableProj2");

    client.set_project_claimable(&project_id, &original_owner, &true);

    let proof = String::from_str(
        &env,
        "QmClaimProofCid12345678901234567890123456789012345",
    );
    let claim_id = client.submit_claim_request(&project_id, &claimant, &proof);

    let log_count_before = client.get_admin_action_log_count();
    client.reject_claim_request(&claim_id, &admin);
    let log_count_after = client.get_admin_action_log_count();

    assert_eq!(
        log_count_after,
        log_count_before + 1,
        "reject_claim_request must produce one new log entry"
    );

    let entry = client.get_admin_action_log_entry(&log_count_after).unwrap();
    assert_eq!(entry.action_type, AdminActionType::ClaimRequestRejected);
    assert_eq!(entry.target_id, Some(project_id));
    assert_eq!(entry.admin, admin);
}

/// Featuring a project is logged with ProjectFeatured.
#[test]
fn test_log_project_featured() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "FeaturedProj");

    let before = client.get_admin_action_log_count();
    client.mock_all_auths().set_featured(&admin, &project_id, &true);
    let after = client.get_admin_action_log_count();

    assert_eq!(after, before + 1);
    let entry = client.get_admin_action_log_entry(&after).unwrap();
    assert_eq!(entry.action_type, AdminActionType::ProjectFeatured);
    assert_eq!(entry.target_id, Some(project_id));
}

/// Unfeaturing a project is logged with ProjectUnfeatured.
#[test]
fn test_log_project_unfeatured() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "FeaturedProj2");

    client.mock_all_auths().set_featured(&admin, &project_id, &true);

    let before = client.get_admin_action_log_count();
    client.mock_all_auths().set_featured(&admin, &project_id, &false);
    let after = client.get_admin_action_log_count();

    assert_eq!(after, before + 1);
    let entry = client.get_admin_action_log_entry(&after).unwrap();
    assert_eq!(entry.action_type, AdminActionType::ProjectUnfeatured);
    assert_eq!(entry.target_id, Some(project_id));
}

/// Hiding a review is logged with ReviewHidden.
#[test]
fn test_log_review_hidden() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "ReviewedProj");

    client.add_review(&project_id, &reviewer, &4, &None);

    let before = client.get_admin_action_log_count();
    client.hide_review(&project_id, &reviewer, &admin);
    let after = client.get_admin_action_log_count();

    assert_eq!(after, before + 1);
    let entry = client.get_admin_action_log_entry(&after).unwrap();
    assert_eq!(entry.action_type, AdminActionType::ReviewHidden);
    assert_eq!(entry.target_id, Some(project_id));
    assert_eq!(entry.target_address, Some(reviewer));
}

/// Restoring a review is logged with ReviewRestored.
#[test]
fn test_log_review_restored() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "ReviewedProj2");

    client.add_review(&project_id, &reviewer, &3, &None);
    client.hide_review(&project_id, &reviewer, &admin);

    let before = client.get_admin_action_log_count();
    client.restore_review(&project_id, &reviewer, &admin);
    let after = client.get_admin_action_log_count();

    assert_eq!(after, before + 1);
    let entry = client.get_admin_action_log_entry(&after).unwrap();
    assert_eq!(entry.action_type, AdminActionType::ReviewRestored);
}

/// Adding a reserved name is logged with ReservedNameAdded.
#[test]
fn test_log_reserved_name_added() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let before = client.get_admin_action_log_count();
    client
        .mock_all_auths()
        .add_reserved_name(&admin, &String::from_str(&env, "admin"));
    let after = client.get_admin_action_log_count();

    assert_eq!(after, before + 1);
    let entry = client.get_admin_action_log_entry(&after).unwrap();
    assert_eq!(entry.action_type, AdminActionType::ReservedNameAdded);
}

/// Removing a reserved name is logged with ReservedNameRemoved.
#[test]
fn test_log_reserved_name_removed() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    client
        .mock_all_auths()
        .add_reserved_name(&admin, &String::from_str(&env, "admin"));

    let before = client.get_admin_action_log_count();
    client
        .mock_all_auths()
        .remove_reserved_name(&admin, &String::from_str(&env, "admin"));
    let after = client.get_admin_action_log_count();

    assert_eq!(after, before + 1);
    let entry = client.get_admin_action_log_entry(&after).unwrap();
    assert_eq!(entry.action_type, AdminActionType::ReservedNameRemoved);
}

// ── Issue #658: Hidden review visibility filtering ────────────────────────────

/// list_reviews does NOT return hidden reviews.
#[test]
fn test_list_reviews_excludes_hidden() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let reviewer1 = Address::generate(&env);
    let reviewer2 = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "HiddenTest");

    client.add_review(&project_id, &reviewer1, &5, &None);
    client.add_review(&project_id, &reviewer2, &3, &None);

    // Hide reviewer1's review.
    client.hide_review(&project_id, &reviewer1, &admin);

    let reviews = client.list_reviews(&project_id, &0, &10);
    assert_eq!(reviews.len(), 1, "Hidden review must not appear in list_reviews");
    assert_eq!(reviews.get(0).unwrap().reviewer, reviewer2);
}

/// get_reviews_by_ids does NOT return hidden reviews.
#[test]
fn test_get_reviews_by_ids_excludes_hidden() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "HiddenByIds");

    client.add_review(&project_id, &reviewer, &4, &None);
    client.hide_review(&project_id, &reviewer, &admin);

    let mut ids: Vec<(u64, Address)> = Vec::new(&env);
    ids.push_back((project_id, reviewer.clone()));
    let result = client.get_reviews_by_ids(&ids);
    assert_eq!(
        result.len(),
        0,
        "get_reviews_by_ids must exclude hidden reviews"
    );
}

/// get_review_cid returns None for hidden reviews.
#[test]
fn test_get_review_cid_returns_none_for_hidden() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "HiddenCidTest");

    let cid = String::from_str(&env, "QmCidForHiddenReview123456789012345678901234567890ab");
    client.submit_review(&project_id, &reviewer, &4, &cid);

    // CID is available before hiding.
    assert!(
        client.get_review_cid(&project_id, &reviewer).is_some(),
        "CID must be returned before hiding"
    );

    client.hide_review(&project_id, &reviewer, &admin);

    assert!(
        client.get_review_cid(&project_id, &reviewer).is_none(),
        "CID must not be returned for hidden review"
    );
}

/// get_project_review_cids omits CIDs for hidden reviews.
#[test]
fn test_get_project_review_cids_excludes_hidden() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let reviewer1 = Address::generate(&env);
    let reviewer2 = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "HiddenCidsProj");

    let cid1 = String::from_str(&env, "QmCidForReviewer1123456789012345678901234567890abcd");
    let cid2 = String::from_str(&env, "QmCidForReviewer2123456789012345678901234567890abcd");
    client.submit_review(&project_id, &reviewer1, &5, &cid1);
    client.submit_review(&project_id, &reviewer2, &3, &cid2);

    // Hide reviewer1's review.
    client.hide_review(&project_id, &reviewer1, &admin);

    let cids = client.get_project_review_cids(&project_id);
    assert_eq!(cids.len(), 1, "Hidden review CID must not appear");
    assert_eq!(cids.get(0).unwrap().0, reviewer2);
}

/// Direct get_review still returns the full review record (for admin/owner inspection).
/// The contract does not gate it — callers are responsible for authorization.
#[test]
fn test_get_review_direct_returns_hidden_review() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "DirectGetTest");

    client.add_review(&project_id, &reviewer, &2, &None);
    client.hide_review(&project_id, &reviewer, &admin);

    // get_review returns Some even for hidden reviews (admin use-case).
    let review = client.get_review(&project_id, &reviewer);
    assert!(review.is_some(), "get_review must still return hidden review for direct access");
    assert!(
        review.unwrap().hidden,
        "Returned review must have hidden=true"
    );
}

/// Restoring a hidden review makes it visible again in all listing functions.
#[test]
fn test_restored_review_visible_in_all_listings() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "RestoreTest");

    let cid = String::from_str(&env, "QmRestoredReviewCid1234567890123456789012345678901");
    client.submit_review(&project_id, &reviewer, &5, &cid);
    client.hide_review(&project_id, &reviewer, &admin);
    client.restore_review(&project_id, &reviewer, &admin);

    // list_reviews should include it again.
    let reviews = client.list_reviews(&project_id, &0, &10);
    assert_eq!(reviews.len(), 1, "Restored review must appear in list_reviews");

    // get_review_cid should return it again.
    assert!(
        client.get_review_cid(&project_id, &reviewer).is_some(),
        "Restored review CID must be returned"
    );

    // get_project_review_cids should include it.
    let cids = client.get_project_review_cids(&project_id);
    assert_eq!(cids.len(), 1, "Restored review CID must appear in project CID list");

    // get_reviews_by_ids should include it.
    let mut ids: Vec<(u64, Address)> = Vec::new(&env);
    ids.push_back((project_id, reviewer.clone()));
    let by_ids = client.get_reviews_by_ids(&ids);
    assert_eq!(by_ids.len(), 1, "Restored review must appear in get_reviews_by_ids");
}

// ── Helper ─────────────────────────────────────────────────────────────────────

extern crate alloc;
use alloc::string::ToString;

fn alloc_string(i: u32) -> alloc::string::String {
    // Generate a unique short name: "Prj00", "Prj01", … suitable for project names.
    alloc::format!("Prj{:03}", i)
}
