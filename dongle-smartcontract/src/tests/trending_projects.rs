//! Tests for Dynamic Trending Projects (#912).
//!
//! Covers:
//! - Trending score calculation based on reviews, follows, and registration volume
//! - Time window support (24h, 7d, 30d, all-time)
//! - Trend velocity calculation
//! - Recalculate / lazy evaluation cache behavior

use crate::constants::{
    TRENDING_WINDOW_24H, TRENDING_WINDOW_30D, TRENDING_WINDOW_7D, TRENDING_WINDOW_ALL_TIME,
};
use crate::tests::fixtures::{create_test_project, setup_contract};
use soroban_sdk::{testutils::Address as _, Address, Env, String};

#[test]
fn test_trending_projects_ranking_and_velocity() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_contract(&env);

    let owner = Address::generate(&env);
    let p1 = create_test_project(&client, &owner, "Project Low Activity");
    let p2 = create_test_project(&client, &owner, "Project High Activity");

    // Add followers to p2
    let follower = Address::generate(&env);
    client.follow_project(&p2, &follower);

    // Add review to p2
    let reviewer = Address::generate(&env);
    client.submit_review(
        &p2,
        &reviewer,
        &5,
        &String::from_str(&env, "Qm123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmn"),
        &None,
    );

    // Query trending projects for 24h window
    let trending = client.get_trending_projects(&TRENDING_WINDOW_24H, &10);
    assert!(trending.len() >= 2);

    // p2 should rank higher than p1 due to follows and reviews
    assert_eq!(trending.get(0).unwrap().project.id, p2);
    assert!(trending.get(0).unwrap().trending_score > trending.get(1).unwrap().trending_score);
    assert!(trending.get(0).unwrap().trend_velocity > 0);
}

#[test]
fn test_trending_windows_and_recalculate() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_contract(&env);

    let owner = Address::generate(&env);
    let p1 = create_test_project(&client, &owner, "Project Alpha");

    let reviewer = Address::generate(&env);
    client.submit_review(
        &p1,
        &reviewer,
        &5,
        &String::from_str(&env, "Qm123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmn"),
        &None,
    );

    // Check all windows
    let list_24h = client.get_trending_projects(&TRENDING_WINDOW_24H, &10);
    let list_7d = client.get_trending_projects(&TRENDING_WINDOW_7D, &10);
    let list_30d = client.get_trending_projects(&TRENDING_WINDOW_30D, &10);
    let list_all = client.get_trending_projects(&TRENDING_WINDOW_ALL_TIME, &10);

    assert!(!list_24h.is_empty());
    assert!(!list_7d.is_empty());
    assert!(!list_30d.is_empty());
    assert!(!list_all.is_empty());

    // Explicit recalculate
    let recalculated = client.recalculate_trending_projects(&TRENDING_WINDOW_7D);
    assert!(!recalculated.is_empty());
    assert_eq!(recalculated.get(0).unwrap().project.id, p1);
}
