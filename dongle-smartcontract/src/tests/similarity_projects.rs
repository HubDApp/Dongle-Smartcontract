//! Tests for Similar and Duplicate Projects Detection (#913).
//!
//! Covers:
//! - Similarity scoring based on name, description, tags, and dependencies
//! - find_similar_projects endpoint with threshold filtering and descending sorting
//! - Fuzzy string matching
//! - Duplicate dispute integration

use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::ProjectRegistrationParams;
use crate::DongleContractClient;
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};

fn register_project_with_details(
    client: &DongleContractClient<'_>,
    owner: &Address,
    name: &str,
    description: &str,
    tags: Option<Vec<String>>,
) -> u64 {
    let env = &client.env;
    let slug = name.to_lowercase().replace(' ', "-");
    client.mock_all_auths().register_project(&ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(env, name),
        slug: String::from_str(env, &slug),
        description: String::from_str(env, description),
        category: String::from_str(env, "Finance"),
        website: None,
        license: None,
        logo_cid: None,
        metadata_cid: None,
        tags,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
        repository_url: None,
    })
}

#[test]
fn test_find_similar_projects_fuzzy_and_sorted() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_contract(&env);

    let owner = Address::generate(&env);

    let p_orig = register_project_with_details(
        &client,
        &owner,
        "Uniswap Protocol",
        "Decentralized automated liquidity protocol",
        None,
    );

    let p_near_dup = register_project_with_details(
        &client,
        &owner,
        "UniSwap Protocol v2",
        "Decentralized automated liquidity exchange",
        None,
    );

    let p_unrelated = register_project_with_details(
        &client,
        &owner,
        "Crypto Kitties Game",
        "Collect and breed digital cats",
        None,
    );

    // Search similar to p_orig with 30% threshold (3000 bps)
    let similar = client.find_similar_projects(&p_orig, &3000);
    assert!(!similar.is_empty());

    // Near duplicate should be found and scored high
    let first = similar.get(0).unwrap();
    assert_eq!(first.project_id, p_near_dup);
    assert!(first.similarity_score > 5000);

    // Unrelated should either not be present or score lower
    if similar.len() > 1 {
        let second = similar.get(1).unwrap();
        assert!(first.similarity_score >= second.similarity_score);
    }
}

#[test]
fn test_open_duplicate_dispute_with_similarity_gate() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_contract(&env);

    let owner = Address::generate(&env);
    let creator = Address::generate(&env);

    let p1 = register_project_with_details(&client, &owner, "Dongle Dex", "Fast DEX", None);
    let p2 = register_project_with_details(&client, &owner, "Dongle DEX Pro", "Fast DEX Pro", None);
    let p3 = register_project_with_details(&client, &owner, "Game Guild", "Gaming NFT", None);

    let evidence_cid = String::from_str(&env, "Qm123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmn");

    // Attempting dispute between unrelated projects p1 and p3 with high threshold (8000 bps = 80%) should fail
    let err_result = client.try_open_duplicate_dispute_with_similarity(
        &p3,
        &p1,
        &creator,
        &evidence_cid,
        &8000,
    );
    assert_eq!(err_result, Err(Ok(ContractError::SimilarityThresholdNotMet)));

    // Dispute between similar projects p1 and p2 should succeed
    let dispute_id = client.open_duplicate_dispute_with_similarity(
        &p2,
        &p1,
        &creator,
        &evidence_cid,
        &3000,
    );
    assert!(dispute_id > 0);
}
