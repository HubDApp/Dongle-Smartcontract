//! Verification renewal tests: request, approve, reject, expiry, and history.

use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::DongleContractClient;
use soroban_sdk::{testutils::Address as _, testutils::Ledger as _, Address, Env, String};

fn setup(env: &Env) -> (DongleContractClient<'_>, Address) {
    setup_contract(env)
}

// ---------------------------------------------------------------------------
// request_renewal
// ---------------------------------------------------------------------------

#[test]
fn test_request_renewal_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectA");

    let owner = admin.clone();
    let evidence_cid = String::from_str(&env, "QmTestEvidenceCid123456789012345678901234567890");

    // First verify the project
    client.request_verification(&project_id, &owner, &evidence_cid);
    client.approve_verification(&project_id, &admin);

    // Now request renewal
    let result = client.try_request_renewal(&project_id, &owner, &evidence_cid);
    assert!(result.is_ok());

    let renewal = client.get_renewal_request(&project_id);
    assert_eq!(renewal.project_id, project_id);
    assert_eq!(renewal.requester, owner);
}

#[test]
fn test_request_renewal_unverified_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectB");

    let owner = admin.clone();
    let evidence_cid = String::from_str(&env, "QmTestEvidenceCid123456789012345678901234567890");

    // Try to renew without verification
    let result = client.try_request_renewal(&project_id, &owner, &evidence_cid);
    assert!(result.is_err());
}

#[test]
fn test_request_renewal_duplicate_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectC");

    let owner = admin.clone();
    let evidence_cid = String::from_str(&env, "QmTestEvidenceCid123456789012345678901234567890");

    // Verify the project
    client.request_verification(&project_id, &owner, &evidence_cid);
    client.approve_verification(&project_id, &admin);

    // Request renewal
    let _result = client.try_request_renewal(&project_id, &owner, &evidence_cid);

    // Try to request renewal again
    let result = client.try_request_renewal(&project_id, &owner, &evidence_cid);
    assert!(result.is_err());
}

#[test]
fn test_request_renewal_not_owner_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectD");

    let owner = admin.clone();
    let not_owner = Address::generate(&env);
    let evidence_cid = String::from_str(&env, "QmTestEvidenceCid123456789012345678901234567890");

    // Verify the project
    client.request_verification(&project_id, &owner, &evidence_cid);
    client.approve_verification(&project_id, &admin);

    // Try to renew as non-owner
    let result = client.try_request_renewal(&project_id, &not_owner, &evidence_cid);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// approve_renewal
// ---------------------------------------------------------------------------

#[test]
fn test_approve_renewal_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectE");

    let owner = admin.clone();
    let evidence_cid = String::from_str(&env, "QmTestEvidenceCid123456789012345678901234567890");

    // Verify the project
    client.request_verification(&project_id, &owner, &evidence_cid);
    client.approve_verification(&project_id, &admin);

    // Request renewal
    let _result = client.try_request_renewal(&project_id, &owner, &evidence_cid);

    // Approve renewal
    let result = client.try_approve_renewal(&project_id, &admin);
    assert!(result.is_ok());

    // Renewal should be gone (approved)
    let result = client.try_get_renewal_request(&project_id);
    assert!(result.is_err());
}

#[test]
fn test_approve_renewal_updates_expiry() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectF");

    let owner = admin.clone();
    let evidence_cid = String::from_str(&env, "QmTestEvidenceCid123456789012345678901234567890");

    // Verify the project
    client.request_verification(&project_id, &owner, &evidence_cid);
    client.approve_verification(&project_id, &admin);

    let before_renewal = client.get_verification(&project_id);
    let before_expires = before_renewal.expires_at;

    // Advance ledger timestamp to ensure expiry increases
    env.ledger().set_timestamp(100);

    // Request and approve renewal
    let _result = client.try_request_renewal(&project_id, &owner, &evidence_cid);
    let _result = client.try_approve_renewal(&project_id, &admin);

    let after_renewal = client.get_verification(&project_id);
    let after_expires = after_renewal.expires_at;

    // Expiry should be updated
    assert!(after_expires > before_expires);
}

#[test]
fn test_approve_renewal_non_admin_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectG");

    let owner = admin.clone();
    let non_admin = Address::generate(&env);
    let evidence_cid = String::from_str(&env, "QmTestEvidenceCid123456789012345678901234567890");

    // Verify the project
    client.request_verification(&project_id, &owner, &evidence_cid);
    client.approve_verification(&project_id, &admin);

    // Request renewal
    let _result = client.try_request_renewal(&project_id, &owner, &evidence_cid);

    // Try to approve as non-admin
    let result = client.try_approve_renewal(&project_id, &non_admin);
    assert!(result.is_err());
}

#[test]
fn test_approve_renewal_not_found_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectH");

    let owner = admin.clone();
    let evidence_cid = String::from_str(&env, "QmTestEvidenceCid123456789012345678901234567890");

    // Verify the project
    client.request_verification(&project_id, &owner, &evidence_cid);
    client.approve_verification(&project_id, &admin);

    // Try to approve renewal without requesting
    let result = client.try_approve_renewal(&project_id, &admin);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// reject_renewal
// ---------------------------------------------------------------------------

#[test]
fn test_reject_renewal_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectI");

    let owner = admin.clone();
    let evidence_cid = String::from_str(&env, "QmTestEvidenceCid123456789012345678901234567890");

    // Verify the project
    client.request_verification(&project_id, &owner, &evidence_cid);
    client.approve_verification(&project_id, &admin);

    // Request renewal
    let _result = client.try_request_renewal(&project_id, &owner, &evidence_cid);

    // Reject renewal
    let result = client.try_reject_renewal(&project_id, &admin);
    assert!(result.is_ok());

    // Renewal should be gone
    let result = client.try_get_renewal_request(&project_id);
    assert!(result.is_err());

    // Verification should still be verified
    let verification = client.get_verification(&project_id);
    assert_eq!(verification.status, crate::types::VerificationStatus::Verified);
}

#[test]
fn test_reject_renewal_non_admin_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectJ");

    let owner = admin.clone();
    let non_admin = Address::generate(&env);
    let evidence_cid = String::from_str(&env, "QmTestEvidenceCid123456789012345678901234567890");

    // Verify the project
    client.request_verification(&project_id, &owner, &evidence_cid);
    client.approve_verification(&project_id, &admin);

    // Request renewal
    let _result = client.try_request_renewal(&project_id, &owner, &evidence_cid);

    // Try to reject as non-admin
    let result = client.try_reject_renewal(&project_id, &non_admin);
    assert!(result.is_err());
}

#[test]
fn test_reject_renewal_not_found_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectK");

    let owner = admin.clone();
    let evidence_cid = String::from_str(&env, "QmTestEvidenceCid123456789012345678901234567890");

    // Verify the project
    client.request_verification(&project_id, &owner, &evidence_cid);
    client.approve_verification(&project_id, &admin);

    // Try to reject renewal without requesting
    let result = client.try_reject_renewal(&project_id, &admin);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Renewal history
// ---------------------------------------------------------------------------

#[test]
fn test_renewal_history_single() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectL");

    let owner = admin.clone();
    let evidence_cid = String::from_str(&env, "QmTestEvidenceCid123456789012345678901234567890");

    // Verify the project
    client.request_verification(&project_id, &owner, &evidence_cid);
    client.approve_verification(&project_id, &admin);

    // Request and approve renewal
    let _result = client.try_request_renewal(&project_id, &owner, &evidence_cid);
    let _result = client.try_approve_renewal(&project_id, &admin);

    // Check history
    let history = client.get_renewal_history(&project_id, &0, &100);
    assert_eq!(history.len(), 1);
}

#[test]
fn test_renewal_history_multiple() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectM");

    let owner = admin.clone();
    let evidence_cid = String::from_str(&env, "QmTestEvidenceCid123456789012345678901234567890");

    // Verify the project
    client.request_verification(&project_id, &owner, &evidence_cid);
    client.approve_verification(&project_id, &admin);

    // Do multiple renewals
    for _ in 0..3 {
        let _result = client.try_request_renewal(&project_id, &owner, &evidence_cid);
        let _result = client.try_approve_renewal(&project_id, &admin);
    }

    // Check history
    let history = client.get_renewal_history(&project_id, &0, &100);
    assert_eq!(history.len(), 3);
}

#[test]
fn test_renewal_history_pagination() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectN");

    let owner = admin.clone();
    let evidence_cid = String::from_str(&env, "QmTestEvidenceCid123456789012345678901234567890");

    // Verify the project
    client.request_verification(&project_id, &owner, &evidence_cid);
    client.approve_verification(&project_id, &admin);

    // Do multiple renewals
    for _ in 0..5 {
        let _result = client.try_request_renewal(&project_id, &owner, &evidence_cid);
        let _result = client.try_approve_renewal(&project_id, &admin);
    }

    // Check pagination
    let page1 = client.get_renewal_history(&project_id, &0, &2);
    assert_eq!(page1.len(), 2);

    let page2 = client.get_renewal_history(&project_id, &2, &2);
    assert_eq!(page2.len(), 2);

    let page3 = client.get_renewal_history(&project_id, &4, &2);
    assert_eq!(page3.len(), 1);
}

// ---------------------------------------------------------------------------
// Expiry checking
// ---------------------------------------------------------------------------

#[test]
fn test_is_verification_expired_not_expired() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectO");

    let owner = admin.clone();
    let evidence_cid = String::from_str(&env, "QmTestEvidenceCid123456789012345678901234567890");

    // Verify the project
    client.request_verification(&project_id, &owner, &evidence_cid);
    client.approve_verification(&project_id, &admin);

    // Check expiry
    let is_expired = client.is_verification_expired(&project_id);
    assert_eq!(is_expired, false);
}

#[test]
fn test_is_verification_expired_no_expiry() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectP");

    let owner = admin.clone();
    let evidence_cid = String::from_str(&env, "QmTestEvidenceCid123456789012345678901234567890");

    // Verify the project (without renewal, expires_at = 0)
    client.request_verification(&project_id, &owner, &evidence_cid);
    client.approve_verification(&project_id, &admin);

    // Check expiry (should be false since expires_at = 0)
    let is_expired = client.is_verification_expired(&project_id);
    assert_eq!(is_expired, false);
}

// ---------------------------------------------------------------------------
// Complex scenarios
// ---------------------------------------------------------------------------

#[test]
fn test_renewal_after_rejection() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectQ");

    let owner = admin.clone();
    let evidence_cid = String::from_str(&env, "QmTestEvidenceCid123456789012345678901234567890");

    // Verify the project
    client.request_verification(&project_id, &owner, &evidence_cid);
    client.approve_verification(&project_id, &admin);

    // Request and reject renewal
    let _result = client.try_request_renewal(&project_id, &owner, &evidence_cid);
    let _result = client.try_reject_renewal(&project_id, &admin);

    // Should be able to request renewal again
    let result = client.try_request_renewal(&project_id, &owner, &evidence_cid);
    assert!(result.is_ok());

    let renewal = client.get_renewal_request(&project_id);
    assert_eq!(renewal.project_id, project_id);
}

#[test]
fn test_multiple_projects_independent_renewal() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project1 = create_test_project(&client, &admin, "ProjectR");
    let project2 = create_test_project(&client, &admin, "ProjectS");

    let owner = admin.clone();
    let evidence_cid = String::from_str(&env, "QmTestEvidenceCid123456789012345678901234567890");

    // Verify both projects
    client.request_verification(&project1, &owner, &evidence_cid);
    client.approve_verification(&project1, &admin);
    client.request_verification(&project2, &owner, &evidence_cid);
    client.approve_verification(&project2, &admin);

    // Request renewal for project1 only
    let _result = client.try_request_renewal(&project1, &owner, &evidence_cid);

    // Project1 should have renewal
    let renewal1 = client.get_renewal_request(&project1);
    assert_eq!(renewal1.project_id, project1);

    // Project2 should not have renewal
    let result2 = client.try_get_renewal_request(&project2);
    assert!(result2.is_err());
}

#[test]
fn test_renewal_preserves_verification_status() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectT");

    let owner = admin.clone();
    let evidence_cid = String::from_str(&env, "QmTestEvidenceCid123456789012345678901234567890");

    // Verify the project
    client.request_verification(&project_id, &owner, &evidence_cid);
    client.approve_verification(&project_id, &admin);

    let before_renewal = client.get_verification(&project_id);
    assert_eq!(before_renewal.status, crate::types::VerificationStatus::Verified);

    // Request and approve renewal
    let _result = client.try_request_renewal(&project_id, &owner, &evidence_cid);
    let _result = client.try_approve_renewal(&project_id, &admin);

    let after_renewal = client.get_verification(&project_id);
    assert_eq!(after_renewal.status, crate::types::VerificationStatus::Verified);
}

#[test]
fn test_renewal_updates_last_renewed_at() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectU");

    let owner = admin.clone();
    let evidence_cid = String::from_str(&env, "QmTestEvidenceCid123456789012345678901234567890");

    // Verify the project
    client.request_verification(&project_id, &owner, &evidence_cid);
    client.approve_verification(&project_id, &admin);

    let before_renewal = client.get_verification(&project_id);
    let before_renewed_at = before_renewal.last_renewed_at;

    // Advance ledger timestamp to ensure last_renewed_at increases
    env.ledger().set_timestamp(100);

    // Request and approve renewal
    let _result = client.try_request_renewal(&project_id, &owner, &evidence_cid);
    let _result = client.try_approve_renewal(&project_id, &admin);

    let after_renewal = client.get_verification(&project_id);
    let after_renewed_at = after_renewal.last_renewed_at;

    // last_renewed_at should be updated
    assert!(after_renewed_at > before_renewed_at);
}

#[test]
fn test_request_renewal_after_expiry_success() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup(&env);
    let project_id = create_test_project(&client, &admin, "ProjectV");

    let owner = admin.clone();
    let evidence_cid = String::from_str(&env, "QmTestEvidenceCid123456789012345678901234567890");

    // Configure verification duration to 1000 seconds
    client.set_verification_duration(&admin, &1000);

    // Initial verification request and approval at timestamp 0
    client.request_verification(&project_id, &owner, &evidence_cid);
    client.approve_verification(&project_id, &admin);

    // Check it is verified and not expired at timestamp 500
    env.ledger().set_timestamp(500);
    assert_eq!(client.is_verification_expired(&project_id), false);

    // Advance to timestamp 1200, so it's expired
    env.ledger().set_timestamp(1200);
    assert_eq!(client.is_verification_expired(&project_id), true);

    // Request renewal should succeed even though verification is expired
    let result = client.try_request_renewal(&project_id, &owner, &evidence_cid);
    assert!(result.is_ok());

    let renewal = client.get_renewal_request(&project_id);
    assert_eq!(renewal.project_id, project_id);
    assert_eq!(renewal.requester, owner);

    // Approve renewal at timestamp 1300
    env.ledger().set_timestamp(1300);
    let approve_result = client.try_approve_renewal(&project_id, &admin);
    assert!(approve_result.is_ok());

    // Expiry should now be 2300 (1300 + 1000)
    let verification = client.get_verification(&project_id);
    assert_eq!(verification.expires_at, 2300);
    assert_eq!(verification.status, crate::types::VerificationStatus::Verified);

    // Should no longer be expired at timestamp 1400
    env.ledger().set_timestamp(1400);
    assert_eq!(client.is_verification_expired(&project_id), false);
}



