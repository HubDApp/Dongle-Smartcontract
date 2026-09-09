//! Targeted tests for Issue #624 — Verification Expiry Check Automation.

#![cfg(test)]

use crate::tests::fixtures::setup_contract;
use crate::types::{ProjectRegistrationParams, VerificationStatus};
use soroban_sdk::{testutils::Address as _, testutils::Ledger as _, Address, Env, String};

fn register_test_project(
    client: &crate::DongleContractClient<'_>,
    env: &Env,
    owner: &Address,
    slug: &str,
) -> u64 {
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(env, slug),
        slug: String::from_str(env, slug),
        description: String::from_str(env, "Test project for verification expiry"),
        category: String::from_str(env, "DeFi"),
        website: None,
        license: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
        repository_url: None,
    };
    client.register_project(&params)
}

#[test]
fn test_verification_expiry_boundary_conditions() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    // 1. Set verification duration to 600 seconds
    let duration = 600u64;
    client.set_verification_duration(&admin, &duration);

    // 2. Register project and request + approve verification at ledger timestamp 1000
    env.ledger().set_timestamp(1000);
    let project_id = register_test_project(&client, &env, &owner, "proj-expiry-test");
    let evidence_cid = String::from_str(&env, "QmXoypizjW3WknFiJnKLwHCnL72vedxjQkDDP1mXWo6uco");
    client.request_verification(&project_id, &owner, &evidence_cid);
    client.approve_verification(&project_id, &admin);

    let rec = client.get_verification(&project_id).unwrap();
    let expected_expires_at = 1000 + duration; // 1600
    assert_eq!(rec.expires_at, expected_expires_at);

    // 3. Before expiry: timestamp 1599 (< 1600)
    env.ledger().set_timestamp(1599);
    assert!(!client.is_verification_expired(&project_id));
    assert!(client.is_verification_active(&project_id));
    assert_eq!(
        client.get_project(&project_id).unwrap().verification_status,
        VerificationStatus::Verified
    );

    // 4. Exact boundary expiry timestamp: timestamp 1600 (== 1600)
    env.ledger().set_timestamp(1600);
    assert!(client.is_verification_expired(&project_id));
    assert!(!client.is_verification_active(&project_id));

    // 5. After expiry timestamp: timestamp 1601 (> 1600)
    env.ledger().set_timestamp(1601);
    assert!(client.is_verification_expired(&project_id));
    assert!(!client.is_verification_active(&project_id));
}

#[test]
fn test_process_verification_expiry_state_transition_and_idempotency() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    client.set_verification_duration(&admin, &500);

    env.ledger().set_timestamp(1000);
    let project_id = register_test_project(&client, &env, &owner, "proj-proc-expiry");
    let evidence_cid = String::from_str(&env, "QmXoypizjW3WknFiJnKLwHCnL72vedxjQkDDP1mXWo6uco");
    client.request_verification(&project_id, &owner, &evidence_cid);
    client.approve_verification(&project_id, &admin);

    // Advance timestamp past expires_at (expires at 1500)
    env.ledger().set_timestamp(1501);

    // Process expiry explicitly
    let transitioned = client.process_verification_expiry(&project_id);
    assert!(transitioned);

    // Verify stored states updated to Unverified
    let proj = client.get_project(&project_id).unwrap();
    assert_eq!(proj.verification_status, VerificationStatus::Unverified);
    let rec = client.get_verification(&project_id).unwrap();
    assert_eq!(rec.status, VerificationStatus::Unverified);

    // Idempotent retry: calling again on already unverified project returns false
    let transitioned_again = client.process_verification_expiry(&project_id);
    assert!(!transitioned_again);
}

#[test]
fn test_request_verification_auto_processes_expired_status() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    client.set_verification_duration(&admin, &500);

    env.ledger().set_timestamp(1000);
    let project_id = register_test_project(&client, &env, &owner, "proj-re-request");
    let evidence1 = String::from_str(&env, "QmXoypizjW3WknFiJnKLwHCnL72vedxjQkDDP1mXWo6uco");
    client.request_verification(&project_id, &owner, &evidence1);
    client.approve_verification(&project_id, &admin);

    // Advance timestamp to 1600 (expired)
    env.ledger().set_timestamp(1600);

    // Request verification again — should auto-process expiry and succeed in setting status to Pending
    let evidence2 = String::from_str(&env, "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG");
    client.request_verification(&project_id, &owner, &evidence2);

    let proj = client.get_project(&project_id).unwrap();
    assert_eq!(proj.verification_status, VerificationStatus::Pending);
}
