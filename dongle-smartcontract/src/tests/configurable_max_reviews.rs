//! Tests for issue #527: configurable maximum reviews per project.
//!
//! Covers all acceptance criteria:
//! - Default behavior matches existing 500-review limit (storage fallback).
//! - Configured maximum is stored and read back correctly.
//! - Authorized admin can update the maximum.
//! - Unauthorized user cannot update the maximum.
//! - Review creation respects the configured maximum.
//! - A project can reach exactly the configured maximum.
//! - Adding a review beyond the configured maximum is rejected.
//! - Lowering the maximum affects subsequent review creation.
//! - Increasing the maximum allows additional reviews.
//! - Invalid maximum values are rejected.
//! - Multiple projects independently respect the same configured limit.
//! - Existing review/project tests still pass.
//! - Existing admin/authorization tests still pass.

use crate::constants::MAX_REVIEWS_PER_PROJECT;
use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use soroban_sdk::{testutils::Address as _, Address, Env};

// ── Default behavior ─────────────────────────────────────────────────────────

/// Without any explicit configuration the getter must return 500, matching
/// the existing compile-time constant and preserving backwards compatibility
/// for deployments that have never called `set_max_reviews_per_project`.
#[test]
fn test_default_max_reviews_matches_constant() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    assert_eq!(
        client.get_max_reviews_per_project(),
        MAX_REVIEWS_PER_PROJECT,
        "default must equal the compile-time constant (500)"
    );
}

/// Verify that get_config also surfaces the default 500 before any explicit
/// configuration.
#[test]
fn test_get_config_shows_default_limit() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let config = client.get_config();
    assert_eq!(
        config.limits.max_reviews_per_project,
        MAX_REVIEWS_PER_PROJECT
    );
}

// ── Storage correctness ───────────────────────────────────────────────────────

/// The configured value must survive a round-trip through persistent storage.
#[test]
fn test_configured_maximum_stored_correctly() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    client.set_max_reviews_per_project(&admin, &100);

    assert_eq!(client.get_max_reviews_per_project(), 100);
}

/// get_config must reflect the newly configured value.
#[test]
fn test_get_config_reflects_configured_limit() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    client.set_max_reviews_per_project(&admin, &42);

    let config = client.get_config();
    assert_eq!(config.limits.max_reviews_per_project, 42);
}

// ── Authorization ─────────────────────────────────────────────────────────────

/// A registered admin must be able to set the maximum without error.
#[test]
fn test_admin_can_set_max_reviews() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    // Should succeed without panicking.
    client.set_max_reviews_per_project(&admin, &250);
    assert_eq!(client.get_max_reviews_per_project(), 250);
}

/// A non-admin address must be rejected with `AdminOnly`.
#[test]
fn test_non_admin_cannot_set_max_reviews() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let attacker = Address::generate(&env);
    let result = client
        .mock_all_auths()
        .try_set_max_reviews_per_project(&attacker, &100);

    assert_eq!(result, Err(Ok(ContractError::AdminOnly.into())));
}

// ── Boundary enforcement ──────────────────────────────────────────────────────

/// A project can accumulate exactly `max` reviews without error.
#[test]
fn test_project_can_reach_exactly_configured_max() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    let max: u32 = 5;
    client.set_max_reviews_per_project(&admin, &max);

    let project_id = create_test_project(&client, &admin, "Cap-Project");

    for _ in 0..max {
        let reviewer = Address::generate(&env);
        client.add_review(&project_id, &reviewer, &4, &None);
    }

    // Exactly `max` reviews should now be present.
    let stats = client.get_project_stats(&project_id);
    assert_eq!(stats.review_count, max);
}

/// The (max + 1)-th review must be rejected with `MaxProjectsExceeded`.
#[test]
fn test_review_beyond_configured_max_is_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    let max: u32 = 3;
    client.set_max_reviews_per_project(&admin, &max);

    let project_id = create_test_project(&client, &admin, "Overflow-Project");

    for _ in 0..max {
        let reviewer = Address::generate(&env);
        client.add_review(&project_id, &reviewer, &5, &None);
    }

    let overflow_reviewer = Address::generate(&env);
    let result = client
        .mock_all_auths()
        .try_add_review(&project_id, &overflow_reviewer, &3, &None);

    assert_eq!(result, Err(Ok(ContractError::MaxProjectsExceeded.into())));
}

/// After lowering the maximum, reviews that would now exceed the new limit
/// must be rejected, even though previously they would have been allowed.
#[test]
fn test_lowering_max_prevents_future_reviews() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    // Start with a limit of 5.
    client.set_max_reviews_per_project(&admin, &5);
    let project_id = create_test_project(&client, &admin, "Lowered-Project");

    // Add 3 reviews (well within the initial limit).
    for _ in 0..3 {
        let reviewer = Address::generate(&env);
        client.add_review(&project_id, &reviewer, &4, &None);
    }

    // Lower the limit to 3 (i.e., currently at the cap).
    client.set_max_reviews_per_project(&admin, &3);

    // The next review must be rejected.
    let overflow_reviewer = Address::generate(&env);
    let result = client
        .mock_all_auths()
        .try_add_review(&project_id, &overflow_reviewer, &2, &None);

    assert_eq!(result, Err(Ok(ContractError::MaxProjectsExceeded.into())));
}

/// After raising the maximum, additional reviews are accepted up to the new limit.
#[test]
fn test_raising_max_allows_additional_reviews() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    // Start with a low limit of 2.
    client.set_max_reviews_per_project(&admin, &2);
    let project_id = create_test_project(&client, &admin, "Raised-Project");

    // Fill to the initial limit.
    for _ in 0..2 {
        let reviewer = Address::generate(&env);
        client.add_review(&project_id, &reviewer, &5, &None);
    }

    // This review should be rejected under the old limit.
    let third_reviewer = Address::generate(&env);
    let result_before = client
        .mock_all_auths()
        .try_add_review(&project_id, &third_reviewer, &3, &None);
    assert_eq!(
        result_before,
        Err(Ok(ContractError::MaxProjectsExceeded.into()))
    );

    // Raise the limit to 4.
    client.set_max_reviews_per_project(&admin, &4);

    // The same review (new address) should now succeed.
    let new_reviewer = Address::generate(&env);
    client.add_review(&project_id, &new_reviewer, &3, &None);

    let stats = client.get_project_stats(&project_id);
    assert_eq!(stats.review_count, 3);
}

// ── Input validation ──────────────────────────────────────────────────────────

/// A maximum of 0 is semantically meaningless (no reviews allowed) and must
/// be rejected with `InvalidInput`.
#[test]
fn test_zero_max_is_invalid() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    let result = client
        .mock_all_auths()
        .try_set_max_reviews_per_project(&admin, &0);

    assert_eq!(result, Err(Ok(ContractError::InvalidInput.into())));
}

/// The maximum u32 value is a valid configuration (no overflow on comparison).
#[test]
fn test_max_u32_value_is_accepted() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    client.set_max_reviews_per_project(&admin, &u32::MAX);
    assert_eq!(client.get_max_reviews_per_project(), u32::MAX);
}

// ── Multiple projects share the same contract-level limit ────────────────────

/// Two projects created under the same configured limit both enforce that limit
/// independently: filling one project does not affect the other.
#[test]
fn test_multiple_projects_independently_respect_limit() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    let max: u32 = 4;
    client.set_max_reviews_per_project(&admin, &max);

    let project_a = create_test_project(&client, &admin, "Multi-A");
    let project_b = create_test_project(&client, &admin, "Multi-B");

    // Fill project A to the limit.
    for _ in 0..max {
        let reviewer = Address::generate(&env);
        client.add_review(&project_a, &reviewer, &3, &None);
    }

    // Project B should still accept reviews — A's count doesn't affect B.
    let reviewer_b = Address::generate(&env);
    client.add_review(&project_b, &reviewer_b, &5, &None);

    let stats_a = client.get_project_stats(&project_a);
    let stats_b = client.get_project_stats(&project_b);
    assert_eq!(stats_a.review_count, max);
    assert_eq!(stats_b.review_count, 1);

    // Now overflow project B shouldn't happen yet (only 1 of 4 reviews).
    let reviewer_b2 = Address::generate(&env);
    client.add_review(&project_b, &reviewer_b2, &4, &None);

    // A is still at the cap.
    let overflow_a = Address::generate(&env);
    let result = client
        .mock_all_auths()
        .try_add_review(&project_a, &overflow_a, &2, &None);
    assert_eq!(result, Err(Ok(ContractError::MaxProjectsExceeded.into())));
}

// ── Behavior without explicit configuration (storage fallback) ───────────────

/// When no value has ever been stored, review creation up to the default 500
/// must work. We use a small representative count here (3) to avoid the test
/// being slow, and confirm the limit is still the default.
#[test]
fn test_no_configured_value_falls_back_to_default() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    // Do NOT call set_max_reviews_per_project — rely on fallback.
    let project_id = create_test_project(&client, &admin, "Fallback-Project");

    // A small number of reviews well below 500 should succeed.
    for _ in 0..3 {
        let reviewer = Address::generate(&env);
        client.add_review(&project_id, &reviewer, &4, &None);
    }

    let stats = client.get_project_stats(&project_id);
    assert_eq!(stats.review_count, 3);
    assert_eq!(client.get_max_reviews_per_project(), MAX_REVIEWS_PER_PROJECT);
}

// ── Admin can update the value multiple times ─────────────────────────────────

/// The admin may call set_max_reviews_per_project more than once; the latest
/// value wins.
#[test]
fn test_admin_can_update_max_multiple_times() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    client.set_max_reviews_per_project(&admin, &10);
    assert_eq!(client.get_max_reviews_per_project(), 10);

    client.set_max_reviews_per_project(&admin, &200);
    assert_eq!(client.get_max_reviews_per_project(), 200);

    client.set_max_reviews_per_project(&admin, &1);
    assert_eq!(client.get_max_reviews_per_project(), 1);
}

// ── Edge: minimum value of 1 ─────────────────────────────────────────────────

/// A maximum of 1 means only one reviewer is allowed per project.
#[test]
fn test_max_of_one_allows_single_review_then_rejects() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    client.set_max_reviews_per_project(&admin, &1);
    let project_id = create_test_project(&client, &admin, "Single-Review-Project");

    let first_reviewer = Address::generate(&env);
    client.add_review(&project_id, &first_reviewer, &5, &None);

    let second_reviewer = Address::generate(&env);
    let result = client
        .mock_all_auths()
        .try_add_review(&project_id, &second_reviewer, &3, &None);
    assert_eq!(result, Err(Ok(ContractError::MaxProjectsExceeded.into())));
}
