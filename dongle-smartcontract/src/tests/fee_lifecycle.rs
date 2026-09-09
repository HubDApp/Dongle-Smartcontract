//! Integration test: full verification-fee payment lifecycle.
//!
//! Covers the complete sequence in one end-to-end scenario:
//!
//!  1. Admin configures a non-zero fee with a real SAC token.
//!  2. Owner pays the fee → fee-paid flag is set, payment details are stored.
//!  3. `request_verification` consumes the fee → flag is cleared.
//!  4. A second `request_verification` (without re-payment) is rejected with
//!     `InsufficientFee`.
//!  5. Re-payment restores the flag and a third `request_verification` succeeds.
//!
//! Additionally verifies:
//!  - token balances change correctly on payment
//!  - `get_fee_payment_details` returns correct data after payment
//!  - `get_fee_payment_details` is retained (audit trail) after the flag is cleared
//!  - the payment details record contains the right payer, amount, and token

#![cfg(test)]

use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::VerificationStatus;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

// A valid IPFS CIDv0 reused across sub-tests.
const CID_A: &str = "QmTu64kW8cUwwigCcJcKQS6F6wTwwJeD8Y18qr9s9DXkXy";
const CID_B: &str = "QmYwAPJhy5nTAQCj9g1s2bkss7jBlEd22bN2R4s5gR5PTa";
const CID_C: &str = "QmYwAPJhy5nTAQCj9g1s2bkss7jBlEd22bN2R4s5gR5PTb";

/// Shared setup: contract + admin + token, fee = 100 stroops, treasury = admin.
///
/// Returns `(client, admin, owner, token, treasury)`.
fn setup_with_fee(
    env: &Env,
) -> (
    crate::DongleContractClient<'_>,
    Address,
    Address,
    Address,
    Address,
) {
    let (client, admin) = setup_contract(env);

    // Deploy a Stellar Asset Contract so the token transfer path is exercised.
    let token_admin = Address::generate(env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();

    let treasury = Address::generate(env);

    // verification_fee = 100, registration_fee = 0 (keep registration free so
    // create_test_project doesn't need a pre-payment in every test).
    client
        .mock_all_auths()
        .set_fee(&admin, &Some(token.clone()), &100u128, &0u128, &treasury);

    let owner = Address::generate(env);
    (client, admin, owner, token, treasury)
}

/// Mint `amount` of `token` to `to`.
fn mint(env: &Env, token: &Address, to: &Address, amount: i128) {
    soroban_sdk::token::StellarAssetClient::new(env, token).mint(to, &amount);
}

// ─── Core lifecycle ──────────────────────────────────────────────────────────

/// Full happy-path lifecycle in one test so the sequence is unambiguous.
#[test]
fn test_full_fee_payment_lifecycle() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, owner, token, treasury) = setup_with_fee(&env);
    let token_client = soroban_sdk::token::Client::new(&env, &token);

    let project_id = create_test_project(&client, &owner, "LifecycleProject");

    // ── Step 1: before payment ───────────────────────────────────────────────

    assert!(
        !client.is_fee_paid(&project_id),
        "fee-paid flag must be unset before any payment"
    );
    assert!(
        client.get_fee_payment_details(&project_id).is_none(),
        "no payment details should exist before payment"
    );

    // Attempting to request verification without paying must be rejected.
    let err = client
        .try_request_verification(&project_id, &owner, &String::from_str(&env, CID_A))
        .unwrap_err()
        .unwrap();
    assert_eq!(
        err,
        ContractError::InsufficientFee,
        "request_verification before payment must fail with InsufficientFee"
    );

    // ── Step 2: pay_fee ──────────────────────────────────────────────────────

    mint(&env, &token, &owner, 300); // enough for three payments

    client.pay_fee(&owner, &project_id, &Some(token.clone()));

    // Flag is now set.
    assert!(
        client.is_fee_paid(&project_id),
        "fee-paid flag must be set immediately after pay_fee"
    );

    // Payment details are stored with the correct fields.
    let details = client
        .get_fee_payment_details(&project_id)
        .expect("payment details must exist after pay_fee");
    assert_eq!(details.payer, owner, "payer must match the owner");
    assert_eq!(details.amount, 100u128, "amount must equal configured fee");
    assert_eq!(details.token, Some(token.clone()), "token must be recorded");

    // Token balances reflect the transfer.
    assert_eq!(
        token_client.balance(&owner),
        200, // 300 minted − 100 paid
        "owner balance must decrease by the fee amount"
    );
    assert_eq!(
        token_client.balance(&treasury),
        100,
        "treasury must receive the fee"
    );

    // ── Step 3: request_verification consumes the fee ────────────────────────

    client.request_verification(&project_id, &owner, &String::from_str(&env, CID_A));

    // Project is now Pending.
    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.verification_status, VerificationStatus::Pending);

    // Fee-paid flag is cleared after consumption.
    assert!(
        !client.is_fee_paid(&project_id),
        "fee-paid flag must be cleared after request_verification consumes it"
    );

    // The payment-details record is retained as an audit trail even though the
    // flag is gone.
    let details_after = client
        .get_fee_payment_details(&project_id)
        .expect("payment details audit record must survive fee consumption");
    assert_eq!(
        details_after.payer, owner,
        "retained details must still identify the original payer"
    );
    assert_eq!(
        details_after.amount, 100u128,
        "retained details must still show the original amount"
    );

    // ── Step 4: second request_verification without re-payment is rejected ───

    // Admin approves the first request so the project moves to Verified, then
    // revokes so it returns to Unverified (the only state that allows a fresh
    // request_verification call).
    client.approve_verification(&project_id, &admin);
    assert_eq!(
        client.get_project(&project_id).unwrap().verification_status,
        VerificationStatus::Verified
    );
    client.revoke_verification(
        &project_id,
        &admin,
        &String::from_str(&env, "lifecycle test revoke"),
    );
    assert_eq!(
        client.get_project(&project_id).unwrap().verification_status,
        VerificationStatus::Unverified
    );

    // Now the status allows a new request, but the fee has not been re-paid.
    let err = client
        .try_request_verification(&project_id, &owner, &String::from_str(&env, CID_B))
        .unwrap_err()
        .unwrap();
    assert_eq!(
        err,
        ContractError::InsufficientFee,
        "second request_verification without re-payment must fail with InsufficientFee"
    );

    // ── Step 5: re-payment restores the flag; third request succeeds ─────────

    client.pay_fee(&owner, &project_id, &Some(token.clone()));

    assert!(
        client.is_fee_paid(&project_id),
        "fee-paid flag must be set again after re-payment"
    );

    // Third request must succeed now that the fee has been re-paid.
    client.request_verification(&project_id, &owner, &String::from_str(&env, CID_C));

    assert_eq!(
        client.get_project(&project_id).unwrap().verification_status,
        VerificationStatus::Pending,
        "project must be Pending after successful third request_verification"
    );

    // Flag is cleared again.
    assert!(
        !client.is_fee_paid(&project_id),
        "fee-paid flag must be cleared after the third request_verification"
    );

    // Final balances: owner paid 200 total (2 × 100), 100 still unspent.
    assert_eq!(
        token_client.balance(&owner),
        100, // 300 minted − 200 paid
        "owner must have 100 tokens remaining after two payments"
    );
    assert_eq!(
        token_client.balance(&treasury),
        200,
        "treasury must have accumulated both fee payments"
    );
}

// ─── Isolated unit assertions ────────────────────────────────────────────────

/// Confirm `is_fee_paid` returns `false` for a project that has never had a
/// payment — guards against a stale-default bug.
#[test]
fn test_is_fee_paid_false_before_payment() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, owner, _token, _treasury) = setup_with_fee(&env);
    let project_id = create_test_project(&client, &owner, "FlagPreCheck");
    assert!(!client.is_fee_paid(&project_id));
}

/// After `pay_fee`, `is_fee_paid` returns `true`.
#[test]
fn test_is_fee_paid_true_after_payment() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, owner, token, _treasury) = setup_with_fee(&env);
    let project_id = create_test_project(&client, &owner, "FlagSet");
    mint(&env, &token, &owner, 100);
    client.pay_fee(&owner, &project_id, &Some(token.clone()));
    assert!(client.is_fee_paid(&project_id));
}

/// After `request_verification`, `is_fee_paid` returns `false`.
#[test]
fn test_is_fee_paid_false_after_consumption() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, owner, token, _treasury) = setup_with_fee(&env);
    let project_id = create_test_project(&client, &owner, "FlagConsumed");
    mint(&env, &token, &owner, 100);
    client.pay_fee(&owner, &project_id, &Some(token.clone()));
    client.request_verification(&project_id, &owner, &String::from_str(&env, CID_A));
    assert!(!client.is_fee_paid(&project_id));
}

/// `get_fee_payment_details` is `None` before any payment.
#[test]
fn test_payment_details_none_before_payment() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, owner, _token, _treasury) = setup_with_fee(&env);
    let project_id = create_test_project(&client, &owner, "DetailsNone");
    assert!(client.get_fee_payment_details(&project_id).is_none());
}

/// `get_fee_payment_details` contains the correct payer, amount, and token
/// immediately after `pay_fee`.
#[test]
fn test_payment_details_correct_after_payment() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, owner, token, _treasury) = setup_with_fee(&env);
    let project_id = create_test_project(&client, &owner, "DetailsCorrect");
    mint(&env, &token, &owner, 100);
    client.pay_fee(&owner, &project_id, &Some(token.clone()));

    let rec = client.get_fee_payment_details(&project_id).unwrap();
    assert_eq!(rec.payer, owner);
    assert_eq!(rec.amount, 100u128);
    assert_eq!(rec.token, Some(token));
}

/// `get_fee_payment_details` is still present (audit trail) after the fee is
/// consumed by `request_verification`.
#[test]
fn test_payment_details_retained_after_consumption() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, owner, token, _treasury) = setup_with_fee(&env);
    let project_id = create_test_project(&client, &owner, "DetailsRetained");
    mint(&env, &token, &owner, 100);
    client.pay_fee(&owner, &project_id, &Some(token.clone()));
    client.request_verification(&project_id, &owner, &String::from_str(&env, CID_A));

    // The flag is gone …
    assert!(!client.is_fee_paid(&project_id));
    // … but the details record lives on.
    assert!(
        client.get_fee_payment_details(&project_id).is_some(),
        "payment details must be retained as an audit trail after fee consumption"
    );
}

/// Verify that `request_verification` without prior `pay_fee` is rejected even
/// when the project exists and the CID is valid.
#[test]
fn test_request_verification_without_payment_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, owner, _token, _treasury) = setup_with_fee(&env);
    let project_id = create_test_project(&client, &owner, "NoPay");

    let err = client
        .try_request_verification(&project_id, &owner, &String::from_str(&env, CID_A))
        .unwrap_err()
        .unwrap();
    assert_eq!(err, ContractError::InsufficientFee);
}

/// Treasury balance increases by exactly the configured fee amount after payment.
#[test]
fn test_treasury_receives_fee_on_payment() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, owner, token, treasury) = setup_with_fee(&env);
    let token_client = soroban_sdk::token::Client::new(&env, &token);
    let project_id = create_test_project(&client, &owner, "TreasuryCheck");

    let treasury_before = token_client.balance(&treasury);
    mint(&env, &token, &owner, 100);
    client.pay_fee(&owner, &project_id, &Some(token.clone()));

    assert_eq!(
        token_client.balance(&treasury),
        treasury_before + 100,
        "treasury must increase by exactly the fee amount"
    );
    assert_eq!(
        token_client.balance(&owner),
        0,
        "owner balance must be zero after spending the full minted amount"
    );
}
