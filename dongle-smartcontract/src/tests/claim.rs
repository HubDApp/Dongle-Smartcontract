use crate::types::{ClaimRequest, ClaimStatus};
use crate::tests::fixtures::{setup_contract, create_test_project};
use soroban_sdk::{testutils::Address as _, Address, Env, String};

#[test]
fn test_set_project_claimable() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Test Project");

    // Check initial claimable state is false
    let project = client.get_project(&project_id).unwrap();
    assert!(!project.claimable);

    // Owner sets project as claimable
    client
        .mock_all_auths()
        .set_project_claimable(&project_id, &owner, &true);

    let project = client.get_project(&project_id).unwrap();
    assert!(project.claimable);

    // Owner sets project as not claimable
    client
        .mock_all_auths()
        .set_project_claimable(&project_id, &owner, &false);

    let project = client.get_project(&project_id).unwrap();
    assert!(!project.claimable);

    // Admin sets project as claimable
    client
        .mock_all_auths()
        .set_project_claimable(&project_id, &admin, &true);

    let project = client.get_project(&project_id).unwrap();
    assert!(project.claimable);
}

#[test]
fn test_submit_claim_request() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let claimant = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Test Project");

    // Try to submit claim when project is not claimable (should fail)
    let proof_cid = String::from_str(&env, "QmTestProofCid");
    let result = client
        .mock_all_auths()
        .try_submit_claim_request(&project_id, &claimant, &proof_cid);
    assert!(result.is_err());

    // Set project as claimable
    client
        .mock_all_auths()
        .set_project_claimable(&project_id, &owner, &true);

    // Submit claim request
    let claim_request_id = client
        .mock_all_auths()
        .submit_claim_request(&project_id, &claimant, &proof_cid);
    assert_eq!(claim_request_id, 1);

    // Check claim request
    let claim_request = client.get_claim_request(&claim_request_id).unwrap();
    assert_eq!(claim_request.project_id, project_id);
    assert_eq!(claim_request.claimant, claimant);
    assert_eq!(claim_request.proof_cid, proof_cid);
    assert_eq!(claim_request.status, ClaimStatus::Pending);

    // Check project's claim requests
    let claim_requests = client.get_claim_requests_for_project(&project_id);
    assert_eq!(claim_requests.len(), 1);
    assert_eq!(claim_requests.get(0).unwrap().id, claim_request_id);
}

#[test]
fn test_approve_claim_request() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let claimant = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Test Project");

    // Set project as claimable and submit claim
    client
        .mock_all_auths()
        .set_project_claimable(&project_id, &owner, &true);
    let proof_cid = String::from_str(&env, "QmTestProofCid");
    let claim_request_id = client
        .mock_all_auths()
        .submit_claim_request(&project_id, &claimant, &proof_cid);

    // Approve claim
    client
        .mock_all_auths()
        .approve_claim_request(&claim_request_id, &admin);

    // Check claim request status
    let claim_request = client.get_claim_request(&claim_request_id).unwrap();
    assert_eq!(claim_request.status, ClaimStatus::Approved);

    // Check project ownership transferred
    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.owner, claimant);
    assert!(!project.claimable); // Should no longer be claimable
}

#[test]
fn test_reject_claim_request() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let claimant = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Test Project");

    // Set project as claimable and submit claim
    client
        .mock_all_auths()
        .set_project_claimable(&project_id, &owner, &true);
    let proof_cid = String::from_str(&env, "QmTestProofCid");
    let claim_request_id = client
        .mock_all_auths()
        .submit_claim_request(&project_id, &claimant, &proof_cid);

    // Reject claim
    client
        .mock_all_auths()
        .reject_claim_request(&claim_request_id, &admin);

    // Check claim request status
    let claim_request = client.get_claim_request(&claim_request_id).unwrap();
    assert_eq!(claim_request.status, ClaimStatus::Rejected);

    // Check project ownership not transferred
    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.owner, owner);
}

#[test]
fn test_unauthorized_approve() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let claimant = Address::generate(&env);
    let non_admin = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Test Project");

    // Set project as claimable and submit claim
    client
        .mock_all_auths()
        .set_project_claimable(&project_id, &owner, &true);
    let proof_cid = String::from_str(&env, "QmTestProofCid");
    let claim_request_id = client
        .mock_all_auths()
        .submit_claim_request(&project_id, &claimant, &proof_cid);

    // Try to approve as non-admin (should fail)
    let result = client
        .mock_all_auths()
        .try_approve_claim_request(&claim_request_id, &non_admin);
    assert!(result.is_err());
}
