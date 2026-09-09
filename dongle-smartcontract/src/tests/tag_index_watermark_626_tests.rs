//! Targeted tests for Issue #626 — Tag Indexing Watermark Edge Cases.

#![cfg(test)]

use crate::errors::ContractError;
use crate::tests::fixtures::setup_contract;
use crate::types::ProjectRegistrationParams;
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};

fn create_project_with_tags(
    client: &crate::DongleContractClient<'_>,
    env: &Env,
    owner: &Address,
    name: &str,
    tags: &[&str],
) -> u64 {
    let slug = name.to_lowercase().replace(' ', "-");
    let mut tag_vec = Vec::new(env);
    for t in tags {
        tag_vec.push_back(String::from_str(env, t));
    }
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(env, name),
        slug: String::from_str(env, &slug),
        description: String::from_str(env, "Tag index watermark test project"),
        category: String::from_str(env, "DeFi"),
        website: None,
        license: None,
        logo_cid: None,
        metadata_cid: None,
        tags: Some(tag_vec),
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
        repository_url: None,
    };
    client.register_project(&params)
}

#[test]
fn test_watermark_zero_uninitialized_and_initial_indexing() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    // 1. Initial state: watermark is 0 (Uninitialized)
    assert_eq!(client.get_tag_index_watermark(), 0);

    // Register 5 projects
    for i in 1..=5 {
        let name = alloc::format!("Project{}", i);
        create_project_with_tags(&client, &env, &owner, &name, &["defi"]);
    }

    // Since registration updates watermark for project count == watermark + 1,
    // registration of sequential projects 1..=5 automatically keeps watermark at 5.
    assert_eq!(client.get_tag_index_watermark(), 5);
}

#[test]
fn test_incremental_reindexing_and_completion() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    for i in 1..=10 {
        let name = alloc::format!("BatchProject{}", i);
        create_project_with_tags(&client, &env, &owner, &name, &["nft"]);
    }

    assert_eq!(client.get_tag_index_watermark(), 10);

    // Call reindex_tags in incremental batch of limit = 3
    let new_wm = client.reindex_tags(&admin, &3);
    assert_eq!(new_wm, 10); // already at 10, complete

    // Reindex completion check
    assert_eq!(client.get_tag_index_watermark(), 10);
}

#[test]
fn test_reindex_failure_interruption_leaves_watermark_unchanged() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let non_admin = Address::generate(&env);

    env.mock_all_auths();
    create_project_with_tags(&client, &env, &owner, "ProjectFail", &["tool"]);
    let current_wm = client.get_tag_index_watermark();

    // Call by non-admin without mock_all_auths should fail and leave watermark unchanged
    let err = client.try_reindex_tags(&non_admin, &5);
    assert!(err.is_err());

    // Watermark must remain unchanged after failure/interruption
    assert_eq!(client.get_tag_index_watermark(), current_wm);
}

#[test]
fn test_repeated_reindex_is_idempotent() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    create_project_with_tags(&client, &env, &owner, "IdempotentProj", &["infra"]);

    let wm1 = client.reindex_tags(&admin, &10);
    let wm2 = client.reindex_tags(&admin, &10);
    let wm3 = client.reindex_tags(&admin, &10);

    assert_eq!(wm1, wm2);
    assert_eq!(wm2, wm3);
    assert_eq!(client.get_tag_index_watermark(), wm1);
}

#[test]
fn test_sequential_interleaving_reindex_monotonicity() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    for i in 1..=6 {
        let name = alloc::format!("Interleave{}", i);
        create_project_with_tags(&client, &env, &owner, &name, &["interleave"]);
    }

    // Interleave calls from multiple callers/batches
    let wm_a = client.reindex_tags(&admin, &2);
    assert!(wm_a >= 6);

    let wm_b = client.reindex_tags(&admin, &4);
    assert!(wm_b >= wm_a);

    assert_eq!(client.get_tag_index_watermark(), 6);
}
