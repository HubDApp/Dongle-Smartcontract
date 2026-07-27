//! Tests for fee payment details getter (Issue #223).
//!
//! Verifies that FeePaymentRecord stores payer, amount, token, and timestamp,
//! and that the getter returns correct data after payment.

use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::ProjectRegistrationParams;
use crate::DongleContract;
use crate::DongleContractClient;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

const VALID_EVIDENCE_CID: &str = "QmTu64kW8cUwwigCcJcKQS6F6wTwwJeD8Y18qr9s9DXkXy";

fn setup_with_token(env: &Env) -> (DongleContractClient<'_>, Address, Address) {
    let (client, admin) = setup_contract(env);
    let token_admin = Address::generate(env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    client
        .mock_all_auths()
        .set_fee(&admin, &Some(token.clone()), &100, &0u128, &admin);
    (client, admin, token)
}

fn mint(env: &Env, token: &Address, to: &Address, amount: i128) {
    soroban_sdk::token::StellarAssetClient::new(env, token).mint(to, &amount);
}

#[test]
fn test_fee_payment_details_stored_after_pay() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, token) = setup_with_token(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "PayerTest");
    mint(&env, &token, &owner, 200);

    // Before payment, no details
    let details = client.get_fee_payment_details(&project_id);
    assert!(details.is_none());

    // Pay fee
    client.pay_fee(&owner, &project_id, &Some(token.clone()));

    // After payment, details exist
    let details = client.get_fee_payment_details(&project_id).unwrap();
    assert_eq!(details.payer, owner);
    assert_eq!(details.amount, 100);
    assert_eq!(details.token, Some(token));
    assert!(details.paid_at > 0);
}

#[test]
fn test_fee_payment_details_consumed_after_verification() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, token) = setup_with_token(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "ConsumeTest");
    mint(&env, &token, &owner, 200);

    client.pay_fee(&owner, &project_id, &Some(token.clone()));

    // Details are available after pay
    let details = client.get_fee_payment_details(&project_id);
    assert!(details.is_some());

    // Request verification (consumes fee)
    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, VALID_EVIDENCE_CID),
    );

    // Details record should still be readable (it's a separate storage key)
    let details = client.get_fee_payment_details(&project_id);
    assert!(details.is_some());
}

#[test]
fn test_fee_payment_details_zero_fee() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    // Zero fees
    client
        .mock_all_auths()
        .set_fee(&admin, &None, &0u128, &0u128, &admin);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "ZeroFeeDetails");

    client.pay_fee(&owner, &project_id, &None);

    let details = client.get_fee_payment_details(&project_id).unwrap();
    assert_eq!(details.payer, owner);
    assert_eq!(details.amount, 0);
    assert!(details.token.is_none());
}
