use crate::errors::ContractError;
use crate::types::{ProjectRegistrationParams, VerificationStatus};
use crate::DongleContract;
use crate::DongleContractClient;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

fn setup(env: &Env) -> (DongleContractClient<'_>, Address, Address) {
    let contract_id = env.register_contract(None, DongleContract);
    let client = DongleContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.initialize(&admin);
    (client, admin, Address::generate(env))
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
        description: String::from_str(&env, "Description... Description... Description..."),
        category: String::from_str(&env, "NFT"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
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

    // First payment succeeds
    client.pay_fee(&owner, &project_id, &Some(token_address.clone()));

    // Second payment in same cycle must be rejected
    let result = client
        .try_pay_fee(&owner, &project_id, &Some(token_address.clone()));
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

    client.set_fee(&admin, &Some(correct_token.clone()), &100, &admin);

    // Paying with a different token must be rejected
    let result = client.try_pay_fee(&owner, &project_id, &Some(wrong_token));
    assert_eq!(result, Err(Ok(ContractError::InvalidToken)));
}

#[test]
fn test_replay_attack_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, owner) = setup(&env);

    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "Project Replay"),
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

    // Pay and request verification (fee is consumed)
    client.pay_fee(&owner, &project_id, &Some(token_address.clone()));
    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "ipfs://evidence"),
    );

    // Replaying request_verification without paying again must be rejected
    let result = client.try_request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "ipfs://evidence2"),
    );
    assert_eq!(result, Err(Ok(ContractError::InvalidStatusTransition)));
}
