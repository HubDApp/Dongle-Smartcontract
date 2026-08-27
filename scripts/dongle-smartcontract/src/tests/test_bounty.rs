#![cfg(test)]

use soroban_sdk::{testutils::Address as _, Address, Env, String};

use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::ProjectRegistrationParams;

#[test]
fn test_register_project_with_valid_bounty_url() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let mut params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "Test"),
        slug: String::from_str(&env, "test"),
        description: String::from_str(&env, "desc"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        license: None,
        bounty_url: Some(String::from_str(&env, "https://bounty.example.com")),
        bounty_cid: None,
    };
    client.mock_all_auths().register_project(&params);
    // No panic means success
}

#[test]
fn test_register_project_with_invalid_bounty_url() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let mut params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "Test"),
        slug: String::from_str(&env, "test"),
        description: String::from_str(&env, "desc"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        license: None,
        bounty_url: Some(String::from_str(&env, "ftp://invalid")),
        bounty_cid: None,
    };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.mock_all_auths().register_project(&params);
    }));
    assert!(result.is_err()); // Should panic with InvalidBountyUrl
}

#[test]
fn test_register_project_with_valid_bounty_cid() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let mut params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "Test"),
        slug: String::from_str(&env, "test"),
        description: String::from_str(&env, "desc"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        license: None,
        bounty_url: None,
        bounty_cid: Some(String::from_str(&env, "QmXoypizjW3WknFiJnKLwHCnL72vedxjQkDDP1mXWo6uco")),
    };
    client.mock_all_auths().register_project(&params);
}

#[test]
fn test_register_project_with_invalid_bounty_cid() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let mut params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "Test"),
        slug: String::from_str(&env, "test"),
        description: String::from_str(&env, "desc"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        license: None,
        bounty_url: None,
        bounty_cid: Some(String::from_str(&env, "invalid-cid")),
    };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.mock_all_auths().register_project(&params);
    }));
    assert!(result.is_err());
}
