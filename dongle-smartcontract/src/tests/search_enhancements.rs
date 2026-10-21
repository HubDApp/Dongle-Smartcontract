//! Tests for Search Enhancements (Issue #909).
//!
//! Covers:
//! - Wildcard search with '*'
//! - Phrase search with exact matching
//! - search_projects_with_limit endpoint
//! - Relevance and rating ranking

use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::ProjectRegistrationParams;
use crate::DongleContractClient;
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};

fn register_searchable_project(
    client: &DongleContractClient<'_>,
    owner: &Address,
    name: &str,
    description: &str,
) -> u64 {
    let env = &client.env;
    let slug = name.to_lowercase().replace(' ', "-");
    client.mock_all_auths().register_project(&ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(env, name),
        slug: String::from_str(env, &slug),
        description: String::from_str(env, description),
        category: String::from_str(env, "Infrastructure"),
        website: None,
        license: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
        repository_url: None,
    })
}

#[test]
fn test_wildcard_search() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_contract(&env);

    let owner = Address::generate(&env);
    let p1 = register_searchable_project(&client, &owner, "StellarPay Gateway", "Fast payment gateway");
    let p2 = register_searchable_project(&client, &owner, "AnchorPay Network", "Cross-border settlements");
    let p3 = register_searchable_project(&client, &owner, "Other Project", "Completely different thing");

    // Wildcard query "*Pay*" should match StellarPay and AnchorPay
    let results = client.search_projects(&String::from_str(&env, "*Pay*"), &0, &10);
    assert!(results.iter().any(|p| p.id == p1));
    assert!(results.iter().any(|p| p.id == p2));
    assert!(!results.iter().any(|p| p.id == p3));
}

#[test]
fn test_phrase_search() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_contract(&env);

    let owner = Address::generate(&env);
    let p1 = register_searchable_project(&client, &owner, "Alpha DEX", "Automated market maker protocol");
    let p2 = register_searchable_project(&client, &owner, "Beta DEX", "Order book protocol for markets");

    // Exact phrase query in quotes
    let results = client.search_projects(&String::from_str(&env, "\"automated market maker\""), &0, &10);
    assert_eq!(results.len(), 1);
    assert_eq!(results.get(0).unwrap().id, p1);
}

#[test]
fn test_search_projects_with_limit_endpoint() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_contract(&env);

    let owner = Address::generate(&env);
    register_searchable_project(&client, &owner, "Project One", "Test description 1");
    register_searchable_project(&client, &owner, "Project Two", "Test description 2");

    let results = client.search_projects_with_limit(&String::from_str(&env, "Project"), &1);
    assert_eq!(results.len(), 1);
}
