//! Comprehensive tests for verification lifecycle and state machine enforcement

use crate::errors::ContractError;
use crate::types::{ProjectRegistrationParams, VerificationStatus};
use crate::DongleContract;
use crate::DongleContractClient;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

fn setup(env: &Env) -> (DongleContractClient<'_>, Address, Address) {
    let contract_id = env.register(DongleContract, ());
    let client = DongleContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.initialize(&admin);
    (client, admin, Address::generate(env))
}

fn setup_project_with_fee(
    client: &DongleContractClient<'_>,
    env: &Env,
    admin: &Address,
    owner: &Address,
    project_name: &str,
) -> u64 {
    let slug = project_name.to_lowercase().replace(' ', "-");
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(env, project_name),
        slug: String::from_str(env, &slug),
        description: String::from_str(env, "Test project description"),
        category: String::from_str(env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
    };
    let project_id = client.register_project(&params);

    // Set up fee configuration
    let token_admin = Address::generate(env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    client.set_fee(admin, &Some(token_address.clone()), &100, admin);

    // Mint tokens and pay fee
    let token_client = soroban_sdk::token::StellarAssetClient::new(env, &token_address);
    token_client.mint(owner, &1000);
    client.pay_fee(owner, &project_id, &Some(token_address));

    project_id
}

// --- Basic Verification Lifecycle Tests ---

#[test]
fn test_verification_lifecycle() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, owner) = setup(&env);

    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "Project X"),
        slug: String::from_str(&env, "project-x"),
        description: String::from_str(&env, "Description... Description... Description..."),
        category: String::from_str(&env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
    };
    let project_id = client.register_project(&params);

    // 1. Initially unverified
    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.verification_status, VerificationStatus::Unverified);

    // 2. Set fee (using admin)
    client.set_fee(&admin, &None, &100, &admin);

    // 3. Pay fee (using owner)
    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    client.set_fee(&admin, &Some(token_address.clone()), &100, &admin);

    // Mock token balance for owner
    let token_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_address);
    token_client.mint(&owner, &1000);

    client.pay_fee(&owner, &project_id, &Some(token_address.clone()));

    // 4. Request verification
    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "ipfs://evidence"),
    );

    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.verification_status, VerificationStatus::Pending);

    // 5. Approve verification (using admin)
    client.approve_verification(&project_id, &admin);

    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.verification_status, VerificationStatus::Verified);
}

#[test]
fn test_reject_verification() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, owner) = setup(&env);

    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "Project Y"),
        slug: String::from_str(&env, "project-y"),
        description: String::from_str(&env, "Description... Description... Description..."),
        category: String::from_str(&env, "NFT"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
    };
    let project_id = client.register_project(&params);

    // Set fee and pay
    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    let token_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_address);
    token_client.mint(&owner, &100);
    client.set_fee(&admin, &Some(token_address.clone()), &100, &admin);
    client.pay_fee(&owner, &project_id, &Some(token_address));

    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "ipfs://evidence"),
    );

    // Reject
    client.reject_verification(&project_id, &admin);

    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.verification_status, VerificationStatus::Rejected);
}

// --- State Machine Transition Tests ---

#[test]
fn test_valid_state_transitions() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, owner) = setup(&env);

    // Test 1: Unverified -> Pending (verification request)
    let project_id = setup_project_with_fee(&client, &env, &admin, &owner, "Project 1");

    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.verification_status, VerificationStatus::Unverified);

    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "ipfs://evidence1"),
    );

    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.verification_status, VerificationStatus::Pending);

    // Test 2: Pending -> Verified (admin approval)
    client.approve_verification(&project_id, &admin);
    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.verification_status, VerificationStatus::Verified);

    // Test 3: Rejected -> Pending (re-request verification)
    let project_id2 = setup_project_with_fee(&client, &env, &admin, &owner, "Project 2");

    client.request_verification(
        &project_id2,
        &owner,
        &String::from_str(&env, "ipfs://evidence2"),
    );
    client.reject_verification(&project_id2, &admin);

    let project = client.get_project(&project_id2).unwrap();
    assert_eq!(project.verification_status, VerificationStatus::Rejected);

    // Re-request verification after rejection
    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    let token_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_address);
    token_client.mint(&owner, &1000);
    client.set_fee(&admin, &Some(token_address.clone()), &100, &admin);
    client.pay_fee(&owner, &project_id2, &Some(token_address));

    client.request_verification(
        &project_id2,
        &owner,
        &String::from_str(&env, "ipfs://evidence2_updated"),
    );

    let project = client.get_project(&project_id2).unwrap();
    assert_eq!(project.verification_status, VerificationStatus::Pending);

    // Test 4: Pending -> Rejected (admin rejection)
    client.reject_verification(&project_id2, &admin);
    let project = client.get_project(&project_id2).unwrap();
    assert_eq!(project.verification_status, VerificationStatus::Rejected);
}

#[test]
fn test_invalid_transitions_from_unverified() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, owner) = setup(&env);

    let project_id = setup_project_with_fee(&client, &env, &admin, &owner, "Project Invalid 1");

    // Cannot approve directly from Unverified - no verification record exists
    let result = client.try_approve_verification(&project_id, &admin);
    assert_eq!(result, Err(Ok(ContractError::VerificationNotFound)));

    // Cannot reject directly from Unverified - no verification record exists
    let result = client.try_reject_verification(&project_id, &admin);
    assert_eq!(result, Err(Ok(ContractError::VerificationNotFound)));
}

#[test]
fn test_invalid_transitions_from_pending() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, owner) = setup(&env);

    let project_id = setup_project_with_fee(&client, &env, &admin, &owner, "Project Invalid 2");

    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "ipfs://evidence"),
    );

    // Cannot request verification again while already pending
    let result = client.try_request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "ipfs://evidence2"),
    );
    assert_eq!(result, Err(Ok(ContractError::InvalidStatus)));
}

#[test]
fn test_invalid_transitions_from_verified() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, owner) = setup(&env);

    let project_id = setup_project_with_fee(&client, &env, &admin, &owner, "Project Invalid 3");

    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "ipfs://evidence"),
    );
    client.approve_verification(&project_id, &admin);

    // Cannot request verification for already verified project
    let result = client.try_request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "ipfs://evidence2"),
    );
    assert_eq!(result, Err(Ok(ContractError::InvalidStatus)));

    // Cannot approve already verified project
    let result = client.try_approve_verification(&project_id, &admin);
    assert_eq!(result, Err(Ok(ContractError::InvalidStatus)));

    // Cannot reject already verified project
    let result = client.try_reject_verification(&project_id, &admin);
    assert_eq!(result, Err(Ok(ContractError::InvalidStatus)));
}

#[test]
fn test_invalid_transitions_from_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, owner) = setup(&env);

    let project_id = setup_project_with_fee(&client, &env, &admin, &owner, "Project Invalid 4");

    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "ipfs://evidence"),
    );
    client.reject_verification(&project_id, &admin);

    // Cannot approve directly from rejected state
    let result = client.try_approve_verification(&project_id, &admin);
    assert_eq!(result, Err(Ok(ContractError::InvalidStatus)));

    // Cannot reject again from rejected state
    let result = client.try_reject_verification(&project_id, &admin);
    assert_eq!(result, Err(Ok(ContractError::InvalidStatus)));
}

#[test]
fn test_multiple_verification_cycles() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, owner) = setup(&env);

    let project_id = setup_project_with_fee(&client, &env, &admin, &owner, "Project Cycle");

    // First cycle: Request -> Reject -> Request -> Approve
    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "ipfs://evidence1"),
    );
    assert_eq!(
        client.get_project(&project_id).unwrap().verification_status,
        VerificationStatus::Pending
    );

    client.reject_verification(&project_id, &admin);
    assert_eq!(
        client.get_project(&project_id).unwrap().verification_status,
        VerificationStatus::Rejected
    );

    // Pay fee again for re-submission
    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    let token_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_address);
    token_client.mint(&owner, &1000);
    client.set_fee(&admin, &Some(token_address.clone()), &100, &admin);
    client.pay_fee(&owner, &project_id, &Some(token_address));

    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "ipfs://evidence2"),
    );
    assert_eq!(
        client.get_project(&project_id).unwrap().verification_status,
        VerificationStatus::Pending
    );

    client.approve_verification(&project_id, &admin);
    assert_eq!(
        client.get_project(&project_id).unwrap().verification_status,
        VerificationStatus::Verified
    );

    // After verification, no more transitions should be possible
    let token_admin2 = Address::generate(&env);
    let token_address2 = env
        .register_stellar_asset_contract_v2(token_admin2)
        .address();
    let token_client2 = soroban_sdk::token::StellarAssetClient::new(&env, &token_address2);
    token_client2.mint(&owner, &1000);
    client.set_fee(&admin, &Some(token_address2.clone()), &100, &admin);
    client.pay_fee(&owner, &project_id, &Some(token_address2));

    let result = client.try_request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "ipfs://evidence3"),
    );
    assert_eq!(result, Err(Ok(ContractError::InvalidStatus)));
}

#[test]
fn test_idempotent_transitions() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, owner) = setup(&env);

    let project_id = setup_project_with_fee(&client, &env, &admin, &owner, "Project Idempotent");

    // Initial state should be Unverified
    assert_eq!(
        client.get_project(&project_id).unwrap().verification_status,
        VerificationStatus::Unverified
    );

    // Request verification
    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "ipfs://evidence"),
    );

    // Approve verification
    client.approve_verification(&project_id, &admin);

    // Try to approve again - should fail (already Verified)
    let result = client.try_approve_verification(&project_id, &admin);
    assert_eq!(result, Err(Ok(ContractError::InvalidStatus)));
}

#[test]
fn test_state_machine_with_different_admins() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, owner) = setup(&env);

    // Add another admin
    let admin2 = Address::generate(&env);
    client.add_admin(&admin, &admin2);

    let project_id = setup_project_with_fee(&client, &env, &admin, &owner, "Project Multi Admin");

    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "ipfs://evidence"),
    );

    // Different admin should be able to approve
    client.approve_verification(&project_id, &admin2);
    assert_eq!(
        client.get_project(&project_id).unwrap().verification_status,
        VerificationStatus::Verified
    );
}

// --- Revocation Tests ---

#[test]
fn test_revoke_verification_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, owner) = setup(&env);

    let project_id = setup_project_with_fee(&client, &env, &admin, &owner, "Project Revoke");

    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "ipfs://evidence"),
    );
    client.approve_verification(&project_id, &admin);

    assert_eq!(
        client.get_project(&project_id).unwrap().verification_status,
        VerificationStatus::Verified
    );

    client.revoke_verification(
        &project_id,
        &admin,
        &String::from_str(&env, "Project became malicious"),
    );

    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.verification_status, VerificationStatus::Unverified);

    let record = client.get_verification(&project_id);
    assert_eq!(record.status, VerificationStatus::Unverified);
    assert_eq!(
        record.revoke_reason,
        Some(String::from_str(&env, "Project became malicious"))
    );
}

#[test]
fn test_revoke_non_verified_project_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, owner) = setup(&env);

    let project_id = setup_project_with_fee(&client, &env, &admin, &owner, "Project Not Verified");

    // Cannot revoke an unverified project
    let result =
        client.try_revoke_verification(&project_id, &admin, &String::from_str(&env, "reason"));
    assert_eq!(result, Err(Ok(ContractError::InvalidStatus)));

    // Cannot revoke a pending project
    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "ipfs://evidence"),
    );
    let result =
        client.try_revoke_verification(&project_id, &admin, &String::from_str(&env, "reason"));
    assert_eq!(result, Err(Ok(ContractError::InvalidStatus)));
}

#[test]
fn test_revoke_by_non_admin_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, owner) = setup(&env);

    let project_id = setup_project_with_fee(&client, &env, &admin, &owner, "Project Non Admin");

    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "ipfs://evidence"),
    );
    client.approve_verification(&project_id, &admin);

    let non_admin = Address::generate(&env);
    let result =
        client.try_revoke_verification(&project_id, &non_admin, &String::from_str(&env, "reason"));
    assert_eq!(result, Err(Ok(ContractError::AdminOnly)));
}

#[test]
fn test_revoke_nonexistent_project_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, _owner) = setup(&env);

    let result = client.try_revoke_verification(&9999, &admin, &String::from_str(&env, "reason"));
    assert_eq!(result, Err(Ok(ContractError::ProjectNotFound)));
}

#[test]
fn test_revoked_project_can_re_request_verification() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, owner) = setup(&env);

    let project_id = setup_project_with_fee(&client, &env, &admin, &owner, "Project Re-request");

    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "ipfs://evidence"),
    );
    client.approve_verification(&project_id, &admin);
    client.revoke_verification(
        &project_id,
        &admin,
        &String::from_str(&env, "Stale project"),
    );

    assert_eq!(
        client.get_project(&project_id).unwrap().verification_status,
        VerificationStatus::Unverified
    );

    // Pay fee again and re-request
    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    let token_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_address);
    token_client.mint(&owner, &1000);
    client.set_fee(&admin, &Some(token_address.clone()), &100, &admin);
    client.pay_fee(&owner, &project_id, &Some(token_address));

    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "ipfs://new-evidence"),
    );

    assert_eq!(
        client.get_project(&project_id).unwrap().verification_status,
        VerificationStatus::Pending
    );
}

#[test]
fn test_verification_history_ordering() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, owner) = setup(&env);

    let project_id = setup_project_with_fee(&client, &env, &admin, &owner, "Project Order Test");

    // Initially, verification ID is None
    assert_eq!(client.get_project(&project_id).unwrap().current_verification_id, None);

    // Request #1 -> Reject
    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "QmYwAPJzv5CZsnAzt8auVZRnG8X1sC3yRyvCb4s46HoPa1"),
    );
    assert_eq!(client.get_project(&project_id).unwrap().current_verification_id, Some(1));
    client.reject_verification(&project_id, &admin);
    assert_eq!(client.get_project(&project_id).unwrap().current_verification_id, Some(1));

    // Pay fee again for second request
    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    let token_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_address);
    token_client.mint(&owner, &1000);
    client.set_fee(&admin, &Some(token_address.clone()), &100, &admin);
    client.pay_fee(&owner, &project_id, &Some(token_address));

    // Request #2 -> Approve
    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "QmYwAPJzv5CZsnAzt8auVZRnG8X1sC3yRyvCb4s46HoPa2"),
    );
    assert_eq!(client.get_project(&project_id).unwrap().current_verification_id, Some(2));
    client.approve_verification(&project_id, &admin);
    assert_eq!(client.get_project(&project_id).unwrap().current_verification_id, Some(2));

    // Revoke
    client.revoke_verification(
        &project_id,
        &admin,
        &String::from_str(&env, "Revoke for re-request"),
    );
    assert_eq!(client.get_project(&project_id).unwrap().current_verification_id, Some(2));

    // Pay fee again for third request
    let token_admin2 = Address::generate(&env);
    let token_address2 = env
        .register_stellar_asset_contract_v2(token_admin2)
        .address();
    let token_client2 = soroban_sdk::token::StellarAssetClient::new(&env, &token_address2);
    token_client2.mint(&owner, &1000);
    client.set_fee(&admin, &Some(token_address2.clone()), &100, &admin);
    client.pay_fee(&owner, &project_id, &Some(token_address2));

    // Request #3 -> Pending
    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "QmYwAPJzv5CZsnAzt8auVZRnG8X1sC3yRyvCb4s46HoPa3"),
    );
    assert_eq!(client.get_project(&project_id).unwrap().current_verification_id, Some(3));

    // Retrieve history
    let history = client.get_verification_history(&project_id);
    assert_eq!(history.len(), 3);

    // Assert request IDs
    let h0 = history.get(0).unwrap();
    let h1 = history.get(1).unwrap();
    let h2 = history.get(2).unwrap();

    assert_eq!(h0.request_id, 1);
    assert_eq!(h0.status, VerificationStatus::Rejected);
    assert_eq!(
        h0.evidence_cid,
        String::from_str(&env, "QmYwAPJzv5CZsnAzt8auVZRnG8X1sC3yRyvCb4s46HoPa1")
    );

    assert_eq!(h1.request_id, 2);
    assert_eq!(h1.status, VerificationStatus::Unverified);
    assert_eq!(
        h1.evidence_cid,
        String::from_str(&env, "QmYwAPJzv5CZsnAzt8auVZRnG8X1sC3yRyvCb4s46HoPa2")
    );
    assert_eq!(
        h1.revoke_reason,
        Some(String::from_str(&env, "Revoke for re-request"))
    );

    assert_eq!(h2.request_id, 3);
    assert_eq!(h2.status, VerificationStatus::Pending);
    assert_eq!(
        h2.evidence_cid,
        String::from_str(&env, "QmYwAPJzv5CZsnAzt8auVZRnG8X1sC3yRyvCb4s46HoPa3")
    );

    // Assert current verification lookup gets the latest record
    let current = client.get_verification(&project_id);
    assert_eq!(current.request_id, 3);
    assert_eq!(current.status, VerificationStatus::Pending);
}

#[test]
fn test_unique_request_ids_across_projects() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, owner) = setup(&env);

    let project_a = setup_project_with_fee(&client, &env, &admin, &owner, "Project A");
    let project_b = setup_project_with_fee(&client, &env, &admin, &owner, "Project B");

    // 1. Request verification for Project A -> request_id 1
    client.request_verification(
        &project_a,
        &owner,
        &String::from_str(&env, "QmYwAPJzv5CZsnAzt8auVZRnG8X1sC3yRyvCb4s46HoPaA1"),
    );

    // 2. Request verification for Project B -> request_id 2
    client.request_verification(
        &project_b,
        &owner,
        &String::from_str(&env, "QmYwAPJzv5CZsnAzt8auVZRnG8X1sC3yRyvCb4s46HoPaB1"),
    );

    // 3. Pay fee again and request verification for Project A -> request_id 3
    client.reject_verification(&project_a, &admin);
    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    let token_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_address);
    token_client.mint(&owner, &1000);
    client.set_fee(&admin, &Some(token_address.clone()), &100, &admin);
    client.pay_fee(&owner, &project_a, &Some(token_address));

    client.request_verification(
        &project_a,
        &owner,
        &String::from_str(&env, "QmYwAPJzv5CZsnAzt8auVZRnG8X1sC3yRyvCb4s46HoPaA2"),
    );

    // Verify Project A history
    let history_a = client.get_verification_history(&project_a);
    assert_eq!(history_a.len(), 2);
    assert_eq!(history_a.get(0).unwrap().request_id, 1);
    assert_eq!(history_a.get(1).unwrap().request_id, 3);

    // Verify Project B history
    let history_b = client.get_verification_history(&project_b);
    assert_eq!(history_b.len(), 1);
    assert_eq!(history_b.get(0).unwrap().request_id, 2);
}
