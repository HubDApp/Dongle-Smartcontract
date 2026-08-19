use crate::constants::MAX_TTL_BATCH_SIZE;
use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use soroban_sdk::{testutils::Address as _, Address, Env, Vec};

#[test]
fn batch_project_ttl_refreshes_existing_and_skips_missing() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let first = create_test_project(&client, &owner, "BulkTTLProjectA");
    let second = create_test_project(&client, &owner, "BulkTTLProjectB");

    let mut ids = Vec::new(&env);
    ids.push_back(first);
    ids.push_back(999_999);
    ids.push_back(second);

    let refreshed = client.extend_projects_ttl(&ids);

    assert_eq!(refreshed, 2);
}

#[test]
fn batch_review_ttl_refreshes_existing_and_skips_missing() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let missing_reviewer = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "BulkTTLReviews");

    client
        .mock_all_auths()
        .add_review(&project_id, &reviewer, &5u32, &None);

    let mut review_ids = Vec::new(&env);
    review_ids.push_back((project_id, reviewer.clone()));
    review_ids.push_back((project_id, missing_reviewer));

    let refreshed = client.extend_reviews_ttl(&review_ids);

    assert_eq!(refreshed, 1);
}

#[test]
fn batch_project_ttl_rejects_oversized_batches() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let mut ids = Vec::new(&env);

    for i in 0..=MAX_TTL_BATCH_SIZE {
        ids.push_back(i as u64);
    }

    let result = client.try_extend_projects_ttl(&ids);

    assert_eq!(result, Err(Ok(ContractError::InvalidInput)));
}

#[test]
fn batch_review_ttl_rejects_oversized_batches() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let reviewer = Address::generate(&env);
    let mut review_ids = Vec::new(&env);

    for i in 0..=MAX_TTL_BATCH_SIZE {
        review_ids.push_back((i as u64, reviewer.clone()));
    }

    let result = client.try_extend_reviews_ttl(&review_ids);

    assert_eq!(result, Err(Ok(ContractError::InvalidInput)));
}
