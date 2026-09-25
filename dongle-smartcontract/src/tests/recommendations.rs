//! Focused recommendation-system tests (issue #749).

use crate::errors::ContractError;
use crate::tests::fixtures::setup_contract;
use crate::types::{
    DependencyRef, ProjectDependency, ProjectRegistrationParams, Recommendation,
    RecommendationABConfig, RecommendationAlgorithm, RecommendationVariant,
};
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};

fn register_project(
    client: &crate::DongleContractClient<'_>,
    env: &Env,
    owner: &Address,
    name: &str,
    category: &str,
    tags: Option<&[&str]>,
) -> u64 {
    let mut project_tags = Vec::new(env);
    if let Some(values) = tags {
        for tag in values {
            project_tags.push_back(String::from_str(env, tag));
        }
    }
    client
        .mock_all_auths()
        .register_project(&ProjectRegistrationParams {
            owner: owner.clone(),
            name: String::from_str(env, name),
            slug: String::from_str(env, &name.to_lowercase()),
            description: String::from_str(env, "Recommendation test project"),
            category: String::from_str(env, category),
            website: None,
            license: None,
            logo_cid: None,
            metadata_cid: None,
            tags: if project_tags.is_empty() {
                None
            } else {
                Some(project_tags)
            },
            social_links: None,
            launch_timestamp: None,
            bounty_url: None,
            repository_url: None,
        })
}

fn find_target(recommendations: &Vec<Recommendation>, project_id: u64) -> Option<Recommendation> {
    for recommendation in recommendations.iter() {
        if recommendation.target_project_id == project_id {
            return Some(recommendation);
        }
    }
    None
}

#[test]
fn ranks_related_project_using_all_core_signals() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let viewer = Address::generate(&env);
    let source = register_project(
        &client,
        &env,
        &owner,
        "source",
        "DeFi",
        Some(&["stellar", "payments"]),
    );
    let related = register_project(
        &client,
        &env,
        &owner,
        "related",
        "DeFi",
        Some(&["stellar", "wallet"]),
    );
    client.mock_all_auths().add_project_dependency(
        &source,
        &owner,
        &ProjectDependency {
            reference: DependencyRef {
                project_id: Some(related),
                external_cid: None,
                external_url: None,
                external_contract: None,
            },
            label: Some(String::from_str(&env, "dependency")),
            metadata_cid: None,
            added_at: 0,
            updated_at: 0,
        },
    );
    let reviewer = Address::generate(&env);
    client
        .mock_all_auths()
        .add_review(&related, &reviewer, &5u32, &None);
    client
        .mock_all_auths()
        .record_project_view(&viewer, &source);

    let recommendations = client
        .mock_all_auths()
        .recommend_projects(&viewer, &source, &10)
        .unwrap();
    let top = recommendations.get(0).unwrap();
    assert_eq!(top.target_project_id, related);
    assert_eq!(top.reference_project_id, Some(source));
    assert_eq!(top.audience, Some(viewer));
    assert!(top.score.unwrap() > 1_000);

    // A second query reuses the viewer/source/candidate record rather than
    // duplicating it in the global recommendation index.
    let repeated = client
        .mock_all_auths()
        .recommend_projects(&viewer, &source, &10)
        .unwrap();
    assert_eq!(repeated.get(0).unwrap().id, top.id);
    assert_eq!(client.get_recommendation_count(), 1);
}

#[test]
fn viewing_history_personalizes_unrelated_candidates() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let viewer = Address::generate(&env);
    let source = register_project(&client, &env, &owner, "source", "DeFi", None);
    let historical = register_project(&client, &env, &owner, "historical", "Infrastructure", None);
    let history_match = register_project(
        &client,
        &env,
        &owner,
        "historymatch",
        "Infrastructure",
        None,
    );

    client
        .mock_all_auths()
        .record_project_view(&viewer, &source);
    client
        .mock_all_auths()
        .record_project_view(&viewer, &historical);
    // Repeating a view refreshes rather than duplicating the history slot.
    client
        .mock_all_auths()
        .record_project_view(&viewer, &historical);
    assert_eq!(client.get_recommendation_view_history(&viewer).len(), 2);

    let recommendations = client
        .mock_all_auths()
        .recommend_projects(&viewer, &source, &10)
        .unwrap();
    assert!(find_target(&recommendations, history_match).is_some());
}

#[test]
fn ab_configuration_assigns_algorithm_deterministically() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let viewer = Address::generate(&env);
    let source = register_project(&client, &env, &owner, "source", "DeFi", None);
    let candidate = register_project(&client, &env, &owner, "candidate", "DeFi", None);
    let config = RecommendationABConfig {
        enabled: true,
        split_bps: 10_000,
        variant_a: RecommendationAlgorithm::Popular,
        variant_b: RecommendationAlgorithm::Similar,
        description: String::from_str(&env, "similarity treatment"),
    };

    let stranger = Address::generate(&env);
    assert_eq!(
        client
            .mock_all_auths()
            .try_set_recommendation_ab_config(&stranger, &config),
        Err(Ok(ContractError::Unauthorized))
    );
    client
        .mock_all_auths()
        .set_recommendation_ab_config(&admin, &config);
    assert_eq!(
        client.get_recommendation_variant(&viewer),
        RecommendationVariant::VariantB
    );
    assert_eq!(client.get_recommendation_ab_config(), config);

    client
        .mock_all_auths()
        .record_project_view(&viewer, &source);
    let recommendations = client
        .mock_all_auths()
        .recommend_projects(&viewer, &source, &1)
        .unwrap();
    let selected = recommendations.get(0).unwrap();
    assert_eq!(selected.target_project_id, candidate);
    assert_eq!(selected.algorithm, RecommendationAlgorithm::Similar);
    assert_eq!(selected.variant, RecommendationVariant::VariantB);
}

#[test]
fn conversion_funnel_and_variant_analytics_are_deduplicated() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let creator = Address::generate(&env);
    let viewer = Address::generate(&env);
    let target = register_project(&client, &env, &creator, "target", "DeFi", None);
    let recommendation_id = client
        .mock_all_auths()
        .create_recommendation(
            &creator,
            &target,
            &RecommendationAlgorithm::Popular,
            &None,
            &Some(viewer.clone()),
            &Some(1_000u64),
            &Some(String::from_str(&env, "conversion-test")),
        )
        .unwrap();

    assert_eq!(
        client
            .mock_all_auths()
            .try_record_recommendation_conversion(&recommendation_id, &viewer),
        Err(Ok(ContractError::RecommendationNoImpression))
    );
    client
        .mock_all_auths()
        .record_recommendation_impression(&recommendation_id, &viewer);
    client
        .mock_all_auths()
        .record_recommendation_impression(&recommendation_id, &viewer);
    client
        .mock_all_auths()
        .record_recommendation_click(&recommendation_id, &viewer);
    client
        .mock_all_auths()
        .record_recommendation_click(&recommendation_id, &viewer);
    client
        .mock_all_auths()
        .record_recommendation_conversion(&recommendation_id, &viewer);

    let analytics = client
        .get_recommendation_analytics(&recommendation_id)
        .unwrap();
    assert_eq!(analytics.impressions, 1);
    assert_eq!(analytics.clicks, 1);
    assert_eq!(analytics.conversions, 1);
    assert_eq!(analytics.click_through_rate_ppm, 1_000_000);
    assert_eq!(analytics.conversion_rate_ppm, 1_000_000);

    let arm = client.get_recommendation_ab_analytics(&RecommendationVariant::VariantA);
    assert_eq!(arm.impressions, 1);
    assert_eq!(arm.clicks, 1);
    assert_eq!(arm.conversions, 1);
    assert_eq!(arm.conversion_rate_ppm, 1_000_000);

    assert_eq!(
        client
            .mock_all_auths()
            .try_record_recommendation_conversion(&recommendation_id, &viewer),
        Err(Ok(ContractError::RecommendationAlreadyConverted))
    );
}
