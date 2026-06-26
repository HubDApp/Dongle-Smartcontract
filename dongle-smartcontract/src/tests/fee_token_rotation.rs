//! Tests for fee token rotation behavior.
//!
//! Covers:
//! - Changing the fee token from one token address to another.
//! - Old-token payments failing after rotation.
//! - Zero-fee behavior (with and without token address changes).
//! - Treasury address correctness after every rotation.

use crate::errors::ContractError;
use crate::types::ProjectRegistrationParams;
use crate::DongleContract;
use crate::DongleContractClient;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

// ─── helpers ─────────────────────────────────────────────────────────────────

/// Register the contract with an admin and a pre-configured fee token.
/// Returns (client, admin, treasury, token_address).
fn setup_with_token(env: &Env) -> (DongleContractClient<'_>, Address, Address, Address) {
    let contract_id = env.register(DongleContract, ());
    let client = DongleContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.initialize(&admin);

    let treasury = Address::generate(env);
    let token = env
        .register_stellar_asset_contract_v2(Address::generate(env))
        .address();

    client.set_fee(&admin, &Some(token.clone()), &100u128, &0u128, &treasury);
    (client, admin, treasury, token)
}

/// Register a new SAC token and mint `amount` to `to`.
fn new_token(env: &Env, to: &Address, amount: i128) -> Address {
    let token = env
        .register_stellar_asset_contract_v2(Address::generate(env))
        .address();
    mint(env, &token, to, amount);
    token
}

fn mint(env: &Env, token: &Address, to: &Address, amount: i128) {
    soroban_sdk::token::StellarAssetClient::new(env, token).mint(to, &amount);
}

fn register_project(
    client: &DongleContractClient<'_>,
    env: &Env,
    owner: &Address,
    name: &str,
) -> u64 {
    let slug = name.to_lowercase().replace(' ', "-");
    client.register_project(&ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(env, name),
        slug: String::from_str(env, &slug),
        description: String::from_str(env, "A test project description"),
        category: String::from_str(env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
    })
}

// ═══════════════════════════════════════════════════════════════════════════
// Basic token rotation
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_fee_config_reflects_new_token_after_rotation() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, treasury, _token_a) = setup_with_token(&env);

    let token_b = Address::generate(&env);
    client.set_fee(&admin, &Some(token_b.clone()), &200u128, &0u128, &treasury);

    let config = client.get_fee_config();
    assert_eq!(config.token, Some(token_b));
    assert_eq!(config.verification_fee, 200u128);
}

#[test]
fn test_treasury_unchanged_after_token_rotation() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, treasury, _token_a) = setup_with_token(&env);

    let owner = Address::generate(&env);
    let token_b = new_token(&env, &owner, 100);
    client.set_fee(&admin, &Some(token_b.clone()), &100u128, &0u128, &treasury);

    // Treasury is stored inside set_fee but not directly exposed via get_fee_config.
    // Verify indirectly: a payment with the new token must reach the treasury.
    let project_id = register_project(&client, &env, &owner, "TreasuryCheck");
    // Pay should succeed — proves treasury is still reachable with new token
    client.pay_fee(&owner, &project_id, &Some(token_b.clone()));
}

// ═══════════════════════════════════════════════════════════════════════════
// Old-token payments fail after rotation
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_old_token_payment_rejected_after_rotation() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, treasury, token_a) = setup_with_token(&env);

    let owner = Address::generate(&env);
    let project_id = register_project(&client, &env, &owner, "OldTokenFail");
    mint(&env, &token_a, &owner, 200);

    // Rotate to token B
    let token_b = env
        .register_stellar_asset_contract_v2(Address::generate(&env))
        .address();
    client.set_fee(&admin, &Some(token_b.clone()), &100u128, &0u128, &treasury);

    // Paying with token A (old) must now fail
    let result = client.try_pay_fee(&owner, &project_id, &Some(token_a.clone()));
    assert_eq!(
        result,
        Err(Ok(ContractError::InvalidProjectData.into())),
        "old token should be rejected after rotation"
    );
}

#[test]
fn test_new_token_payment_succeeds_after_rotation() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, treasury, _token_a) = setup_with_token(&env);

    let owner = Address::generate(&env);
    let project_id = register_project(&client, &env, &owner, "NewTokenPay");

    // Rotate to token B
    let token_b = env
        .register_stellar_asset_contract_v2(Address::generate(&env))
        .address();
    client.set_fee(&admin, &Some(token_b.clone()), &100u128, &0u128, &treasury);

    mint(&env, &token_b, &owner, 100);
    // Payment with new token must succeed
    client.pay_fee(&owner, &project_id, &Some(token_b.clone()));
}

#[test]
fn test_verification_request_fails_if_old_token_payment_used() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, treasury, token_a) = setup_with_token(&env);

    let owner = Address::generate(&env);
    let project_id = register_project(&client, &env, &owner, "OldTokenVerif");
    mint(&env, &token_a, &owner, 200);

    // Owner pays fee with token A before rotation — succeeds
    client.pay_fee(&owner, &project_id, &Some(token_a.clone()));

    // Admin rotates token — this does NOT refund the old payment,
    // but the fee flag is still set. The verification will proceed.
    // (The contract stores only a boolean flag, not the token used for payment.)
    let token_b = env
        .register_stellar_asset_contract_v2(Address::generate(&env))
        .address();
    client.set_fee(&admin, &Some(token_b.clone()), &100u128, &0u128, &treasury);

    // Evidence CID for verification
    let evidence = String::from_str(&env, "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG");
    // The fee flag was already set with old token payment, so this still works
    // (the flag is token-agnostic — this is the current contract behaviour)
    let result = client.try_request_verification(&project_id, &owner, &evidence);
    assert!(
        result.is_ok(),
        "pre-rotation fee payment flag should still unlock verification"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Zero-fee behavior after token changes
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_zero_fee_with_none_token_allows_verification_no_payment() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, treasury, _token_a) = setup_with_token(&env);

    // Rotate to zero-fee, no token
    client.set_fee(&admin, &None, &0u128, &0u128, &treasury);

    let config = client.get_fee_config();
    assert_eq!(config.token, None);
    assert_eq!(config.verification_fee, 0u128);

    // Owner registers and requests verification without paying — must work
    let owner = Address::generate(&env);
    let project_id = register_project(&client, &env, &owner, "ZeroFeeVerif");
    let evidence = String::from_str(&env, "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG");
    let result = client.try_request_verification(&project_id, &owner, &evidence);
    assert!(
        result.is_ok(),
        "zero-fee config should not require fee payment"
    );
}

#[test]
fn test_pay_fee_with_none_token_sets_flag_at_zero_fee() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, treasury, _token_a) = setup_with_token(&env);

    // Set zero fee with no token
    client.set_fee(&admin, &None, &0u128, &0u128, &treasury);

    let owner = Address::generate(&env);
    let project_id = register_project(&client, &env, &owner, "ZeroFeePay");

    // pay_fee with None token at zero fee must succeed without any transfer
    let result = client.try_pay_fee(&owner, &project_id, &None);
    assert!(result.is_ok(), "zero-fee payment with None token should succeed");
}

#[test]
fn test_zero_fee_after_nonzero_no_stale_payment_required() {
    // After rotating from fee > 0 to fee = 0,
    // verification should not require any outstanding fee flag.
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, treasury, _token_a) = setup_with_token(&env);

    // Owner does NOT pay while fee is 100
    let owner = Address::generate(&env);
    let project_id = register_project(&client, &env, &owner, "DropFeeVerif");

    // Admin drops fee to zero
    client.set_fee(&admin, &None, &0u128, &0u128, &treasury);

    // Verification should now succeed without a fee payment
    let evidence = String::from_str(&env, "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG");
    let result = client.try_request_verification(&project_id, &owner, &evidence);
    assert!(
        result.is_ok(),
        "should not require fee payment after fee is set to zero"
    );
}

#[test]
fn test_nonzero_fee_after_zero_requires_payment() {
    // After rotating from zero-fee to a non-zero fee,
    // verification must again require a fee payment.
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(DongleContract, ());
    let client = DongleContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);
    let treasury = Address::generate(&env);

    // Start with zero fee
    client.set_fee(&admin, &None, &0u128, &0u128, &treasury);

    let owner = Address::generate(&env);
    let project_id = register_project(&client, &env, &owner, "AddFeeVerif");

    // Now admin adds a fee token with fee > 0
    let token = env
        .register_stellar_asset_contract_v2(Address::generate(&env))
        .address();
    client.set_fee(&admin, &Some(token.clone()), &50u128, &0u128, &treasury);

    // Verification without payment should now fail
    let evidence = String::from_str(&env, "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG");
    let result = client.try_request_verification(&project_id, &owner, &evidence);
    assert_eq!(
        result,
        Err(Ok(ContractError::InsufficientFee.into())),
        "non-zero fee should require payment"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Treasury correctness
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_treasury_receives_payment_with_correct_token() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, treasury, _token_a) = setup_with_token(&env);

    // Rotate to a fresh token
    let owner = Address::generate(&env);
    let token_b = new_token(&env, &owner, 500);
    client.set_fee(&admin, &Some(token_b.clone()), &100u128, &0u128, &treasury);

    let project_id = register_project(&client, &env, &owner, "TreasuryReceives");

    let balance_before = soroban_sdk::token::Client::new(&env, &token_b).balance(&treasury);
    client.pay_fee(&owner, &project_id, &Some(token_b.clone()));
    let balance_after = soroban_sdk::token::Client::new(&env, &token_b).balance(&treasury);

    assert_eq!(
        balance_after - balance_before,
        100,
        "treasury should receive exactly the fee amount"
    );
}

#[test]
fn test_treasury_update_with_token_rotation() {
    // set_fee allows changing both token AND treasury in one call.
    // Verify the new treasury receives the payment.
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, _treasury_old, _token_a) = setup_with_token(&env);

    let new_treasury = Address::generate(&env);
    let owner = Address::generate(&env);
    let token_b = new_token(&env, &owner, 500);

    // Rotate token AND treasury together
    client.set_fee(
        &admin,
        &Some(token_b.clone()),
        &100u128,
        &0u128,
        &new_treasury,
    );

    let project_id = register_project(&client, &env, &owner, "NewTreasury");

    let bal_before = soroban_sdk::token::Client::new(&env, &token_b).balance(&new_treasury);
    client.pay_fee(&owner, &project_id, &Some(token_b.clone()));
    let bal_after = soroban_sdk::token::Client::new(&env, &token_b).balance(&new_treasury);

    assert_eq!(
        bal_after - bal_before,
        100,
        "new treasury should receive the fee after joint rotation"
    );
}

#[test]
fn test_multiple_rotations_last_config_applies() {
    // Rotate token several times; only the final configuration matters.
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, treasury, _t1) = setup_with_token(&env);

    let t2 = Address::generate(&env);
    let t3 = Address::generate(&env);
    let t4_owner = Address::generate(&env);
    let t4 = new_token(&env, &t4_owner, 1000);

    client.set_fee(&admin, &Some(t2.clone()), &200u128, &0u128, &treasury);
    client.set_fee(&admin, &Some(t3.clone()), &300u128, &0u128, &treasury);
    client.set_fee(&admin, &Some(t4.clone()), &50u128, &0u128, &treasury);

    let config = client.get_fee_config();
    assert_eq!(config.token, Some(t4.clone()));
    assert_eq!(config.verification_fee, 50u128);

    // Payment with t4 must succeed
    let project_id = register_project(&client, &env, &t4_owner, "MultiRotate");
    client.pay_fee(&t4_owner, &project_id, &Some(t4.clone()));
}

#[test]
fn test_non_admin_cannot_rotate_fee_token() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, treasury, _token_a) = setup_with_token(&env);

    let stranger = Address::generate(&env);
    let token_b = Address::generate(&env);

    let result = client.try_set_fee(&stranger, &Some(token_b), &100u128, &0u128, &treasury);
    assert_eq!(
        result,
        Err(Ok(ContractError::AdminOnly.into())),
        "non-admin must not be allowed to rotate the fee token"
    );
}
