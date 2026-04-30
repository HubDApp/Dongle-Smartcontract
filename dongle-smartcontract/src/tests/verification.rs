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
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(env, project_name),
        description: String::from_str(env, "Test project description"),
        category: String::from_str(env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };

    let project_id = client.register_project(&params);

    let token_admin = Address::generate(env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();

    client.set_fee(admin, &Some(token_address.clone()), &100, admin);

    let token_client = soroban_sdk::token::StellarAssetClient::new(env, &token_address);
    token_client.mint(owner, &1000);

    client.pay_fee(owner, &project_id, &Some(token_address));

    project_id
}

#[test]
fn test_verification_lifecycle() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, owner) = setup(&env);

    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "Project X"),
        description: String::from_str(&env, "Description... Description... Description..."),
        category: String::from_str(&env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };

    let project_id = client.register_project(&params);

    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.verification_status, VerificationStatus::Unverified);

    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();

    client.set_fee(&admin, &Some(token_address.clone()), &100, &admin);

    let token_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_address);
    token_client.mint(&owner, &1000);

    client.pay_fee(&owner, &project_id, &Some(token_address));

    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "ipfs://evidence"),
    );

    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.verification_status, VerificationStatus::Pending);

    client.approve_verification(&project_id, &admin);

    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.verification_status, VerificationStatus::Verified);
}

#[test]
fn test_reject_verification() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, owner) = setup(&env);

    let project_id = setup_project_with_fee(&client, &env, &admin, &owner, "Project Y");

    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "ipfs://evidence"),
    );

    client.reject_verification(&project_id, &admin);

    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.verification_status, VerificationStatus::Rejected);
}

#[test]
fn test_valid_state_transitions() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, owner) = setup(&env);

    let project_id = setup_project_with_fee(&client, &env, &admin, &owner, "Project 1");

    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.verification_status, VerificationStatus::Unverified);

    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "ipfs://evidence1"),
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

    let project_id2 = setup_project_with_fee(&client, &env, &admin, &owner, "Project 2");

    client.request_verification(
        &project_id2,
        &owner,
        &String::from_str(&env, "ipfs://evidence2"),
    );

    client.reject_verification(&project_id2, &admin);

    let project = client.get_project(&project_id2).unwrap();
    assert_eq!(project.verification_status, VerificationStatus::Rejected);

    assert_eq!(
        client.get_project(&project_id2).unwrap().verification_status,
        VerificationStatus::Rejected
    );

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

    assert_eq!(
        client.get_project(&project_id2).unwrap().verification_status,
        VerificationStatus::Pending
    );
}

#[test]
fn test_duplicate_payment_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, owner) = setup(&env);

    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "Project Dup"),
        description: String::from_str(&env, "Description... Description... Description..."),
        category: String::from_str(&env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };

    let project_id = client.register_project(&params);

    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();

    let token_client = soroban_sdk::token::StellarAssetClient::new(&env, &token_address);
    token_client.mint(&owner, &1000);

    client.set_fee(&admin, &Some(token_address.clone()), &100, &admin);

    client.pay_fee(&owner, &project_id, &Some(token_address.clone()));

    let result = client.try_pay_fee(&owner, &project_id, &Some(token_address));
    assert_eq!(result, Err(Ok(ContractError::FeeAlreadyPaid)));
}

#[test]
fn test_wrong_token_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, owner) = setup(&env);

    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "Project WrongTok"),
        description: String::from_str(&env, "Description... Description... Description..."),
        category: String::from_str(&env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };

    let project_id = client.register_project(&params);

    let token_admin = Address::generate(&env);
    let correct_token = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();

    let wrong_token_admin = Address::generate(&env);
    let wrong_token = env
        .register_stellar_asset_contract_v2(wrong_token_admin)
        .address();

    client.set_fee(&admin, &Some(correct_token), &100, &admin);

    let result = client.try_pay_fee(&owner, &project_id, &Some(wrong_token));
    assert_eq!(result, Err(Ok(ContractError::InvalidToken)));
}

#[test]
fn test_replay_attack_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, owner) = setup(&env);

    let project_id = setup_project_with_fee(&client, &env, &admin, &owner, "Project Replay");

    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "ipfs://evidence"),
    );

    let result = client.try_request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "ipfs://evidence2"),
    );

    assert_eq!(result, Err(Ok(ContractError::InvalidStatusTransition)));
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

    let result = client.try_request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "ipfs://evidence2"),
    );

    assert_eq!(result, Err(Ok(ContractError::InvalidStatusTransition)));
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

    let result = client.try_request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "ipfs://evidence2"),
    );
    assert_eq!(result, Err(Ok(ContractError::InvalidStatusTransition)));

    let result = client.try_approve_verification(&project_id, &admin);
    assert_eq!(result, Err(Ok(ContractError::InvalidStatusTransition)));

    let result = client.try_reject_verification(&project_id, &admin);
    assert_eq!(result, Err(Ok(ContractError::InvalidStatusTransition)));
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

    let result = client.try_approve_verification(&project_id, &admin);
    assert_eq!(result, Err(Ok(ContractError::InvalidStatusTransition)));

    let result = client.try_reject_verification(&project_id, &admin);
    assert_eq!(result, Err(Ok(ContractError::InvalidStatusTransition)));
}

#[test]
fn test_multiple_verification_cycles() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, owner) = setup(&env);

    let project_id = setup_project_with_fee(&client, &env, &admin, &owner, "Project Cycle");

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

    assert_eq!(result, Err(Ok(ContractError::InvalidStatusTransition)));
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

    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "ipfs://evidence"),
    );

    client.approve_verification(&project_id, &admin);

    let result = client.try_approve_verification(&project_id, &admin);
    assert_eq!(result, Err(Ok(ContractError::InvalidStatusTransition)));
}

#[test]
fn test_state_machine_with_different_admins() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, owner) = setup(&env);

    let admin2 = Address::generate(&env);
    client.add_admin(&admin, &admin2);

    let project_id = setup_project_with_fee(&client, &env, &admin, &owner, "Project Multi Admin");

    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "ipfs://evidence"),
    );

    client.approve_verification(&project_id, &admin2);

    assert_eq!(
        client.get_project(&project_id).unwrap().verification_status,
        VerificationStatus::Verified
    );
}