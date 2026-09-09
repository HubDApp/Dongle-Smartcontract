//! Tests for owner-bound verification fee payments.
//!
//! Coverage matrix
//! ───────────────
//! • Token fee    – non-zero fee with a real SAC token address
//! • Zero fee     – token = None, both fees = 0 (no transfer required)
//! • Native fee   – token = None with non-zero fee (must be rejected)

use crate::errors::ContractError;
use crate::types::ProjectRegistrationParams;
use crate::DongleContract;
use crate::DongleContractClient;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

// A valid IPFS CIDv0 for testing (46 characters)
const VALID_EVIDENCE_CID: &str = "QmTu64kW8cUwwigCcJcKQS6F6wTwwJeD8Y18qr9s9DXkXy";

fn setup(env: &Env) -> (DongleContractClient<'_>, Address, Address, Address) {
    let contract_id = env.register(DongleContract, ());
    let client = DongleContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.initialize(&admin);

    let token_admin = Address::generate(env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    // Set fees: verification_fee = 100, registration_fee = 0 (to avoid registration fee during tests)
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
        launch_timestamp: None,
        bounty_url: None,
        repository_url: None,
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
    let project_id = register(&client, &env, &owner, "OwnerPay");
    mint(&env, &token, &owner, 100);

    // Should succeed without error
    client.pay_fee(&owner, &project_id, &Some(token.clone()));

    // Fee consumed during request_verification — just verify it doesn't error
    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, VALID_EVIDENCE_CID),
    );
}

// --- Third-party payment rejection ---

#[test]
fn test_non_owner_pay_fee_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, owner, token) = setup(&env);
    let project_id = register(&client, &env, &owner, "ThirdPartyPay");

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
    let project_id = register(&client, &env, &owner, "NoStrangerFee");

    let stranger = Address::generate(&env);
    mint(&env, &token, &stranger, 100);

    // Stranger's payment is rejected
    let _ = client.try_pay_fee(&stranger, &project_id, &Some(token.clone()));

    // Owner has not paid — verification request must fail
    let result = client.try_request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, VALID_EVIDENCE_CID),
    );
    assert_eq!(result, Err(Ok(ContractError::InsufficientFee)));
}

// --- Repeated payment ---

#[test]
fn test_repeated_payment_by_owner_overwrites_flag() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, owner, token) = setup(&env);
    let project_id = register(&client, &env, &owner, "RepeatPay");
    mint(&env, &token, &owner, 200);

    // Pay twice — second call should succeed (idempotent flag set)
    client.pay_fee(&owner, &project_id, &Some(token.clone()));
    client.pay_fee(&owner, &project_id, &Some(token.clone()));

    // Verification should still work (flag is set)
    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, VALID_EVIDENCE_CID),
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

// --- Zero-fee path (no token required) ---

/// When both fees are zero the admin may omit the token address entirely.
/// pay_fee with None must succeed without any token transfer.
#[test]
fn test_zero_fee_no_token_succeeds() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(DongleContract, ());
    let client = DongleContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    // Zero fees — no token needed
    client.set_fee(&admin, &None, &0u128, &0u128, &admin);

    let owner = Address::generate(&env);
    let project_id = register(&client, &env, &owner, "Zero Fee Project");

    // pay_fee with None token and zero fee should succeed (no transfer occurs)
    client.pay_fee(&owner, &project_id, &None);

    // Verification can proceed because fee consumption is skipped when fee == 0
    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "ipfs://evidence"),
    );
}

// --- Native fee rejection ---

/// Configuring a non-zero fee without a token address (i.e. native asset) must
/// be rejected at set_fee time with FeeConfigNotSet.
#[test]
fn test_native_fee_rejected_at_config() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(DongleContract, ());
    let client = DongleContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    // verification_fee non-zero, no token → must fail
    let result = client.try_set_fee(&admin, &None, &100u128, &0u128, &admin);
    assert_eq!(result, Err(Ok(ContractError::FeeConfigNotSet)));

    // registration_fee non-zero, no token → must also fail
    let result = client.try_set_fee(&admin, &None, &0u128, &50u128, &admin);
    assert_eq!(result, Err(Ok(ContractError::FeeConfigNotSet)));
}

// --- Fee consumed after verification request ---

#[test]
fn test_fee_consumed_after_request_verification() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, owner, token) = setup(&env);
    let project_id = register(&client, &env, &owner, "FeeConsumed");
    mint(&env, &token, &owner, 200);

    client.pay_fee(&owner, &project_id, &Some(token.clone()));
    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, VALID_EVIDENCE_CID),
    );

    // Reject so we can try to re-request without paying again
    client.approve_verification(&project_id, &admin);

    // Revoke so status goes back to Unverified
    client.revoke_verification(&project_id, &admin, &String::from_str(&env, "test revoke"));

    // Fee was consumed — re-request without paying should fail
    let result = client.try_request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdj"),
    );
    assert_eq!(result, Err(Ok(ContractError::InsufficientFee)));
}

// --- Fee Cancellation Tests ---

#[test]
fn test_owner_can_cancel_pending_fee_payment() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, owner, token) = setup(&env);
    let project_id = register(&client, &env, &owner, "OwnerCancel");
    mint(&env, &token, &owner, 100);

    // Pay fee
    client.pay_fee(&owner, &project_id, &Some(token.clone()));
    assert!(client.is_fee_paid(&project_id));

    // Verify token balance of treasury (should be 100)
    let token_client = soroban_sdk::token::Client::new(&env, &token);
    let treasury = client.get_config().unwrap().treasury.unwrap();
    assert_eq!(token_client.balance(&treasury), 100);
    assert_eq!(token_client.balance(&owner), 0);

    // Cancel fee payment
    client.cancel_fee_payment(&owner, &project_id);

    // Verify fee is no longer paid and refund is received
    assert!(!client.is_fee_paid(&project_id));
    assert_eq!(token_client.balance(&owner), 100);
    assert_eq!(token_client.balance(&treasury), 0);
}

#[test]
fn test_admin_can_cancel_pending_fee_payment() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, owner, token) = setup(&env);
    let project_id = register(&client, &env, &owner, "AdminCancel");
    mint(&env, &token, &owner, 100);

    // Pay fee
    client.pay_fee(&owner, &project_id, &Some(token.clone()));
    assert!(client.is_fee_paid(&project_id));

    // Admin cancels
    client.cancel_fee_payment(&admin, &project_id);

    // Verify fee is cancelled and refunded to owner
    let token_client = soroban_sdk::token::Client::new(&env, &token);
    assert!(!client.is_fee_paid(&project_id));
    assert_eq!(token_client.balance(&owner), 100);
}

#[test]
fn test_unauthorized_cancellation_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, owner, token) = setup(&env);
    let project_id = register(&client, &env, &owner, "UnauthorizedCancel");
    mint(&env, &token, &owner, 100);

    // Pay fee
    client.pay_fee(&owner, &project_id, &Some(token.clone()));

    let stranger = Address::generate(&env);
    let result = client.try_cancel_fee_payment(&stranger, &project_id);
    assert_eq!(result, Err(Ok(ContractError::Unauthorized)));
}

#[test]
fn test_cancel_unpaid_fee_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, owner, _token) = setup(&env);
    let project_id = register(&client, &env, &owner, "UnpaidCancel");

    let result = client.try_cancel_fee_payment(&owner, &project_id);
    assert_eq!(result, Err(Ok(ContractError::InsufficientFee)));
}

#[test]
fn test_cancel_consumed_fee_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, owner, token) = setup(&env);
    let project_id = register(&client, &env, &owner, "ConsumedCancel");
    mint(&env, &token, &owner, 100);

    // Pay fee
    client.pay_fee(&owner, &project_id, &Some(token.clone()));
    // Consume fee by requesting verification
    client.request_verification(&project_id, &owner, &String::from_str(&env, VALID_EVIDENCE_CID));

    // Attempt cancellation after consumption
    let result = client.try_cancel_fee_payment(&owner, &project_id);
    assert_eq!(result, Err(Ok(ContractError::InsufficientFee)));
}
