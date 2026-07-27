#![cfg(test)]

use crate::errors::ContractError;
use crate::tests::fixtures::setup_contract;
use crate::types::ProjectRegistrationParams;
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};

fn register_project_with_bounty(
    env: &Env,
    client: &crate::DongleContractClient<'_>,
    owner: &Address,
    name: &str,
    bounty_url: Option<String>,
    bounty_cid: Option<String>,
) -> Result<u64, ContractError> {
    let slug = name.to_lowercase().replace(' ', "-");
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(env, name),
        slug: String::from_str(env, &slug),
        description: String::from_str(env, "A test project for bounty metadata"),
        category: String::from_str(env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: bounty_url.map(|s| String::from_str(env, &s)),
        bounty_cid: bounty_cid.map(|s| String::from_str(env, &s)),
    };
    let env_clone = env.clone();
    client.mock_all_auths().try_register_project(&params)
}

#[test]
fn test_valid_bounty_url() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let result = register_project_with_bounty(
        &env,
        &client,
        &owner,
        "Project URL",
        Some("https://example.com/bounty".to_string()),
        None,
    );
    assert!(result.is_ok());
    let project = client.get_project(&result.unwrap()).unwrap();
    assert_eq!(
        project.bounty_url.unwrap(),
        String::from_str(&env, "https://example.com/bounty")
    );
}

#[test]
fn test_invalid_bounty_url_no_scheme() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let result = register_project_with_bounty(
        &env,
        &client,
        &owner,
        "Invalid URL",
        Some("ftp://example.com/bounty".to_string()),
        None,
    );
    assert!(result.is_err());
    assert_eq!(result.err().unwrap(), ContractError::InvalidInput);
}

#[test]
fn test_invalid_bounty_url_short() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let result = register_project_with_bounty(
        &env,
        &client,
        &owner,
        "Short URL",
        Some("http://a".to_string()),
        None,
    );
    assert!(result.is_err());
    assert_eq!(result.err().unwrap(), ContractError::InvalidInput);
}

#[test]
fn test_valid_bounty_cid_v0() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let valid_cid = "QmXoypizjW3WknFiJnKLwHCnL72vedxjQkDDP1mXWo6uco";  // example CIDv0
    let result = register_project_with_bounty(
        &env,
        &client,
        &owner,
        "Project CIDv0",
        None,
        Some(valid_cid.to_string()),
    );
    assert!(result.is_ok());
    let project = client.get_project(&result.unwrap()).unwrap();
    assert_eq!(
        project.bounty_cid.unwrap(),
        String::from_str(&env, valid_cid)
    );
}

#[test]
fn test_valid_bounty_cid_v1() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let valid_cid = "bafybeigdzeq3z7q3kz3z3z3z3z3z3z3z3z3z3z3z3z3z3z3z3z3z3z3"; // shortened for test
    let result = register_project_with_bounty(
        &env,
        &client,
        &owner,
        "Project CIDv1",
        None,
        Some(valid_cid.to_string()),
    );
    assert!(result.is_ok());
}

#[test]
fn test_invalid_bounty_cid_wrong_prefix() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let invalid_cid = "XmXoypizjW3WknFiJnKLwHCnL72vedxjQkDDP1mXWo6uco";
    let result = register_project_with_bounty(
        &env,
        &client,
        &owner,
        "Invalid CID",
        None,
        Some(invalid_cid.to_string()),
    );
    assert!(result.is_err());
    assert_eq!(result.err().unwrap(), ContractError::InvalidInput);
}

#[test]
fn test_invalid_bounty_cid_short_v1() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let invalid_cid = "bafy";
    let result = register_project_with_bounty(
        &env,
        &client,
        &owner,
        "Short CIDv1",
        None,
        Some(invalid_cid.to_string()),
    );
    assert!(result.is_err());
    assert_eq!(result.err().unwrap(), ContractError::InvalidInput);
}

#[test]
fn test_bounty_fields_missing() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    // Register without any bounty fields
    let name = "No Bounty";
    let slug = name.to_lowercase().replace(' ', "-");
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, name),
        slug: String::from_str(&env, &slug),
        description: String::from_str(&env, "No bounty project"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
        bounty_cid: None,
    };
    let result = client.mock_all_auths().try_register_project(&params);
    assert!(result.is_ok());
    let project = client.get_project(&result.unwrap()).unwrap();
    assert!(project.bounty_url.is_none());
    assert!(project.bounty_cid.is_none());
}
