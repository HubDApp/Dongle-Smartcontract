//! Tests for rule-based ("smart") collections — issue #906.

use crate::errors::ContractError;
use crate::tests::fixtures::setup_contract;
use crate::types::{CurationRule, ProjectRegistrationParams};
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};

fn register_project(
    env: &Env,
    client: &crate::DongleContractClient<'_>,
    owner: &Address,
    name: &str,
    category: &str,
    tags: Vec<String>,
) -> u64 {
    let slug = name.to_lowercase().replace(' ', "-");
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(env, name),
        slug: String::from_str(env, &slug),
        description: String::from_str(env, "Curation test project"),
        category: String::from_str(env, category),
        website: None,
        license: None,
        logo_cid: None,
        metadata_cid: None,
        tags: Some(tags),
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
        repository_url: None,
        language_code: None,
    };
    client.mock_all_auths().register_project(&params)
}

fn tag(env: &Env, value: &str) -> String {
    String::from_str(env, value)
}

#[test]
fn test_create_smart_collection_admin_only() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let stranger = Address::generate(&env);

    let rules: Vec<CurationRule> = Vec::new(&env);
    let result = client.mock_all_auths().try_create_smart_collection(
        &stranger,
        &String::from_str(&env, "Auto DeFi"),
        &String::from_str(&env, "Curated automatically"),
        &rules,
        &true,
    );

    assert_eq!(result, Err(Ok(ContractError::AdminOnly)));
}

#[test]
fn test_smart_collection_rejects_rule_without_condition() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let mut rules: Vec<CurationRule> = Vec::new(&env);
    rules.push_back(CurationRule {
        tag: None,
        min_rating: None,
        category: None,
    });

    let result = client.mock_all_auths().try_create_smart_collection(
        &admin,
        &String::from_str(&env, "Auto DeFi"),
        &String::from_str(&env, "Curated automatically"),
        &rules,
        &true,
    );

    assert_eq!(result, Err(Ok(ContractError::InvalidCurationRule)));
}

#[test]
fn test_refresh_populates_collection_by_tag_and_category() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let mut defi_tags: Vec<String> = Vec::new(&env);
    defi_tags.push_back(tag(&env, "defi"));
    let defi_project = register_project(
        &env,
        &client,
        &owner,
        "Alpha Defi",
        "DeFi",
        defi_tags.clone(),
    );

    let mut nft_tags: Vec<String> = Vec::new(&env);
    nft_tags.push_back(tag(&env, "nft"));
    let nft_project = register_project(&env, &client, &owner, "Beta Nft", "NFT", nft_tags);

    let mut rules: Vec<CurationRule> = Vec::new(&env);
    rules.push_back(CurationRule {
        tag: Some(tag(&env, "DeFi")),
        min_rating: None,
        category: Some(tag(&env, "DeFi")),
    });

    let collection_id = client.mock_all_auths().create_smart_collection(
        &admin,
        &String::from_str(&env, "Auto DeFi"),
        &String::from_str(&env, "Curated automatically"),
        &rules,
        &true,
    );

    let result = client
        .mock_all_auths()
        .refresh_smart_collection(&admin, &collection_id);

    assert_eq!(result.added, 1);
    assert_eq!(result.removed, 0);
    assert_eq!(result.total_matched, 1);

    let members = client.list_smart_collection_projects(&collection_id);
    assert_eq!(members.len(), 1);
    assert_eq!(members.get(0), Some(defi_project));
    assert!(members.iter().all(|id| id != nft_project));

    // A second refresh is idempotent.
    let second = client
        .mock_all_auths()
        .refresh_smart_collection(&admin, &collection_id);
    assert_eq!(second.added, 0);
    assert_eq!(second.removed, 0);
}

#[test]
fn test_rule_versioning_and_rollback() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let mut defi_tags: Vec<String> = Vec::new(&env);
    defi_tags.push_back(tag(&env, "defi"));
    register_project(&env, &client, &owner, "Alpha Defi", "DeFi", defi_tags);

    let mut rules_v1: Vec<CurationRule> = Vec::new(&env);
    rules_v1.push_back(CurationRule {
        tag: Some(tag(&env, "defi")),
        min_rating: None,
        category: None,
    });

    let collection_id = client.mock_all_auths().create_smart_collection(
        &admin,
        &String::from_str(&env, "Versioned"),
        &String::from_str(&env, "Rule versioning"),
        &rules_v1,
        &true,
    );

    let version_v1 = client
        .get_smart_collection(&collection_id)
        .unwrap()
        .rule_version;
    assert_eq!(version_v1, 1);

    // Version 2 uses a different condition.
    let mut rules_v2: Vec<CurationRule> = Vec::new(&env);
    rules_v2.push_back(CurationRule {
        tag: Some(tag(&env, "nft")),
        min_rating: None,
        category: None,
    });
    let version_v2 =
        client
            .mock_all_auths()
            .update_smart_collection_rules(&admin, &collection_id, &rules_v2);
    assert_eq!(version_v2, 2);

    let versions = client.list_smart_collection_versions(&collection_id);
    assert_eq!(versions.len(), 2);

    // Roll back to version 1 — the restored rules become version 3.
    let version_v3 =
        client
            .mock_all_auths()
            .rollback_smart_collection_rules(&admin, &collection_id, &1);
    assert_eq!(version_v3, 3);

    let collection = client.get_smart_collection(&collection_id).unwrap();
    assert_eq!(collection.rule_version, 3);
    assert_eq!(collection.rules.len(), 1);
    assert_eq!(
        collection.rules.get(0).unwrap().tag,
        Some(tag(&env, "defi"))
    );

    // Rolling back to an unknown version fails.
    let result =
        client
            .mock_all_auths()
            .try_rollback_smart_collection_rules(&admin, &collection_id, &99);
    assert_eq!(
        result,
        Err(Ok(ContractError::SmartCollectionRuleVersionNotFound))
    );
}

#[test]
fn test_refresh_removes_project_that_stops_matching() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let mut defi_tags: Vec<String> = Vec::new(&env);
    defi_tags.push_back(tag(&env, "defi"));
    let project_id = register_project(&env, &client, &owner, "Alpha Defi", "DeFi", defi_tags);

    let mut rules: Vec<CurationRule> = Vec::new(&env);
    rules.push_back(CurationRule {
        tag: Some(tag(&env, "defi")),
        min_rating: None,
        category: None,
    });

    let collection_id = client.mock_all_auths().create_smart_collection(
        &admin,
        &String::from_str(&env, "Auto DeFi"),
        &String::from_str(&env, "Curated automatically"),
        &rules,
        &true,
    );

    client
        .mock_all_auths()
        .refresh_smart_collection(&admin, &collection_id);
    assert_eq!(
        client.list_smart_collection_projects(&collection_id).len(),
        1
    );

    // Narrow the rules so the project no longer matches.
    let mut narrow: Vec<CurationRule> = Vec::new(&env);
    narrow.push_back(CurationRule {
        tag: Some(tag(&env, "nft")),
        min_rating: None,
        category: None,
    });
    client
        .mock_all_auths()
        .update_smart_collection_rules(&admin, &collection_id, &narrow);

    let result = client
        .mock_all_auths()
        .refresh_smart_collection(&admin, &collection_id);
    assert_eq!(result.removed, 1);
    assert!(client
        .list_smart_collection_projects(&collection_id)
        .iter()
        .all(|id| id != project_id));
}
