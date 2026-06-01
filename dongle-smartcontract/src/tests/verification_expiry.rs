//! Tests for verification expiry: active, expired, and renewed verification.

use crate::errors::ContractError;
use crate::types::{ProjectRegistrationParams, VerificationStatus};
use crate::DongleContract;
use crate::DongleContractClient;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn setup(env: &Env) -> (DongleContractClient<'_>, Address, Address) {
    let contract_id = env.register(DongleContract, ());
    let client = DongleContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.initialize(&admin);
    (client, admin, Address::generate(env))
}

/// Register a project, configure a fee token, mint tokens, pay the fee, and
/// request verification. Returns the project_id.
fn setup_verified_project(
    client: &DongleContractClient<'_>,
    env: &Env,
    admin: &Address,
    owner: &Address,
    name: &str,
) -> u64 {
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(env, name),
        description: String::from_str(env, "Test description for expiry tests"),
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
    let token_client = soroban_sdk::token::StellarAssetClient::new(env, &token_address);
    token_client.mint(owner, &1000);
    client.set_fee(admin, &Some(token_address.clone()), &100, admin);
    client.pay_fee(owner, &project_id, &Some(token_address));
    client.request_verification(&project_id, owner, &String::from_str(env, "ipfs://evidence"));
    client.approve_verification(&project_id, admin);

    project_id
}

/// Pay the verification fee again for an existing project (for renewal).
fn pay_fee_again(
    client: &DongleContractClient<'_>,
    env: &Env,
    admin: &Address,
    owner: &Address,
    project_id: u64,
) {
    let token_admin = Address::generate(env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    let token_client = soroban_sdk::token::StellarAssetClient::new(env, &token_address);
    token_client.mint(owner, &1000);
    client.set_fee(admin, &Some(token_address.clone()), &100, admin);
    client.pay_fee(owner, &project_id, &Some(token_address));
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// After approval the record carries an expires_at and is_verification_active returns true.
#[test]
fn test_active_verification() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, owner) = setup(&env);

    let project_id = setup_verified_project(&client, &env, &admin, &owner, "Active Project");

    // is_verification_active should be true immediately after approval
    assert!(client.is_verification_active(&project_id));

    // The record should have an expires_at set
    let record = client.get_verification(&project_id);
    assert_eq!(record.status, VerificationStatus::Verified);
    assert!(record.expires_at.is_some());
}

/// After the ledger timestamp advances past expires_at, is_verification_active returns false.
#[test]
fn test_expired_verification() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, owner) = setup(&env);

    // Set a very short duration: 100 seconds
    client.set_verification_duration(&admin, &Some(100u64));

    let project_id = setup_verified_project(&client, &env, &admin, &owner, "Expiring Project");

    // Advance ledger time past the expiry
    env.ledger().with_mut(|li| {
        li.timestamp += 200; // 200 seconds later
    });

    assert!(!client.is_verification_active(&project_id));

    // The record status is still Verified (expiry is checked at read time, not written back)
    let record = client.get_verification(&project_id);
    assert_eq!(record.status, VerificationStatus::Verified);
}

/// After expiry, renewing the verification extends expires_at and makes it active again.
#[test]
fn test_renew_verification() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, owner) = setup(&env);

    // Short duration so we can expire it quickly
    client.set_verification_duration(&admin, &Some(100u64));

    let project_id = setup_verified_project(&client, &env, &admin, &owner, "Renewable Project");

    // Expire it
    env.ledger().with_mut(|li| {
        li.timestamp += 200;
    });
    assert!(!client.is_verification_active(&project_id));

    // Pay fee and renew
    pay_fee_again(&client, &env, &admin, &owner, project_id);
    client.renew_verification(&project_id, &owner);

    // Should be active again
    assert!(client.is_verification_active(&project_id));

    // expires_at should be updated
    let record = client.get_verification(&project_id);
    let now = env.ledger().timestamp();
    let expires_at = record.expires_at.unwrap();
    assert!(expires_at > now);
}

/// Admin can disable expiry (duration = None); verification never expires.
#[test]
fn test_no_expiry_when_duration_is_none() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, owner) = setup(&env);

    // Disable expiry
    client.set_verification_duration(&admin, &None);

    let project_id = setup_verified_project(&client, &env, &admin, &owner, "No Expiry Project");

    // Advance time by a very large amount
    env.ledger().with_mut(|li| {
        li.timestamp += 10_000_000;
    });

    assert!(client.is_verification_active(&project_id));

    let record = client.get_verification(&project_id);
    assert!(record.expires_at.is_none());
}

/// get_verification_duration returns the configured value.
#[test]
fn test_get_verification_duration() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, _owner) = setup(&env);

    // Default should be 365 days in seconds
    let default_duration = client.get_verification_duration();
    assert_eq!(default_duration, Some(365 * 24 * 60 * 60));

    // Set a custom duration
    client.set_verification_duration(&admin, &Some(7200u64));
    assert_eq!(client.get_verification_duration(), Some(7200u64));

    // Disable expiry
    client.set_verification_duration(&admin, &None);
    assert_eq!(client.get_verification_duration(), None);
}

/// Only admin can call set_verification_duration.
#[test]
fn test_set_verification_duration_non_admin_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, _owner) = setup(&env);

    let non_admin = Address::generate(&env);
    let result = client.try_set_verification_duration(&non_admin, &Some(3600u64));
    assert_eq!(result, Err(Ok(ContractError::AdminOnly)));
}

/// renew_verification fails if the project is not in Verified status.
#[test]
fn test_renew_non_verified_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, owner) = setup(&env);

    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "Pending Project"),
        description: String::from_str(&env, "Test description"),
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
    client.request_verification(&project_id, &owner, &String::from_str(&env, "ipfs://ev"));

    // Project is Pending, not Verified — renew should fail
    // Pay fee first so we don't hit InsufficientFee
    let token_admin2 = Address::generate(&env);
    let token_address2 = env
        .register_stellar_asset_contract_v2(token_admin2)
        .address();
    let token_client2 = soroban_sdk::token::StellarAssetClient::new(&env, &token_address2);
    token_client2.mint(&owner, &1000);
    client.set_fee(&admin, &Some(token_address2.clone()), &100, &admin);
    client.pay_fee(&owner, &project_id, &Some(token_address2));

    let result = client.try_renew_verification(&project_id, &owner);
    assert_eq!(result, Err(Ok(ContractError::InvalidStatusTransition)));
}

/// is_verification_active returns false for an unverified project.
#[test]
fn test_is_active_unverified_project() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, owner) = setup(&env);

    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "Unverified Project"),
        description: String::from_str(&env, "Test description"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
    };
    let project_id = client.register_project(&params);

    assert!(!client.is_verification_active(&project_id));
}
