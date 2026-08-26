//! Integration test for the full verification lifecycle (issue #488).
//!
//! Covers the complete end-to-end flow:
//!   register_project → pay_fee → request_verification → approve_verification
//!   → verify expiry → request_renewal → approve_renewal

#![cfg(test)]

use crate::errors::ContractError;
use crate::tests::fixtures::setup_contract;
use crate::types::{ProjectRegistrationParams, VerificationStatus};
use soroban_sdk::{testutils::Address as _, testutils::Ledger as _, Address, Env, String};

/// Helper: register a minimal project and return its id.
fn register_project(
    client: &crate::DongleContractClient<'_>,
    env: &Env,
    owner: &Address,
    slug: &str,
) -> u64 {
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(env, slug),
        slug: String::from_str(env, slug),
        description: String::from_str(env, "Integration test project description"),
        category: String::from_str(env, "DeFi"),
        website: None,
        license: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
    };
    client.register_project(&params)
}

/// Helper: configure a fee token, mint tokens to `payer`, and pay the
/// verification fee for `project_id`.  Returns the token address used.
fn setup_and_pay_fee(
    client: &crate::DongleContractClient<'_>,
    env: &Env,
    admin: &Address,
    payer: &Address,
    project_id: u64,
) -> Address {
    let token_admin = Address::generate(env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();

    // Set verification fee to 100, registration fee to 0
    client.set_fee(admin, &Some(token_address.clone()), &100u128, &0u128, admin);

    // Mint tokens to payer and pay the fee
    let token_client = soroban_sdk::token::StellarAssetClient::new(env, &token_address);
    token_client.mint(payer, &1_000i128);
    client.pay_fee(payer, &project_id, &Some(token_address.clone()));

    token_address
}

// ---------------------------------------------------------------------------
// Full happy-path lifecycle
// ---------------------------------------------------------------------------

#[test]
fn test_full_verification_lifecycle_with_fee() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000);

    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    // ── Step 1: Register a project ──────────────────────────────────────
    let project_id = register_project(&client, &env, &owner, "lifecycle-project");

    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.verification_status, VerificationStatus::Unverified);

    // ── Step 2: Pay the verification fee ────────────────────────────────
    let fee_token = setup_and_pay_fee(&client, &env, &admin, &owner, project_id);
    assert!(client.is_fee_paid(&project_id));

    // ── Step 3: Request verification ────────────────────────────────────
    let evidence_cid = String::from_str(&env, "QmYwAPJhy5nTAQCj9g1s2bkss7jBlEd22bN2R4s5gR5PTa");
    client.request_verification(&project_id, &owner, &evidence_cid);

    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.verification_status, VerificationStatus::Pending);

    let record = client.get_verification(&project_id).unwrap();
    assert_eq!(record.status, VerificationStatus::Pending);
    assert_eq!(record.evidence_cid, evidence_cid);
    assert_eq!(record.project_id, project_id);

    // ── Step 4: Approve verification ────────────────────────────────────
    // Set a short verification duration so we can test expiry easily.
    client.set_verification_duration(&admin, &1_000u64); // 1 000 s
    client.approve_verification(&project_id, &admin);

    let record = client.get_verification(&project_id).unwrap();
    assert_eq!(record.status, VerificationStatus::Verified);
    assert!(
        record.expires_at > 0,
        "expires_at must be set after approval"
    );
    // expires_at = 1_000 (now) + 1_000 (duration) = 2_000
    assert_eq!(record.expires_at, 2_000);

    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.verification_status, VerificationStatus::Verified);

    // is_verification_active returns true while not expired
    assert!(client.is_verification_active(&project_id));

    // ── Step 5: Verify expiry behaviour ─────────────────────────────────
    // Still valid at timestamp 1_500 (before expiry)
    env.ledger().set_timestamp(1_500);
    assert!(!client.is_verification_expired(&project_id));
    assert!(client.is_verification_active(&project_id));

    // Expiry threshold helper: 600 s remaining → expiring soon within 700 s
    assert!(client.is_verification_expiring_soon(&project_id, &700u64));
    // But not "expiring soon" within 400 s
    assert!(!client.is_verification_expiring_soon(&project_id, &400u64));

    // Advance past expiry
    env.ledger().set_timestamp(2_001);
    assert!(client.is_verification_expired(&project_id));

    // ── Step 6: Request renewal ─────────────────────────────────────────
    // A renewal opens a new verification cycle and consumes a fee of its own —
    // the original payment was consumed by `request_verification` in step 3.
    // Without this the call fails with `InsufficientFee`.
    client.pay_fee(&owner, &project_id, &Some(fee_token.clone()));
    assert!(client.is_fee_paid(&project_id));

    let renewal_cid = String::from_str(&env, "QmYwAPJhy5nTAQCj9g1s2bkss7jBlEd22bN2R4s5gR5PTb");
    client.request_renewal(&project_id, &owner, &renewal_cid);

    let renewal = client.get_renewal_request(&project_id).unwrap();
    assert_eq!(renewal.project_id, project_id);
    assert_eq!(renewal.requester, owner);

    // Verification status remains Verified even after renewal is requested
    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.verification_status, VerificationStatus::Verified);

    // ── Step 7: Approve renewal ─────────────────────────────────────────
    env.ledger().set_timestamp(2_100);
    client.approve_renewal(&project_id, &admin);

    let record = client.get_verification(&project_id).unwrap();
    assert_eq!(record.status, VerificationStatus::Verified);
    // New expiry = 2_100 + 1_000 = 3_100
    assert_eq!(record.expires_at, 3_100);
    assert!(record.last_renewed_at > 0);

    // After renewal, the verification is no longer expired
    assert!(!client.is_verification_expired(&project_id));

    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.verification_status, VerificationStatus::Verified);

    // Renewal history should contain one entry
    let history = client.get_renewal_history(&project_id, &0, &10);
    assert_eq!(history.len(), 1);
    assert_eq!(history.get(0).unwrap().status, VerificationStatus::Verified);
}

// ---------------------------------------------------------------------------
// Fee gate: request_verification blocked when fee not paid
// ---------------------------------------------------------------------------

#[test]
fn test_request_verification_blocked_without_fee() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    // Register project and configure fee, but do NOT pay it
    let project_id = register_project(&client, &env, &owner, "no-fee-project");

    let token_admin = Address::generate(&env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    client.set_fee(&admin, &Some(token_address), &100u128, &0u128, &admin);

    let evidence_cid = String::from_str(&env, "QmYwAPJhy5nTAQCj9g1s2bkss7jBlEd22bN2R4s5gR5PTa");
    let result = client.try_request_verification(&project_id, &owner, &evidence_cid);
    assert!(
        result.is_err(),
        "verification should be blocked without fee payment"
    );
}

// ---------------------------------------------------------------------------
// Verification status flows through history correctly
// ---------------------------------------------------------------------------

#[test]
fn test_verification_history_recorded_across_lifecycle() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let project_id = register_project(&client, &env, &owner, "history-project");
    let evidence_cid = String::from_str(&env, "QmYwAPJhy5nTAQCj9g1s2bkss7jBlEd22bN2R4s5gR5PTa");

    // First cycle: approve then revoke
    client.request_verification(&project_id, &owner, &evidence_cid);
    client.approve_verification(&project_id, &admin);
    let revoke_reason = String::from_str(&env, "compliance issue");
    client.revoke_verification(&project_id, &admin, &revoke_reason);

    // Second cycle: approve again
    let evidence2 = String::from_str(&env, "QmYwAPJhy5nTAQCj9g1s2bkss7jBlEd22bN2R4s5gR5PTb");
    client.request_verification(&project_id, &owner, &evidence2);
    client.approve_verification(&project_id, &admin);

    let history = client.get_verification_history(&project_id);
    assert_eq!(history.len(), 2, "two verification records should exist");
    assert_eq!(history.get(0).unwrap().evidence_cid, evidence_cid);
    assert_eq!(history.get(1).unwrap().evidence_cid, evidence2);
    assert_eq!(history.get(1).unwrap().status, VerificationStatus::Verified);
}

// ---------------------------------------------------------------------------
// Renewal is rejected when requested on an unverified project
// ---------------------------------------------------------------------------

#[test]
fn test_renewal_requires_verified_status() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let project_id = register_project(&client, &env, &owner, "unverified-renewal");
    let evidence_cid = String::from_str(&env, "QmYwAPJhy5nTAQCj9g1s2bkss7jBlEd22bN2R4s5gR5PTa");

    // Project is still Unverified; renewal must fail
    let result = client.try_request_renewal(&project_id, &owner, &evidence_cid);
    assert!(
        result.is_err(),
        "renewal must be rejected for unverified project"
    );

    // Request and reject verification → Rejected state
    client.request_verification(&project_id, &owner, &evidence_cid);
    client.reject_verification(&project_id, &admin);

    let result = client.try_request_renewal(&project_id, &owner, &evidence_cid);
    assert!(
        result.is_err(),
        "renewal must be rejected when verification was rejected"
    );
}
