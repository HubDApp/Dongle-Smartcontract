//! Tests for owner-bound verification fee payments.

use crate::errors::ContractError;
use crate::types::ProjectRegistrationParams;
use crate::DongleContract;
use crate::DongleContractClient;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

fn setup(env: &Env) -> (DongleContractClient<'_>, Address, Address, Address) {
    let contract_id = env.register(DongleContract, ());
    let client = DongleContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.initialize(&admin);

    let token_admin = Address::generate(env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    client.set_fee(&admin, &Some(token.clone()), &100, &0u128, &admin);

    (client, admin, Address::generate(env), token)
}

fn register(client: &DongleContractClient<'_>, env: &Env, owner: &Address, name: &str) -> u64 {
    let slug = name.to_lowercase().replace(' ', "-");
    client.register_project(&ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(env, name),
        slug: String::from_str(env, &slug),
        description: String::from_str(env, "A test project description here"),
        category: String::from_str(env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
    })
}

fn mint(env: &Env, token: &Address, to: &Address, amount: i128) {
    soroban_sdk::token::StellarAssetClient::new(env, token).mint(to, &amount);
}

// --- Owner payment ---

#[test]
fn test_owner_can_pay_fee() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, owner, token) = setup(&env);
    let project_id = register(&client, &env, &owner, "Owner Pay");
    mint(&env, &token, &owner, 100);

    // Should succeed without error
    client.pay_fee(&owner, &project_id, &Some(token.clone()));

    // Fee consumed during request_verification — just verify it doesn't error
    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "ipfs://evidence"),
    );
}

// --- Third-party payment rejection ---

#[test]
fn test_non_owner_pay_fee_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, owner, token) = setup(&env);
    let project_id = register(&client, &env, &owner, "Third Party Pay");

    let stranger = Address::generate(&env);
    mint(&env, &token, &stranger, 100);

    let result = client.try_pay_fee(&stranger, &project_id, &Some(token.clone()));
    assert_eq!(result, Err(Ok(ContractError::Unauthorized)));
}

#[test]
fn test_non_owner_payment_does_not_enable_verification() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, owner, token) = setup(&env);
    let project_id = register(&client, &env, &owner, "No Stranger Fee");

    let stranger = Address::generate(&env);
    mint(&env, &token, &stranger, 100);

    // Stranger's payment is rejected
    let _ = client.try_pay_fee(&stranger, &project_id, &Some(token.clone()));

    // Owner has not paid — verification request must fail
    let result = client.try_request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "ipfs://evidence"),
    );
    assert_eq!(result, Err(Ok(ContractError::InsufficientFee)));
}

// --- Repeated payment ---

#[test]
fn test_repeated_payment_by_owner_overwrites_flag() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, owner, token) = setup(&env);
    let project_id = register(&client, &env, &owner, "Repeat Pay");
    mint(&env, &token, &owner, 200);

    // Pay twice — second call should succeed (idempotent flag set)
    client.pay_fee(&owner, &project_id, &Some(token.clone()));
    client.pay_fee(&owner, &project_id, &Some(token.clone()));

    // Verification should still work (flag is set)
    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "ipfs://evidence"),
    );
}

// --- Pay for nonexistent project ---

#[test]
fn test_pay_fee_for_nonexistent_project_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, owner, token) = setup(&env);
    mint(&env, &token, &owner, 100);

    let result = client.try_pay_fee(&owner, &9999, &Some(token.clone()));
    assert_eq!(result, Err(Ok(ContractError::ProjectNotFound)));
}

// --- Fee consumed after verification request ---

#[test]
fn test_fee_consumed_after_request_verification() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, owner, token) = setup(&env);
    let project_id = register(&client, &env, &owner, "Fee Consumed");
    mint(&env, &token, &owner, 200);

    client.pay_fee(&owner, &project_id, &Some(token.clone()));
    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "ipfs://evidence"),
    );

    // Reject so we can try to re-request without paying again
    client.approve_verification(&project_id, &admin);

    // Revoke so status goes back to Unverified
    client.revoke_verification(&project_id, &admin, &String::from_str(&env, "test revoke"));

    // Fee was consumed — re-request without paying should fail
    let result = client.try_request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "ipfs://evidence2"),
    );
    assert_eq!(result, Err(Ok(ContractError::InsufficientFee)));
}
