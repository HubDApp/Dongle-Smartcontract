//! Boundary tests for fee amounts: u128 storage / i128 transfer safety (#221).

use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::DongleContractClient;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

fn setup_with_token(env: &Env) -> (DongleContractClient<'_>, Address, Address) {
    let (client, admin) = setup_contract(env);
    let token_admin = Address::generate(env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    (client, admin, token)
}

fn mint_token(env: &Env, token: &Address, to: &Address, amount: i128) {
    soroban_sdk::token::StellarAssetClient::new(env, token).mint(to, &amount);
}

// ─── Zero fee ────────────────────────────────────────────────────────────────

#[test]
fn test_zero_fee_succeeds_without_transfer() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    // fee = 0, no token required
    client.set_fee(&admin, &None, &0u128, &0u128, &admin);
    let project_id = create_test_project(&client, &owner, "ZeroFeeProject");
    // pay_fee with zero fee should succeed without any token transfer
    client.pay_fee(&owner, &project_id, &None);
    assert!(client.is_fee_paid(&project_id));
}

// ─── Small valid fee ─────────────────────────────────────────────────────────

#[test]
fn test_small_fee_succeeds() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, token) = setup_with_token(&env);
    let owner = Address::generate(&env);
    client.set_fee(&admin, &Some(token.clone()), &1u128, &0u128, &admin);
    let project_id = create_test_project(&client, &owner, "SmallFeeProject");
    mint_token(&env, &token, &owner, 1);
    client.pay_fee(&owner, &project_id, &Some(token.clone()));
    assert!(client.is_fee_paid(&project_id));
}

// ─── Max valid fee (i128::MAX) ───────────────────────────────────────────────

#[test]
fn test_max_valid_fee_i128_max_succeeds() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, token) = setup_with_token(&env);
    let owner = Address::generate(&env);
    let max_fee = i128::MAX as u128;
    client.set_fee(&admin, &Some(token.clone()), &max_fee, &0u128, &admin);
    let project_id = create_test_project(&client, &owner, "MaxFeeProject");
    mint_token(&env, &token, &owner, i128::MAX);
    client.pay_fee(&owner, &project_id, &Some(token.clone()));
    assert!(client.is_fee_paid(&project_id));
}

// ─── Fee exceeding i128::MAX must be rejected before transfer ────────────────

#[test]
fn test_fee_exceeding_i128_max_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, token) = setup_with_token(&env);
    let owner = Address::generate(&env);
    let overflow_fee = (i128::MAX as u128).saturating_add(1);
    client.set_fee(&admin, &Some(token.clone()), &overflow_fee, &0u128, &admin);
    let project_id = create_test_project(&client, &owner, "OverflowFeeProject");
    // No tokens minted — the guard must fire before the transfer is attempted.
    let result = client.try_pay_fee(&owner, &project_id, &Some(token.clone()));
    assert_eq!(result, Err(Ok(ContractError::InvalidProjectData)));
    // Fee must NOT be marked as paid.
    assert!(!client.is_fee_paid(&project_id));
}

// ─── Registration fee boundary ───────────────────────────────────────────────

#[test]
fn test_registration_fee_exceeding_i128_max_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, token) = setup_with_token(&env);
    let overflow_fee = (i128::MAX as u128).saturating_add(1);
    client.set_fee(&admin, &Some(token.clone()), &0u128, &overflow_fee, &admin);
    let payer = Address::generate(&env);
    let result = client.try_pay_registration_fee(&payer, &Some(token.clone()));
    assert_eq!(result, Err(Ok(ContractError::InvalidProjectData)));
}
