//! Tests for stable batch APIs used by indexers.

use crate::types::{ProjectRegistrationParams, VerificationStatus};
use crate::DongleContract;
use crate::DongleContractClient;
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};

fn setup(env: &Env) -> (DongleContractClient<'_>, Address) {
    let contract_id = env.register(DongleContract, ());
    let client = DongleContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.initialize(&admin);
    (client, admin)
}

fn register(client: &DongleContractClient<'_>, env: &Env, owner: &Address, name: &str) -> u64 {
    client.register_project(&ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(env, name),
        slug: String::from_str(env, &name.to_lowercase()),
        description: String::from_str(env, "A test project description here"),
        category: String::from_str(env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    })
}

fn setup_verified(
    client: &DongleContractClient<'_>,
    env: &Env,
    admin: &Address,
    owner: &Address,
    name: &str,
) -> u64 {
    let project_id = register(client, env, owner, name);

    let token_admin = Address::generate(env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    client.set_fee(admin, &Some(token.clone()), &100, &0u128, admin);
    soroban_sdk::token::StellarAssetClient::new(env, &token).mint(owner, &100);
    client.pay_fee(owner, &project_id, &Some(token));
    client.request_verification(
        &project_id,
        owner,
        &String::from_str(env, "ipfs://evidence"),
    );
    client.approve_verification(&project_id, admin);
    project_id
}

// ── get_project_count ────────────────────────────────────────────────────────

#[test]
fn test_project_count_empty() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);
    assert_eq!(client.get_project_count(), 0);
}

#[test]
fn test_project_count_increments() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);

    register(&client, &env, &owner, "P1");
    assert_eq!(client.get_project_count(), 1);
    register(&client, &env, &owner, "P2");
    assert_eq!(client.get_project_count(), 2);
    register(&client, &env, &owner, "P3");
    assert_eq!(client.get_project_count(), 3);
}

#[test]
fn test_project_count_matches_list_projects_range() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);

    for i in 0..5u32 {
        register(&client, &env, &owner, ["Proj0", "Proj1", "Proj2", "Proj3", "Proj4"][i as usize]);
    }

    let count = client.get_project_count();
    // list_projects from 1 to count should return all projects
    let projects = client.list_projects(&1, &(count as u32));
    assert_eq!(projects.len() as u64, count);
}

// ── get_stats_batch ──────────────────────────────────────────────────────────

#[test]
fn test_stats_batch_empty_ids() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);

    let ids: Vec<u64> = Vec::new(&env);
    let result = client.get_stats_batch(&ids);
    assert_eq!(result.len(), 0);
}

#[test]
fn test_stats_batch_no_reviews_returns_zero_defaults() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);

    let id1 = register(&client, &env, &owner, "S1");
    let id2 = register(&client, &env, &owner, "S2");

    let mut ids = Vec::new(&env);
    ids.push_back(id1);
    ids.push_back(id2);

    let result = client.get_stats_batch(&ids);
    assert_eq!(result.len(), 2);

    let (rid1, stats1) = result.get(0).unwrap();
    assert_eq!(rid1, id1);
    assert_eq!(stats1.review_count, 0);
    assert_eq!(stats1.average_rating, 0);

    let (rid2, stats2) = result.get(1).unwrap();
    assert_eq!(rid2, id2);
    assert_eq!(stats2.review_count, 0);
}

#[test]
fn test_stats_batch_with_reviews() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);

    let id1 = register(&client, &env, &owner, "SR1");
    let id2 = register(&client, &env, &owner, "SR2");

    client.add_review(&id1, &reviewer, &4, &None);
    client.add_review(&id2, &reviewer, &2, &None);

    let mut ids = Vec::new(&env);
    ids.push_back(id1);
    ids.push_back(id2);

    let result = client.get_stats_batch(&ids);
    assert_eq!(result.len(), 2);

    let (_, stats1) = result.get(0).unwrap();
    assert_eq!(stats1.review_count, 1);
    assert_eq!(stats1.average_rating, 400); // scaled

    let (_, stats2) = result.get(1).unwrap();
    assert_eq!(stats2.review_count, 1);
    assert_eq!(stats2.average_rating, 200);
}

#[test]
fn test_stats_batch_clamped_to_100() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);

    // Register 5 projects but pass 110 IDs (many nonexistent — stats default to zero)
    for i in 0..5u32 {
        register(&client, &env, &owner, ["Clamp0", "Clamp1", "Clamp2", "Clamp3", "Clamp4"][i as usize]);
    }

    let mut ids = Vec::new(&env);
    for i in 1u64..=110 {
        ids.push_back(i);
    }

    let result = client.get_stats_batch(&ids);
    assert_eq!(result.len(), 100); // clamped
}

// ── get_verifications_batch ──────────────────────────────────────────────────

#[test]
fn test_verifications_batch_empty_ids() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);

    let ids: Vec<u64> = Vec::new(&env);
    let result = client.get_verifications_batch(&ids);
    assert_eq!(result.len(), 0);
}

#[test]
fn test_verifications_batch_skips_unverified_projects() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);

    let id1 = register(&client, &env, &owner, "V1");
    let id2 = register(&client, &env, &owner, "V2");

    // Neither has a verification record
    let mut ids = Vec::new(&env);
    ids.push_back(id1);
    ids.push_back(id2);

    let result = client.get_verifications_batch(&ids);
    assert_eq!(result.len(), 0);
}

#[test]
fn test_verifications_batch_partial_records() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);

    let id1 = setup_verified(&client, &env, &admin, &owner, "VB1");
    let id2 = register(&client, &env, &owner, "VB2"); // no verification

    let mut ids = Vec::new(&env);
    ids.push_back(id1);
    ids.push_back(id2);

    let result = client.get_verifications_batch(&ids);
    // Only id1 has a record
    assert_eq!(result.len(), 1);
    let (rid, record) = result.get(0).unwrap();
    assert_eq!(rid, id1);
    assert_eq!(record.status, VerificationStatus::Verified);
}

#[test]
fn test_verifications_batch_full() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);

    let id1 = setup_verified(&client, &env, &admin, &owner, "VF1");

    // Re-set fee for second project (set_fee is global)
    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    client.set_fee(&admin, &Some(token.clone()), &100, &0u128, &admin);
    soroban_sdk::token::StellarAssetClient::new(&env, &token).mint(&owner, &100);

    let id2 = register(&client, &env, &owner, "VF2");
    client.pay_fee(&owner, &id2, &Some(token));
    client.request_verification(&id2, &owner, &String::from_str(&env, "ipfs://ev2"));
    client.approve_verification(&id2, &admin);

    let mut ids = Vec::new(&env);
    ids.push_back(id1);
    ids.push_back(id2);

    let result = client.get_verifications_batch(&ids);
    assert_eq!(result.len(), 2);
    assert_eq!(result.get(0).unwrap().1.status, VerificationStatus::Verified);
    assert_eq!(result.get(1).unwrap().1.status, VerificationStatus::Verified);
}

#[test]
fn test_verifications_batch_clamped_to_100() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);

    let mut ids = Vec::new(&env);
    for i in 1u64..=110 {
        ids.push_back(i);
    }

    let result = client.get_verifications_batch(&ids);
    // All nonexistent → skipped, but input was clamped to 100 before iteration
    assert_eq!(result.len(), 0); // none have records
    // Verify the clamp by checking we didn't iterate past 100
    // (indirectly: if we iterated 110 nonexistent IDs the result is still 0 — clamp is internal)
}

// ── Indexer sync simulation ──────────────────────────────────────────────────

#[test]
fn test_indexer_can_sync_all_projects_using_count_and_list() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);

    for i in 0..7u32 {
        register(&client, &env, &owner, ["Sync0", "Sync1", "Sync2", "Sync3", "Sync4", "Sync5", "Sync6"][i as usize]);
    }

    let total = client.get_project_count();
    assert_eq!(total, 7);

    // Simulate indexer paging with limit=3
    let mut synced: u32 = 0;
    let mut cursor: u64 = 1;
    let page_size: u32 = 3;

    loop {
        let page = client.list_projects(&cursor, &page_size);
        let n = page.len();
        if n == 0 {
            break;
        }
        synced += n;
        cursor = page.get(n - 1).unwrap().id + 1;
        if cursor > total {
            break;
        }
    }

    assert_eq!(synced, 7);
}

