//! Tests for verification request replacement rules (issue #225).
//!
//! A re-request after rejection or revocation must **version**, not
//! overwrite: the previous `VerificationRecord` keeps its original status
//! and evidence untouched, the new request gets its own record and id, and
//! `get_verification_history` returns both.

#![cfg(test)]

use crate::errors::ContractError;
use crate::tests::fixtures::setup_contract;
use crate::types::{ProjectRegistrationParams, VerificationStatus};
use soroban_sdk::{testutils::Address as _, Address, Env, String};

fn register_project(client: &crate::DongleContractClient<'_>, env: &Env, owner: &Address) -> u64 {
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(env, "replacement-project"),
        slug: String::from_str(env, "replacement-project"),
        description: String::from_str(env, "Verification replacement test project"),
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
fn test_rerequest_after_rejection_preserves_previous_record() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = register_project(&client, &env, &owner);

    let first_cid = String::from_str(&env, "QmYwAPJzv5CZsnAzt8auVZRnG8X1sC3yRyvCb4s46HoPa1");
    client.request_verification(&project_id, &owner, &first_cid);

    let first_record = client.get_verification(&project_id).unwrap();
    assert_eq!(first_record.status, VerificationStatus::Pending);
    assert_eq!(first_record.evidence_cid, first_cid);
    let first_request_id = first_record.request_id;

    client.reject_verification(&project_id, &admin);

    let rejected_record = client.get_verification_record(&first_request_id).unwrap();
    assert_eq!(rejected_record.status, VerificationStatus::Rejected);
    assert_eq!(rejected_record.evidence_cid, first_cid);

    // Re-request with new evidence — this must NOT mutate the rejected record.
    let second_cid = String::from_str(&env, "QmYwAPJzv5CZsnAzt8auVZRnG8X1sC3yRyvCb4s46HoPa2");
    client.request_verification(&project_id, &owner, &second_cid);

    // The old record is untouched.
    let rejected_record_after = client.get_verification_record(&first_request_id).unwrap();
    assert_eq!(rejected_record_after.status, VerificationStatus::Rejected);
    assert_eq!(rejected_record_after.evidence_cid, first_cid);

    // The new record is separate, with its own id and evidence.
    let current_record = client.get_verification(&project_id).unwrap();
    assert_ne!(current_record.request_id, first_request_id);
    assert_eq!(current_record.status, VerificationStatus::Pending);
    assert_eq!(current_record.evidence_cid, second_cid);

    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.verification_status, VerificationStatus::Pending);
    assert_eq!(
        project.current_verification_id,
        Some(current_record.request_id)
    );

    // History contains both records, oldest first.
    let history = client.get_verification_history(&project_id);
    assert_eq!(history.len(), 2);
    assert_eq!(history.get(0).unwrap().request_id, first_request_id);
    assert_eq!(history.get(0).unwrap().status, VerificationStatus::Rejected);
    assert_eq!(history.get(0).unwrap().evidence_cid, first_cid);
    assert_eq!(
        history.get(1).unwrap().request_id,
        current_record.request_id
    );
    assert_eq!(history.get(1).unwrap().status, VerificationStatus::Pending);
    assert_eq!(history.get(1).unwrap().evidence_cid, second_cid);
}

#[test]
fn test_rerequest_after_revocation_preserves_previous_record() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = register_project(&client, &env, &owner);

    let first_cid = String::from_str(&env, "QmYwAPJzv5CZsnAzt8auVZRnG8X1sC3yRyvCb4s46HoPa3");
    client.request_verification(&project_id, &owner, &first_cid);
    client.approve_verification(&project_id, &admin);

    let approved_record = client.get_verification(&project_id).unwrap();
    let approved_request_id = approved_record.request_id;
    assert_eq!(approved_record.status, VerificationStatus::Verified);

    let reason = String::from_str(&env, "Evidence no longer valid");
    client.revoke_verification(&project_id, &admin, &reason);

    let revoked_record = client
        .get_verification_record(&approved_request_id)
        .unwrap();
    assert_eq!(revoked_record.status, VerificationStatus::Unverified);
    assert_eq!(revoked_record.revoke_reason, Some(reason));
    assert_eq!(revoked_record.evidence_cid, first_cid);

    let second_cid = String::from_str(&env, "QmYwAPJzv5CZsnAzt8auVZRnG8X1sC3yRyvCb4s46HoPa4");
    client.request_verification(&project_id, &owner, &second_cid);

    // The revoked record keeps its revoke reason and original evidence.
    let revoked_record_after = client
        .get_verification_record(&approved_request_id)
        .unwrap();
    assert_eq!(revoked_record_after.status, VerificationStatus::Unverified);
    assert!(revoked_record_after.revoke_reason.is_some());
    assert_eq!(revoked_record_after.evidence_cid, first_cid);

    let current_record = client.get_verification(&project_id).unwrap();
    assert_ne!(current_record.request_id, approved_request_id);
    assert_eq!(current_record.evidence_cid, second_cid);
    assert_eq!(current_record.status, VerificationStatus::Pending);

    let history = client.get_verification_history(&project_id);
    assert_eq!(history.len(), 2);
}

#[test]
fn test_rerequest_ids_are_sequential() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = register_project(&client, &env, &owner);

    let first_cid = String::from_str(&env, "QmYwAPJzv5CZsnAzt8auVZRnG8X1sC3yRyvCb4s46HoPa5");
    client.request_verification(&project_id, &owner, &first_cid);
    let first_record = client.get_verification(&project_id).unwrap();

    client.reject_verification(&project_id, &admin);

    let second_cid = String::from_str(&env, "QmYwAPJzv5CZsnAzt8auVZRnG8X1sC3yRyvCb4s46HoPa6");
    client.request_verification(&project_id, &owner, &second_cid);
    let second_record = client.get_verification(&project_id).unwrap();

    // Re-requests always get a fresh, later request id — never reuse or
    // overwrite the previous request's id.
    assert_eq!(second_record.request_id, first_record.request_id + 1);
}

#[test]
fn test_rerequest_rejected_with_no_new_request_error() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = register_project(&client, &env, &owner);

    let first_cid = String::from_str(&env, "QmYwAPJzv5CZsnAzt8auVZRnG8X1sC3yRyvCb4s46HoPa7");
    client.request_verification(&project_id, &owner, &first_cid);
    client.reject_verification(&project_id, &admin);

    // A rejected project must be allowed to submit a fresh request.
    let second_cid = String::from_str(&env, "QmYwAPJzv5CZsnAzt8auVZRnG8X1sC3yRyvCb4s46HoPa8");
    let result = client.try_request_verification(&project_id, &owner, &second_cid);
    assert!(result.is_ok());

    // But while a request is Pending, another request is rejected.
    let third_cid = String::from_str(&env, "QmYwAPJzv5CZsnAzt8auVZRnG8X1sC3yRyvCb4s46HoPa9");
    let result = client.try_request_verification(&project_id, &owner, &third_cid);
    assert_eq!(result, Err(Ok(ContractError::InvalidStatus)));
}
