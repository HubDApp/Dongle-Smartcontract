use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::ProjectUpdateParams;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

#[test]
fn test_register_project_with_valid_bounty_url() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    // Valid https URL
    let params = crate::types::ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "Test"),
        slug: String::from_str(&env, "test"),
        description: String::from_str(&env, "A test"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        license: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: Some(String::from_str(&env, "https://example.com/bounty")),
    };

    let project_id = client.mock_all_auths().register_project(&params);
    let proj = client.get_project(&project_id).unwrap();
    assert_eq!(
        proj.bounty_url.unwrap(),
        String::from_str(&env, "https://example.com/bounty")
    );
}

#[test]
fn test_register_project_with_invalid_bounty_url() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    // Invalid URL (no scheme)
    let params = crate::types::ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "Test"),
        slug: String::from_str(&env, "test2"),
        description: String::from_str(&env, "A test"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        license: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: Some(String::from_str(&env, "not-a-url")),
    };

    let result = client.try_register_project(&params);
    assert_eq!(result, Err(Ok(ContractError::InvalidBountyUrl)));
}

#[test]
fn test_update_project_with_valid_bounty_url() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "UpdateBounty");

    // Update with valid URL
    let params = ProjectUpdateParams {
        name: None,
        description: None,
        category: None,
        website: None,
        license: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: Some(String::from_str(&env, "https://newbounty.com")),
        bounty_url_clear: false,
    };

    client
        .mock_all_auths()
        .update_project(&project_id, &owner, &params)
        .unwrap();

    let proj = client.get_project(&project_id).unwrap();
    assert_eq!(
        proj.bounty_url.unwrap(),
        String::from_str(&env, "https://newbounty.com")
    );
}

#[test]
fn test_update_project_with_invalid_bounty_url() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "InvalidUpdate");

    // Update with invalid URL
    let params = ProjectUpdateParams {
        name: None,
        description: None,
        category: None,
        website: None,
        license: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: Some(String::from_str(&env, "ftp://bounty")),
        bounty_url_clear: false,
    };

    let result = client
        .mock_all_auths()
        .try_update_project(&project_id, &owner, &params);
    assert_eq!(result, Err(Ok(ContractError::InvalidBountyUrl)));
}

#[test]
fn test_register_with_cid_as_bounty_url() {
    // If CIDs are allowed (starts with Qm or bafy...), this test would pass
    // For simplicity, we treat any non-http string as invalid in this implementation
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    // A CID string like "Qm..." would fail as it doesn't start with http
    let params = crate::types::ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "CIDTest"),
        slug: String::from_str(&env, "cidtest"),
        description: String::from_str(&env, "CID test"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        license: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: Some(String::from_str(&env, "QmXoypizjW3WknFiJnKLwHCnL72vedxjQkDDP1mXWo6uco")),
    };

    let result = client.try_register_project(&params);
    assert_eq!(result, Err(Ok(ContractError::InvalidBountyUrl)));
}
