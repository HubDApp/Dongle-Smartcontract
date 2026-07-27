#![cfg(test)]

use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::ProjectRegistrationParams;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

#[test]
fn test_registration_with_valid_bounty_url() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let mut params = create_test_project_params(&env, &owner, "Test");
    params.bounty_url = Some(String::from_str(&env, "https://example.com/bounty"));
    let res = client.mock_all_auths().register_project(&params);
    assert!(res.is_ok());
}

#[test]
fn test_registration_with_invalid_bounty_url() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let mut params = create_test_project_params(&env, &owner, "Test");
    params.bounty_url = Some(String::from_str(&env, "ftp://bounty.com"));
    let res = client.mock_all_auths().register_project(&params);
    assert_eq!(res.unwrap_err(), ContractError::InvalidBountyUrl);
}

#[test]
fn test_registration_with_valid_bounty_cid() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let mut params = create_test_project_params(&env, &owner, "Test");
    params.bounty_cid = Some(String::from_str(&env, "QmXoypizjW3WknFiJnKLwHCnL72vedxjQkDDP1mXWo6uco"));
    let res = client.mock_all_auths().register_project(&params);
    assert!(res.is_ok());
}

#[test]
fn test_registration_with_invalid_bounty_cid() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let mut params = create_test_project_params(&env, &owner, "Test");
    params.bounty_cid = Some(String::from_str(&env, "invalid-cid"));
    let res = client.mock_all_auths().register_project(&params);
    assert_eq!(res.unwrap_err(), ContractError::InvalidBountyCid);
}

fn create_test_project_params(env: &Env, owner: &Address, name: &str) -> ProjectRegistrationParams {
    let slug = name.to_lowercase().replace(' ', "-");
    ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(env, name),
        slug: String::from_str(env, &slug),
        description: String::from_str(env, "test"),
        category: String::from_str(env, "DeFi"),
        website: None,
        license: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
        bounty_cid: None,
    }
}