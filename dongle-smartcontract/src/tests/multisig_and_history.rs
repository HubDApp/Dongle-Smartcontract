#![cfg(test)]

use crate::errors::ContractError;
use crate::tests::fixtures::{setup_contract, create_test_project};
use crate::types::{ProposalPayload, ProposalStatus, VerificationStatus};
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};

#[test]
fn test_historical_verification_records() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "ProjectH");

    // Initially unverified
    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.verification_status, VerificationStatus::Unverified);
    assert!(project.current_verification_id.is_none());

    // Request 1: Verification Request with valid IPFS CIDv0
    let evidence1 = String::from_str(&env, "QmYwAPJhy5nTAQCj9g1s2bkss7jBlEd22bN2R4s5gR5PTa");
    client.request_verification(&project_id, &owner, &evidence1);

    // Approve verification
    client.approve_verification(&project_id, &admin);

    // Verify Project Status & current_verification_id
    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.verification_status, VerificationStatus::Verified);
    let v_id1 = project.current_verification_id.unwrap();

    let record1 = client.get_verification_record(&v_id1);
    assert_eq!(record1.status, VerificationStatus::Verified);
    assert_eq!(record1.evidence_cid, evidence1);

    // Revoke verification to transition Verified -> Unverified
    let revoke_reason = String::from_str(&env, "revocation for testing");
    client.revoke_verification(&project_id, &admin, &revoke_reason);

    // Verify status is Unverified
    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.verification_status, VerificationStatus::Unverified);

    // Request 2: Rejected Verification Request (Unverified -> Pending)
    let evidence2 = String::from_str(&env, "QmYwAPJhy5nTAQCj9g1s2bkss7jBlEd22bN2R4s5gR5PTb");
    client.request_verification(&project_id, &owner, &evidence2);
    // Reject it (Pending -> Rejected)
    client.reject_verification(&project_id, &admin);

    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.verification_status, VerificationStatus::Rejected);
    let v_id2 = project.current_verification_id.unwrap();
    assert!(v_id2 != v_id1);

    let record2 = client.get_verification_record(&v_id2);
    assert_eq!(record2.status, VerificationStatus::Rejected);
    assert_eq!(record2.evidence_cid, evidence2);

    // Request 3: Verified again (Rejected -> Pending)
    let evidence3 = String::from_str(&env, "QmYwAPJhy5nTAQCj9g1s2bkss7jBlEd22bN2R4s5gR5PTc");
    client.request_verification(&project_id, &owner, &evidence3);
    // Approve it (Pending -> Verified)
    client.approve_verification(&project_id, &admin);

    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.verification_status, VerificationStatus::Verified);
    let v_id3 = project.current_verification_id.unwrap();

    // Fetch history and verify order
    let history = client.get_verification_history(&project_id);
    assert_eq!(history.len(), 3);
    assert_eq!(history.get(0).unwrap().request_id, v_id1);
    assert_eq!(history.get(1).unwrap().request_id, v_id2);
    assert_eq!(history.get(2).unwrap().request_id, v_id3);

    // Fetch current and historical verification records
    let current_record = client.get_verification(&project_id);
    assert_eq!(current_record.request_id, v_id3);
}

#[test]
fn test_admin_multisig_approval_threshold() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin1) = setup_contract(&env);

    let admin2 = Address::generate(&env);
    let admin3 = Address::generate(&env);
    let admin4 = Address::generate(&env);

    // Add admin2 and admin3
    client.add_admin(&admin1, &admin2);
    client.add_admin(&admin1, &admin3);

    assert_eq!(client.get_admin_count(), 3);
    assert_eq!(client.get_admin_approval_threshold(), 1);

    // Change threshold to 2
    client.set_admin_approval_threshold(&admin1, &2);
    assert_eq!(client.get_admin_approval_threshold(), 2);

    // Trying to set threshold again directly should fail with Unauthorized
    let res = client.try_set_admin_approval_threshold(&admin1, &3);
    assert!(res.is_err());

    // Trying to add an admin directly should fail with Unauthorized
    let res = client.try_add_admin(&admin1, &admin4);
    assert!(res.is_err());

    // Create a proposal to add admin4
    let payload = ProposalPayload::AddAdmin(admin4.clone());
    let proposal_id = client.create_proposal(&admin1, &payload);

    // Verify proposal details
    let proposal = client.get_proposal(&proposal_id).unwrap();
    assert_eq!(proposal.proposer, admin1);
    assert_eq!(proposal.status, ProposalStatus::Pending);
    assert_eq!(proposal.approvals.len(), 1);
    assert_eq!(proposal.approvals.get(0).unwrap(), admin1);

    // Try to approve again by admin1 -> duplicate approval should fail
    let res = client.try_approve_proposal(&admin1, &proposal_id);
    assert!(res.is_err());

    // Approve by admin2 -> should meet threshold and change status to Approved
    client.approve_proposal(&admin2, &proposal_id);
    let proposal = client.get_proposal(&proposal_id).unwrap();
    assert_eq!(proposal.status, ProposalStatus::Approved);
    assert_eq!(proposal.approvals.len(), 2);

    // Execute proposal
    client.execute_proposal(&admin3, &proposal_id);
    let proposal = client.get_proposal(&proposal_id).unwrap();
    assert_eq!(proposal.status, ProposalStatus::Executed);

    // Admin 4 is now admin!
    assert!(client.is_admin(&admin4));
    assert_eq!(client.get_admin_count(), 4);

    // Execute again should fail
    let res = client.try_execute_proposal(&admin3, &proposal_id);
    assert!(res.is_err());
}
