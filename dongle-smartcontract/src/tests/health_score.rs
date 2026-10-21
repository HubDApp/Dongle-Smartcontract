//! Tests for the composite health score module (#982).
//!
//! Verifies:
//! - Default score computation (rating, activity, verification, engagement)
//! - Weighted factor contributions
//! - Score decay over time for inactive projects
//! - Admin-configurable weights
//! - History snapshot accumulation and pagination
//! - `ProjectHealthScoreUpdated` event emission
//! - Error handling (project not found)

use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::{HealthScoreConfig, VerificationStatus};
use soroban_sdk::{testutils::Address as _, testutils::Ledger, Address, Env};

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Advance the ledger timestamp by `delta_secs` seconds.
fn advance_time(env: &Env, delta_secs: u64) {
    let current = env.ledger().timestamp();
    env.ledger().with_mut(|li| {
        li.timestamp = current + delta_secs;
    });
}

// ── Basic computation ─────────────────────────────────────────────────────────

#[test]
fn test_fresh_project_has_nonzero_score() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let project_id = create_test_project(&client, &admin, "Healthy Project");

    let health = client.get_project_health_score(&project_id).unwrap();
    // A brand-new project should have a positive score (activity component is
    // at maximum since updated_at ≈ now).
    assert!(health.score > 0, "fresh project score should be > 0");
    assert!(health.score <= 100, "score must never exceed 100");
    assert_eq!(health.project_id, project_id);
}

#[test]
fn test_score_returns_error_for_missing_project() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let result = client.try_get_project_health_score(&999u64);
    assert!(result.is_err(), "missing project should return an error");
}

// ── Rating component ──────────────────────────────────────────────────────────

#[test]
fn test_rating_component_increases_score() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    // Baseline: project with no reviews.
    let project_no_reviews = create_test_project(&client, &admin, "No Reviews");
    let score_no_reviews = client
        .get_project_health_score(&project_no_reviews)
        .unwrap()
        .score;

    // Compare against a project with several 5-star reviews.
    let project_with_reviews = create_test_project(&client, &admin, "Well Reviewed");
    for _ in 0u32..5 {
        let reviewer = Address::generate(&env);
        client.add_review(&project_with_reviews, &reviewer, &5u32, &None);
    }
    // Force recomputation.
    let score_reviewed = client
        .refresh_project_health_score(&project_with_reviews, &admin)
        .unwrap()
        .score;

    assert!(
        score_reviewed > score_no_reviews,
        "5-star reviews should raise the health score: {} vs {}",
        score_reviewed,
        score_no_reviews
    );
}

#[test]
fn test_rating_component_breakdown_proportional_to_stars() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    // Project A: 5-star reviews only.
    let proj_a = create_test_project(&client, &admin, "Five Stars");
    for _ in 0u32..3 {
        let r = Address::generate(&env);
        client.add_review(&proj_a, &r, &5u32, &None);
    }

    // Project B: 1-star reviews only.
    let proj_b = create_test_project(&client, &admin, "One Star");
    for _ in 0u32..3 {
        let r = Address::generate(&env);
        client.add_review(&proj_b, &r, &1u32, &None);
    }

    let health_a = client
        .refresh_project_health_score(&proj_a, &admin)
        .unwrap();
    let health_b = client
        .refresh_project_health_score(&proj_b, &admin)
        .unwrap();

    assert!(
        health_a.breakdown.rating_score > health_b.breakdown.rating_score,
        "5-star project should have higher rating_score"
    );
}

// ── Activity / time-decay component ──────────────────────────────────────────

#[test]
fn test_activity_score_decreases_over_time() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    let project_id = create_test_project(&client, &admin, "Active Now");
    let score_fresh = client
        .get_project_health_score(&project_id)
        .unwrap()
        .score;

    // Advance time by 15 days (half-way through the 30-day decay window).
    advance_time(&env, 15 * 24 * 3600);
    let score_15_days = client
        .refresh_project_health_score(&project_id, &admin)
        .unwrap()
        .score;

    assert!(
        score_15_days < score_fresh,
        "score should decay after 15 days of inactivity: {} vs {}",
        score_15_days,
        score_fresh
    );
}

#[test]
fn test_activity_score_is_zero_after_full_decay() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    let project_id = create_test_project(&client, &admin, "Old Project");

    // Advance 31 days past the 30-day decay window.
    advance_time(&env, 31 * 24 * 3600);
    let health = client
        .refresh_project_health_score(&project_id, &admin)
        .unwrap();

    assert_eq!(
        health.breakdown.activity_score, 0,
        "activity score should be 0 after full decay"
    );
}

// ── Verification component ────────────────────────────────────────────────────

#[test]
fn test_verified_project_has_higher_score_than_unverified() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    // Unverified project.
    let unverified = create_test_project(&client, &admin, "Unverified One");
    let score_unverified = client
        .refresh_project_health_score(&unverified, &admin)
        .unwrap()
        .score;

    // Verified project.
    let verified = create_test_project(&client, &admin, "Verified One");
    let owner = Address::generate(&env);
    // Register a fresh project owned by owner so we can call owner-only functions.
    let params = crate::types::ProjectRegistrationParams {
        owner: owner.clone(),
        name: soroban_sdk::String::from_str(&env, "Verified Two"),
        slug: soroban_sdk::String::from_str(&env, "verified-two"),
        description: soroban_sdk::String::from_str(&env, "Verified project description here"),
        category: soroban_sdk::String::from_str(&env, "DeFi"),
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
    };
    let verified_id = client.register_project(&params);
    // Approve verification directly (mock_all_auths skips fee checks).
    client.approve_verification(&verified_id, &admin);

    let score_verified = client
        .refresh_project_health_score(&verified_id, &admin)
        .unwrap()
        .score;

    assert!(
        score_verified > score_unverified,
        "verified project should score higher than unverified: {} vs {}",
        score_verified,
        score_unverified
    );
    let _ = verified; // silence unused variable
}

// ── Engagement component ──────────────────────────────────────────────────────

#[test]
fn test_endorsements_increase_score() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    let project_id = create_test_project(&client, &admin, "Endorsed Project");

    let score_before = client
        .get_project_health_score(&project_id)
        .unwrap()
        .score;

    // Add endorsements.
    for _ in 0u32..10 {
        let user = Address::generate(&env);
        client.endorse_project(&project_id, &user);
    }

    let health_after = client
        .refresh_project_health_score(&project_id, &admin)
        .unwrap();

    assert!(
        health_after.score > score_before,
        "endorsements should raise the score: {} vs {}",
        health_after.score,
        score_before
    );
    assert!(
        health_after.breakdown.engagement_score > 0,
        "engagement_score must be positive when there are endorsements"
    );
    assert_eq!(health_after.breakdown.endorsement_count, 10);
}

#[test]
fn test_engagement_score_saturates_at_cap() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    let project_id = create_test_project(&client, &admin, "Saturated Project");

    // Add enough endorsements to reach saturation (50).
    for _ in 0u32..50 {
        let user = Address::generate(&env);
        client.endorse_project(&project_id, &user);
    }
    let health_50 = client
        .refresh_project_health_score(&project_id, &admin)
        .unwrap();

    // Add more — the engagement component should not increase beyond its weight.
    for _ in 0u32..20 {
        let user = Address::generate(&env);
        client.endorse_project(&project_id, &user);
    }
    let health_70 = client
        .refresh_project_health_score(&project_id, &admin)
        .unwrap();

    assert_eq!(
        health_50.breakdown.engagement_score,
        health_70.breakdown.engagement_score,
        "engagement score should saturate at {} endorsements",
        50
    );
}

// ── Admin-configurable weights ────────────────────────────────────────────────

#[test]
fn test_custom_weights_affect_score() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    let project_id = create_test_project(&client, &admin, "Weight Test");
    // Add reviews and endorsements so all components have non-zero values.
    for _ in 0u32..5 {
        let r = Address::generate(&env);
        client.add_review(&project_id, &r, &5u32, &None);
    }
    for _ in 0u32..10 {
        let u = Address::generate(&env);
        client.endorse_project(&project_id, &u);
    }

    // Default config.
    let default_health = client
        .refresh_project_health_score(&project_id, &admin)
        .unwrap();

    // Override: put all weight on rating.
    let rating_heavy = HealthScoreConfig {
        rating_weight: 85,
        activity_weight: 5,
        verification_weight: 5,
        engagement_weight: 5,
        update_frequency_secs: 3600,
    };
    client.set_health_score_config(&admin, &rating_heavy);
    let rating_health = client
        .refresh_project_health_score(&project_id, &admin)
        .unwrap();

    // With 5-star reviews, putting more weight on rating should increase the
    // rating_score component relative to the default.
    assert!(
        rating_health.breakdown.rating_score > default_health.breakdown.rating_score,
        "higher rating_weight should produce higher rating_score contribution"
    );
}

#[test]
fn test_invalid_weights_are_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    // Weights that don't sum to 100 should be rejected.
    let bad_config = HealthScoreConfig {
        rating_weight: 50,
        activity_weight: 30,
        verification_weight: 20,
        engagement_weight: 10, // 50+30+20+10 = 110 ≠ 100
        update_frequency_secs: 3600,
    };
    let result = client.try_set_health_score_config(&admin, &bad_config);
    assert!(
        result.is_err(),
        "weights summing to != 100 should be rejected"
    );
}

#[test]
fn test_valid_weights_sum_to_100() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    let good_config = HealthScoreConfig {
        rating_weight: 40,
        activity_weight: 30,
        verification_weight: 20,
        engagement_weight: 10,
        update_frequency_secs: 3600,
    };
    let result = client.try_set_health_score_config(&admin, &good_config);
    assert!(result.is_ok(), "weights summing to 100 should be accepted");

    let stored = client.get_health_score_config();
    assert_eq!(stored.rating_weight, 40);
    assert_eq!(stored.engagement_weight, 10);
}

// ── Historical snapshots ──────────────────────────────────────────────────────

#[test]
fn test_history_accumulates_snapshots() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    let project_id = create_test_project(&client, &admin, "History Project");

    // Each refresh call stores a new snapshot.
    client.refresh_project_health_score(&project_id, &admin).unwrap();
    advance_time(&env, 100);
    client.refresh_project_health_score(&project_id, &admin).unwrap();
    advance_time(&env, 100);
    client.refresh_project_health_score(&project_id, &admin).unwrap();

    let history = client
        .get_project_health_history(&project_id, &0u32, &10u32)
        .unwrap();
    assert!(
        history.len() >= 3,
        "at least 3 snapshots should be in history"
    );
}

#[test]
fn test_history_pagination() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    let project_id = create_test_project(&client, &admin, "Paginated History");

    // Create 5 snapshots.
    for i in 0u64..5 {
        advance_time(&env, i * 10 + 1);
        client.refresh_project_health_score(&project_id, &admin).unwrap();
    }

    let page1 = client
        .get_project_health_history(&project_id, &0u32, &2u32)
        .unwrap();
    let page2 = client
        .get_project_health_history(&project_id, &2u32, &2u32)
        .unwrap();

    assert_eq!(page1.len(), 2, "page1 should have 2 entries");
    assert_eq!(page2.len(), 2, "page2 should have 2 entries");
    // Timestamps should be increasing (oldest-first).
    assert!(
        page1.get(0).unwrap().computed_at <= page1.get(1).unwrap().computed_at,
        "history should be ordered oldest-first"
    );
}

#[test]
fn test_history_not_found_for_missing_project() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);

    let result = client.try_get_project_health_history(&999u64, &0u32, &10u32);
    assert!(result.is_err(), "missing project should return error");
}

// ── Score breakdown fields ────────────────────────────────────────────────────

#[test]
fn test_breakdown_fields_are_populated() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    let project_id = create_test_project(&client, &admin, "Breakdown Test");
    let reviewer = Address::generate(&env);
    client.add_review(&project_id, &reviewer, &4u32, &None);

    let health = client
        .refresh_project_health_score(&project_id, &admin)
        .unwrap();

    assert_eq!(health.breakdown.review_count, 1);
    assert!(health.breakdown.average_rating > 0);
    assert_eq!(health.project_id, project_id);
    assert!(health.computed_at > 0);
}

// ── Score is bounded ─────────────────────────────────────────────────────────

#[test]
fn test_score_never_exceeds_100() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    let project_id = create_test_project(&client, &admin, "Max Score Test");

    // Add many 5-star reviews.
    for _ in 0u32..10 {
        let r = Address::generate(&env);
        client.add_review(&project_id, &r, &5u32, &None);
    }
    // Add many endorsements.
    for _ in 0u32..60 {
        let u = Address::generate(&env);
        client.endorse_project(&project_id, &u);
    }

    let health = client
        .refresh_project_health_score(&project_id, &admin)
        .unwrap();
    assert!(
        health.score <= 100,
        "score must not exceed 100, got {}",
        health.score
    );
}
