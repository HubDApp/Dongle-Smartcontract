#![cfg(test)]

use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::{RecommendationOptions, RecommendationStrategy};
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};

#[test]
fn test_get_recommended_projects_and_strategies() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let p1 = create_test_project(&client, &owner, "ProjectAlpha");
    let p2 = create_test_project(&client, &owner, "ProjectBeta");
    let p3 = create_test_project(&client, &owner, "ProjectGamma");

    // Add tags to p1 and p2
    let mut tags1 = Vec::new(&env);
    tags1.push_back(String::from_str(&env, "defi"));
    tags1.push_back(String::from_str(&env, "lending"));
    client.set_project_tags(&p1, &tags1);

    let mut tags2 = Vec::new(&env);
    tags2.push_back(String::from_str(&env, "defi"));
    client.set_project_tags(&p2, &tags2);

    // Link p1 and p3
    client.link_projects(&p1, &p3);

    // Default similar recommendations
    let recs = client.get_recommended_projects(&p1, &5);
    assert!(!recs.is_empty());
    assert!(recs.contains(p2), "p2 shares defi tag with p1");

    // Complementary recommendation strategy
    let complementary = client.get_recommended_projects_with_options(&p1, &RecommendationOptions {
        strategy: RecommendationStrategy::Complementary,
        count: 5,
        max_per_category: 0,
    });
    assert!(!complementary.is_empty());
    assert_eq!(complementary.get(0).unwrap(), p3, "linked project p3 has highest complementary score");
}

#[test]
fn test_track_recommendation_clicks() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    let p1 = create_test_project(&client, &owner, "SourceProject");
    let p2 = create_test_project(&client, &owner, "RecommendedProject");

    let clicks_before = client.get_recommendation_clicks(&p1, &p2);
    assert_eq!(clicks_before, 0);

    client.track_recommendation_click(&user, &p1, &p2);

    let clicks_after = client.get_recommendation_clicks(&p1, &p2);
    assert_eq!(clicks_after, 1);
}
