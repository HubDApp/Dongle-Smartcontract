//! End-to-end tests for the complete project registration lifecycle (#952).
//!
//! Covers the full journey from submission to retrieval: successful
//! registration with valid data, duplicate-slug rejection, invalid-metadata
//! rejection, registration combined with fee payment, and post-registration
//! retrieval.

#![cfg(test)]

use crate::errors::ContractError;
use crate::tests::fixtures::setup_contract;
use crate::types::ProjectRegistrationParams;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

fn valid_params(env: &Env, owner: &Address, name: &str, slug: &str) -> ProjectRegistrationParams {
    ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(env, name),
        slug: String::from_str(env, slug),
        description: String::from_str(env, "End-to-end test project"),
        category: String::from_str(env, "DeFi"),
        website: Some(String::from_str(env, "https://example.com")),
        license: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
        repository_url: None,
        language_code: None,
    }
}

#[test]
fn test_e2e_register_project_success() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let params = valid_params(&env, &owner, "E2E Project", "e2e-project");
    let id = client.mock_all_auths().register_project(&params);

    assert_eq!(id, 1);
    let project = client.get_project(&id).expect("project must exist");
    assert_eq!(project.name, String::from_str(&env, "E2E Project"));
    assert_eq!(project.slug, String::from_str(&env, "e2e-project"));
    assert_eq!(project.owner, owner);
}

#[test]
fn test_e2e_register_duplicate_slug_rejected() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    client
        .mock_all_auths()
        .register_project(&valid_params(&env, &owner, "First Project", "same-slug"));

    let other = Address::generate(&env);
    let result = client
        .mock_all_auths()
        .try_register_project(&valid_params(&env, &other, "Second Project", "same-slug"));

    assert_eq!(
        result,
        Err(Ok(ContractError::ProjectAlreadyExists.into()))
    );
}

#[test]
fn test_e2e_register_invalid_metadata_rejected() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    // Empty name must be rejected.
    let mut bad = valid_params(&env, &owner, "Bad Project", "bad-project");
    bad.name = String::from_str(&env, "");
    let result = client.mock_all_auths().try_register_project(&bad);
    assert!(result.is_err(), "empty name must be rejected");

    // Non-https website must be rejected.
    let mut bad = valid_params(&env, &owner, "Bad Project", "bad-project-2");
    bad.website = Some(String::from_str(&env, "http://insecure.example.com"));
    let result = client.mock_all_auths().try_register_project(&bad);
    assert!(result.is_err(), "non-https website must be rejected");
}

#[test]
fn test_e2e_register_with_fee_payment() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract_v2(token_admin).address();
    let treasury = Address::generate(&env);
    client.mock_all_auths().set_fee(
        &admin,
        &Some(token.clone()),
        &100u128,
        &0u128,
        &treasury,
    );

    let owner = Address::generate(&env);
    soroban_sdk::token::StellarAssetClient::new(&env, &token).mint(&owner, &10_000);

    let id = client
        .mock_all_auths()
        .register_project(&valid_params(&env, &owner, "Paid Project", "paid-project"));
    assert_eq!(id, 1);

    client
        .mock_all_auths()
        .pay_fee(&owner, &id, &Some(token.clone()));
    assert!(client.is_fee_paid(&id));

    // Project remains retrievable after the fee flow.
    let project = client.get_project(&id).expect("project must exist");
    assert_eq!(project.owner, owner);
}

#[test]
fn test_e2e_project_retrievable_after_registration() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let params = valid_params(&env, &owner, "Retrieval Project", "retrieval-project");
    let id = client.mock_all_auths().register_project(&params);

    let project = client.get_project(&id).expect("project must be retrievable");
    assert_eq!(project.id, id);
    assert_eq!(project.description, params.description);
    assert_eq!(project.category, params.category);
    assert_eq!(project.website, params.website);
}
