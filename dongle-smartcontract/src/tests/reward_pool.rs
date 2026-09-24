use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::DongleContractClient;
use soroban_sdk::{testutils::{Address as _, Ledger}, token, Address, Env, Vec};

fn mint(env: &Env, token_address: &Address, recipient: &Address, amount: i128) {
    token::StellarAssetClient::new(env, token_address).mint(recipient, &amount);
}

#[test]
fn distributes_proportionally_and_records_claims() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let token_address = env.register_stellar_asset_contract_v2(Address::generate(&env)).address();
    let reviewer_a = Address::generate(&env);
    let reviewer_b = Address::generate(&env);
    let project_a = create_test_project(&client, &admin, "RewardA");
    let project_b = create_test_project(&client, &admin, "RewardB");

    client.configure_reward_pool(&admin, &token_address, &100, &2);
    mint(&env, &token_address, &admin, 300);
    client.fund_reward_pool(&admin, &300);
    client.add_review(&project_a, &reviewer_a, &5, &None);
    client.add_review(&project_b, &reviewer_b, &4, &None);
    client.set_review_quality_score(&admin, &project_a, &reviewer_a, &800);
    client.set_review_quality_score(&admin, &project_b, &reviewer_b, &400);

    env.ledger().set_timestamp(100);
    let mut candidates = Vec::new(&env);
    candidates.push_back(reviewer_a.clone());
    candidates.push_back(reviewer_b.clone());
    let period_id = client.finalize_reward_period(&admin, &0, &100, &candidates);
    let period = client.get_reward_period(&period_id).unwrap();
    assert_eq!(period.pool_amount, 300);
    assert_eq!(period.total_quality_score, 1200);
    assert_eq!(period.rewards.len(), 2);
    assert_eq!(period.rewards.get(0).unwrap().amount, 200);
    assert_eq!(period.rewards.get(1).unwrap().amount, 100);

    assert_eq!(client.claim_reward(&reviewer_a, &period_id), 200);
    assert_eq!(client.claim_reward(&reviewer_b, &period_id), 100);
    assert_eq!(token::Client::new(&env, &token_address).balance(&reviewer_a), 200);
    assert_eq!(token::Client::new(&env, &token_address).balance(&reviewer_b), 100);
    assert_eq!(client.try_claim_reward(&reviewer_a, &period_id), Err(Ok(ContractError::RewardAlreadyClaimed.into())));
}

#[test]
fn selects_top_reviewers_when_candidate_set_is_larger_than_pool_limit() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let token_address = env.register_stellar_asset_contract_v2(Address::generate(&env)).address();
    let reviewer_a = Address::generate(&env);
    let reviewer_b = Address::generate(&env);
    let reviewer_c = Address::generate(&env);
    let project_a = create_test_project(&client, &admin, "TopA");
    let project_b = create_test_project(&client, &admin, "TopB");
    let project_c = create_test_project(&client, &admin, "TopC");
    client.configure_reward_pool(&admin, &token_address, &100, &2);
    mint(&env, &token_address, &admin, 100);
    client.fund_reward_pool(&admin, &100);
    client.add_review(&project_a, &reviewer_a, &5, &None);
    client.add_review(&project_b, &reviewer_b, &5, &None);
    client.add_review(&project_c, &reviewer_c, &5, &None);
    client.set_review_quality_score(&admin, &project_a, &reviewer_a, &100);
    client.set_review_quality_score(&admin, &project_b, &reviewer_b, &300);
    client.set_review_quality_score(&admin, &project_c, &reviewer_c, &200);
    env.ledger().set_timestamp(100);
    let candidates = soroban_sdk::vec![&env, reviewer_a, reviewer_b, reviewer_c];
    let period_id = client.finalize_reward_period(&admin, &0, &100, &candidates);
    let period = client.get_reward_period(&period_id).unwrap();
    assert_eq!(period.rewards.len(), 2);
    assert_eq!(period.rewards.get(0).unwrap().quality_score, 300);
    assert_eq!(period.rewards.get(1).unwrap().quality_score, 200);
}