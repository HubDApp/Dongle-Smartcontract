//! Regression tests for the canonical [`ClaimStatus`] enum shared by ownership
//! and contract-address claim workflows (issue #357).

use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::{ClaimRequest, ClaimStatus, ContractClaimRequest};
use soroban_sdk::{testutils::Address as _, Address, Env, String};

// ── ClaimStatus state-machine unit tests ─────────────────────────────────────

#[test]
fn claim_status_pending_transitions_to_approved() {
    let mut status = ClaimStatus::Pending;
    assert!(status.transition_to_approved().is_ok());
    assert_eq!(status, ClaimStatus::Approved);
}

#[test]
fn claim_status_pending_transitions_to_rejected() {
    let mut status = ClaimStatus::Pending;
    assert!(status.transition_to_rejected().is_ok());
    assert_eq!(status, ClaimStatus::Rejected);
}

#[test]
fn claim_status_double_approve_returns_invalid_status() {
    let mut status = ClaimStatus::Approved;
    let err = status.transition_to_approved().unwrap_err();
    assert_eq!(err, ContractError::InvalidStatus);
}

#[test]
fn claim_status_double_reject_returns_invalid_status() {
    let mut status = ClaimStatus::Rejected;
    let err = status.transition_to_rejected().unwrap_err();
    assert_eq!(err, ContractError::InvalidStatus);
}

#[test]
fn claim_status_approved_cannot_transition_to_rejected() {
    let mut status = ClaimStatus::Approved;
    let err = status.transition_to_rejected().unwrap_err();
    assert_eq!(err, ContractError::InvalidStatus);
}

#[test]
fn claim_status_rejected_cannot_transition_to_approved() {
    let mut status = ClaimStatus::Rejected;
    let err = status.transition_to_approved().unwrap_err();
    assert_eq!(err, ContractError::InvalidStatus);
}

#[test]
fn claim_request_and_contract_claim_request_share_status_type() {
    // Compile-time guarantee both workflows store the same status enum.
    let ownership: ClaimRequest = ClaimRequest {
        id: 1,
        project_id: 1,
        claimant: Address::generate(&Env::default()),
        proof_cid: String::from_str(&Env::default(), "QmProof"),
        status: ClaimStatus::Pending,
        created_at: 0,
    };
    let contract: ContractClaimRequest = ContractClaimRequest {
        project_id: 1,
        contract_address: String::from_str(&Env::default(), "CADDR"),
        claimant: Address::generate(&Env::default()),
        proof_cid: String::from_str(&Env::default(), "QmProof"),
        status: ClaimStatus::Pending,
        created_at: 0,
    };
    assert_eq!(ownership.status, ClaimStatus::Pending);
    assert_eq!(contract.status, ClaimStatus::Pending);
}

// ── Integration: both workflows use ClaimStatus end-to-end ─────────────────

#[test]
fn ownership_claim_workflow_uses_shared_claim_status() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let claimant = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "OwnershipClaim");

    client
        .mock_all_auths()
        .set_project_claimable(&project_id, &owner, &true);
    let proof = String::from_str(&env, "QmOwnershipProof");
    let claim_id = client
        .mock_all_auths()
        .submit_claim_request(&project_id, &claimant, &proof);

    assert_eq!(
        client.get_claim_request(&claim_id).unwrap().status,
        ClaimStatus::Pending
    );

    client
        .mock_all_auths()
        .approve_claim_request(&claim_id, &admin);
    assert_eq!(
        client.get_claim_request(&claim_id).unwrap().status,
        ClaimStatus::Approved
    );
}

#[test]
fn contract_address_claim_workflow_uses_shared_claim_status() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "ContractClaim");

    let contract_addr = String::from_str(
        &env,
        "CDLZFC3SYJYDZT7K67VZ75HPJVIEWBE6YAAH2PBNU6K4R457OT7KMBM4",
    );
    let proof = String::from_str(&env, "QmContractProof123456789012345678901234567890");

    let submitted =
        client.claim_contract_address(&project_id, &owner, &contract_addr, &proof);
    assert_eq!(submitted.status, ClaimStatus::Pending);

    let rejected = client.reject_contract_claim(&project_id, &contract_addr, &admin);
    assert_eq!(rejected.status, ClaimStatus::Rejected);

    let resubmitted =
        client.claim_contract_address(&project_id, &owner, &contract_addr, &proof);
    assert_eq!(resubmitted.status, ClaimStatus::Pending);

    let approved = client.approve_contract_claim(&project_id, &contract_addr, &admin);
    assert_eq!(approved.status, ClaimStatus::Approved);
}
