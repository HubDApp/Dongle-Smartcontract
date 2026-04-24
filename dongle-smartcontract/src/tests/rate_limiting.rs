//! Tests for rate limiting functionality

use crate::tests::fixtures::{setup_contract, generate_test_users};
use soroban_sdk::{testutils::Ledger, Address, Env, String};

#[test]
fn test_review_action_rate_limiting() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let user = Address::generate(&env);

    // Register a project first
    let project_params = crate::types::ProjectRegistrationParams {
        owner: user.clone(),
        name: String::from_str(&env, "Test Project"),
        description: String::from_str(&env, "Test Description"),
        category: String::from_str(&env, "Test"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };

    let project_id = client.register_project(&project_params);

    // First review should succeed
    let result = client.add_review(&project_id, &user, &5u32, &None);
    assert!(result.is_ok());

    // Immediate second review should fail due to rate limit
    let result = client.add_review(&project_id, &user, &4u32, &None);
    assert_eq!(result, Err(crate::errors::ContractError::RateLimitExceeded));

    // Check cooldown remaining
    let remaining = client.get_review_action_cooldown_remaining(&user);
    assert!(remaining > 0 && remaining <= 60); // Should be close to 60 seconds

    // Advance time by 61 seconds
    env.ledger().set_timestamp(env.ledger().timestamp() + 61);

    // Now the review should succeed
    let result = client.add_review(&project_id, &user, &4u32, &None);
    assert!(result.is_ok());

    // Cooldown should be reset
    let remaining = client.get_review_action_cooldown_remaining(&user);
    assert_eq!(remaining, 0);
}

#[test]
fn test_update_review_rate_limiting() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let user = Address::generate(&env);

    // Register a project
    let project_params = crate::types::ProjectRegistrationParams {
        owner: user.clone(),
        name: String::from_str(&env, "Test Project"),
        description: String::from_str(&env, "Test Description"),
        category: String::from_str(&env, "Test"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };

    let project_id = client.register_project(&project_params);

    // Add initial review
    client.add_review(&project_id, &user, &5u32, &None).unwrap();

    // Advance time past cooldown
    env.ledger().set_timestamp(env.ledger().timestamp() + 61);

    // Update should succeed
    let result = client.update_review(&project_id, &user, &4u32, &None);
    assert!(result.is_ok());

    // Immediate update should fail
    let result = client.update_review(&project_id, &user, &3u32, &None);
    assert_eq!(result, Err(crate::errors::ContractError::RateLimitExceeded));
}

#[test]
fn test_delete_review_rate_limiting() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let user = Address::generate(&env);

    // Register a project
    let project_params = crate::types::ProjectRegistrationParams {
        owner: user.clone(),
        name: String::from_str(&env, "Test Project"),
        description: String::from_str(&env, "Test Description"),
        category: String::from_str(&env, "Test"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };

    let project_id = client.register_project(&project_params);

    // Add review
    client.add_review(&project_id, &user, &5u32, &None).unwrap();

    // Advance time past cooldown
    env.ledger().set_timestamp(env.ledger().timestamp() + 61);

    // Delete should succeed
    let result = client.delete_review(&project_id, &user);
    assert!(result.is_ok());
}

#[test]
fn test_verification_request_rate_limiting() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let user = Address::generate(&env);

    // Set up fee config
    client.set_fee(&admin, &None, &1000u128, &admin).unwrap();

    // Register a project
    let project_params = crate::types::ProjectRegistrationParams {
        owner: user.clone(),
        name: String::from_str(&env, "Test Project"),
        description: String::from_str(&env, "Test Description"),
        category: String::from_str(&env, "Test"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };

    let project_id = client.register_project(&project_params);

    // Pay fee for verification
    client.pay_fee(&user, &project_id, &None).unwrap();

    // First verification request should succeed
    let result = client.request_verification(&project_id, &user, &String::from_str(&env, "evidence_cid"));
    assert!(result.is_ok());

    // Immediate second request should fail
    let result = client.request_verification(&project_id, &user, &String::from_str(&env, "evidence_cid2"));
    assert_eq!(result, Err(crate::errors::ContractError::RateLimitExceeded));

    // Check cooldown remaining
    let remaining = client.get_verification_request_cooldown_remaining(&user);
    assert!(remaining > 0 && remaining <= 300); // Should be close to 300 seconds

    // Advance time by 301 seconds
    env.ledger().set_timestamp(env.ledger().timestamp() + 301);

    // Register another project for second verification request
    let project_params2 = crate::types::ProjectRegistrationParams {
        owner: user.clone(),
        name: String::from_str(&env, "Test Project 2"),
        description: String::from_str(&env, "Test Description 2"),
        category: String::from_str(&env, "Test"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };

    let project_id2 = client.register_project(&project_params2);

    // Pay fee for second project
    client.pay_fee(&user, &project_id2, &None).unwrap();

    // Now verification request should succeed
    let result = client.request_verification(&project_id2, &user, &String::from_str(&env, "evidence_cid3"));
    assert!(result.is_ok());
}

#[test]
fn test_different_users_not_affected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let users = generate_test_users(&env, 2);
    let user1 = users.get(0).unwrap();
    let user2 = users.get(1).unwrap();

    // Register projects for both users
    let project_params1 = crate::types::ProjectRegistrationParams {
        owner: user1.clone(),
        name: String::from_str(&env, "Test Project 1"),
        description: String::from_str(&env, "Test Description"),
        category: String::from_str(&env, "Test"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };

    let project_params2 = crate::types::ProjectRegistrationParams {
        owner: user2.clone(),
        name: String::from_str(&env, "Test Project 2"),
        description: String::from_str(&env, "Test Description"),
        category: String::from_str(&env, "Test"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };

    let project_id1 = client.register_project(&project_params1);
    let project_id2 = client.register_project(&project_params2);

    // Both users can add reviews simultaneously
    let result1 = client.add_review(&project_id1, &user1, &5u32, &None);
    assert!(result1.is_ok());

    let result2 = client.add_review(&project_id2, &user2, &5u32, &None);
    assert!(result2.is_ok());

    // user1 is rate limited, but user2 can still act
    let result1_limited = client.add_review(&project_id1, &user1, &4u32, &None);
    assert_eq!(result1_limited, Err(crate::errors::ContractError::RateLimitExceeded));

    let result2_ok = client.add_review(&project_id2, &user2, &4u32, &None);
    assert!(result2_ok.is_ok());
}