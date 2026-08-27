//! Multi-sig proposal tests with an approval threshold above one (issue #489).
//!
//! `multisig_and_history.rs` covers a single two-of-three run. This module
//! covers the rest of the surface the issue calls out: two-of-three approval
//! ordering, duplicate votes, execution attempted before the threshold is met,
//! and execution of every `ProposalPayload` variant.

#![cfg(test)]

use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::{ProposalPayload, ProposalStatus, VerificationStatus};
use soroban_sdk::{testutils::Address as _, Address, Env, String};

/// Three admins with the approval threshold raised to two.
///
/// The threshold must be set *before* it exceeds one: once above one,
/// `set_admin_approval_threshold` is itself gated behind a proposal.
fn setup_two_of_three(env: &Env) -> (crate::DongleContractClient<'_>, Address, Address, Address) {
    let (client, admin1) = setup_contract(env);
    let admin2 = Address::generate(env);
    let admin3 = Address::generate(env);

    client.add_admin(&admin1, &admin2);
    client.add_admin(&admin1, &admin3);
    client.set_admin_approval_threshold(&admin1, &2);

    assert_eq!(client.get_admin_count(), 3);
    assert_eq!(client.get_admin_approval_threshold(), 2);

    (client, admin1, admin2, admin3)
}

// ─── Approval accounting ─────────────────────────────────────────────────────

#[test]
fn test_proposer_approval_counts_toward_threshold() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin1, _admin2, _admin3) = setup_two_of_three(&env);

    let target = Address::generate(&env);
    let id = client.create_proposal(&admin1, &ProposalPayload::AddAdmin(target), &0u64);

    // Creating a proposal records the proposer's own approval.
    let proposal = client.get_proposal(&id).unwrap();
    assert_eq!(proposal.approvals.len(), 1);
    assert_eq!(proposal.approvals.get(0).unwrap(), admin1);
    assert_eq!(proposal.status, ProposalStatus::Pending);
}

#[test]
fn test_second_distinct_approval_meets_threshold() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin1, admin2, _admin3) = setup_two_of_three(&env);

    let target = Address::generate(&env);
    let id = client.create_proposal(&admin1, &ProposalPayload::AddAdmin(target), &0u64);

    client.approve_proposal(&admin2, &id);

    let proposal = client.get_proposal(&id).unwrap();
    assert_eq!(proposal.approvals.len(), 2);
    assert_eq!(proposal.status, ProposalStatus::Approved);
}

#[test]
fn test_duplicate_approval_from_same_admin_is_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin1, admin2, _admin3) = setup_two_of_three(&env);

    let target = Address::generate(&env);
    let id = client.create_proposal(&admin1, &ProposalPayload::AddAdmin(target), &0u64);

    // The proposer already approved implicitly.
    assert!(client.try_approve_proposal(&admin1, &id).is_err());

    client.approve_proposal(&admin2, &id);
    // And a second vote from admin2 is refused too.
    assert!(client.try_approve_proposal(&admin2, &id).is_err());

    let proposal = client.get_proposal(&id).unwrap();
    assert_eq!(
        proposal.approvals.len(),
        2,
        "duplicate votes must not inflate the approval count"
    );
}

#[test]
fn test_non_admin_cannot_approve() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin1, _admin2, _admin3) = setup_two_of_three(&env);

    let outsider = Address::generate(&env);
    let target = Address::generate(&env);
    let id = client.create_proposal(&admin1, &ProposalPayload::AddAdmin(target), &0u64);

    assert!(client.try_approve_proposal(&outsider, &id).is_err());
    assert_eq!(client.get_proposal(&id).unwrap().approvals.len(), 1);
}

#[test]
fn test_admin_can_reject_pending_proposal() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin1, _admin2, admin3) = setup_two_of_three(&env);

    let target = Address::generate(&env);
    let id = client.create_proposal(&admin1, &ProposalPayload::AddAdmin(target), &0u64);

    client.reject_proposal(&admin3, &id);

    assert_eq!(
        client.get_proposal(&id).unwrap().status,
        ProposalStatus::Rejected
    );
    assert!(client.try_approve_proposal(&admin3, &id).is_err());
    assert!(client.try_execute_proposal(&admin3, &id).is_err());
}

#[test]
fn test_non_admin_cannot_reject_proposal() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin1, _admin2, _admin3) = setup_two_of_three(&env);

    let outsider = Address::generate(&env);
    let target = Address::generate(&env);
    let id = client.create_proposal(&admin1, &ProposalPayload::AddAdmin(target), &0u64);

    assert!(client.try_reject_proposal(&outsider, &id).is_err());
    assert_eq!(
        client.get_proposal(&id).unwrap().status,
        ProposalStatus::Pending
    );
}

#[test]
fn test_approved_proposal_cannot_be_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin1, admin2, _admin3) = setup_two_of_three(&env);

    let target = Address::generate(&env);
    let id = client.create_proposal(&admin1, &ProposalPayload::AddAdmin(target), &0u64);
    client.approve_proposal(&admin2, &id);

    assert!(client.try_reject_proposal(&admin1, &id).is_err());
    assert_eq!(
        client.get_proposal(&id).unwrap().status,
        ProposalStatus::Approved
    );
}

// ─── Execution gating ────────────────────────────────────────────────────────

#[test]
fn test_execute_before_threshold_is_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin1, _admin2, admin3) = setup_two_of_three(&env);

    let target = Address::generate(&env);
    let id = client.create_proposal(&admin1, &ProposalPayload::AddAdmin(target.clone()), &0u64);

    // Only one approval so far; threshold is two.
    assert!(client.try_execute_proposal(&admin3, &id).is_err());

    // The action must not have taken effect.
    assert!(!client.is_admin(&target));
    assert_eq!(
        client.get_proposal(&id).unwrap().status,
        ProposalStatus::Pending
    );
}

#[test]
fn test_execute_twice_is_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin1, admin2, admin3) = setup_two_of_three(&env);

    let target = Address::generate(&env);
    let id = client.create_proposal(&admin1, &ProposalPayload::AddAdmin(target), &0u64);
    client.approve_proposal(&admin2, &id);
    client.execute_proposal(&admin3, &id);

    assert_eq!(
        client.get_proposal(&id).unwrap().status,
        ProposalStatus::Executed
    );
    assert!(client.try_execute_proposal(&admin3, &id).is_err());
}

#[test]
fn test_any_admin_may_execute_an_approved_proposal() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin1, admin2, admin3) = setup_two_of_three(&env);

    let target = Address::generate(&env);
    let id = client.create_proposal(&admin1, &ProposalPayload::AddAdmin(target.clone()), &0u64);
    client.approve_proposal(&admin2, &id);

    // admin3 never voted, but execution is not restricted to approvers.
    client.execute_proposal(&admin3, &id);
    assert!(client.is_admin(&target));
}

#[test]
fn test_non_admin_cannot_execute() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin1, admin2, _admin3) = setup_two_of_three(&env);

    let outsider = Address::generate(&env);
    let target = Address::generate(&env);
    let id = client.create_proposal(&admin1, &ProposalPayload::AddAdmin(target.clone()), &0u64);
    client.approve_proposal(&admin2, &id);

    assert!(client.try_execute_proposal(&outsider, &id).is_err());
    assert!(!client.is_admin(&target));
}

// ─── Every payload variant ───────────────────────────────────────────────────

/// Approve `id` with `admin2` and execute it with `admin3`.
fn approve_and_execute(
    client: &crate::DongleContractClient<'_>,
    admin2: &Address,
    admin3: &Address,
    id: u64,
) {
    client.approve_proposal(admin2, &id);
    assert_eq!(
        client.get_proposal(&id).unwrap().status,
        ProposalStatus::Approved
    );
    client.execute_proposal(admin3, &id);
    assert_eq!(
        client.get_proposal(&id).unwrap().status,
        ProposalStatus::Executed
    );
}

#[test]
fn test_execute_add_admin_payload() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin1, admin2, admin3) = setup_two_of_three(&env);

    let target = Address::generate(&env);
    let id = client.create_proposal(&admin1, &ProposalPayload::AddAdmin(target.clone()), &0u64);
    approve_and_execute(&client, &admin2, &admin3, id);

    assert!(client.is_admin(&target));
    assert_eq!(client.get_admin_count(), 4);
}

#[test]
fn test_execute_remove_admin_payload() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin1, admin2, admin3) = setup_two_of_three(&env);

    let id = client.create_proposal(
        &admin1,
        &ProposalPayload::RemoveAdmin(admin3.clone()),
        &0u64,
    );
    approve_and_execute(&client, &admin2, &admin3, id);

    assert!(!client.is_admin(&admin3));
    assert_eq!(client.get_admin_count(), 2);
}

#[test]
fn test_execute_set_threshold_payload() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin1, admin2, admin3) = setup_two_of_three(&env);

    let id = client.create_proposal(&admin1, &ProposalPayload::SetThreshold(3), &0u64);
    approve_and_execute(&client, &admin2, &admin3, id);

    assert_eq!(client.get_admin_approval_threshold(), 3);
}

#[test]
fn test_execute_set_fee_payload() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin1, admin2, admin3) = setup_two_of_three(&env);

    let token_admin = Address::generate(&env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    let treasury = Address::generate(&env);

    let id = client.create_proposal(
        &admin1,
        &ProposalPayload::SetFee(Some(token.clone()), 250u128, 75u128, treasury.clone()),
        &0u64,
    );
    approve_and_execute(&client, &admin2, &admin3, id);

    let config = client.get_fee_config();
    assert_eq!(config.verification_fee, 250u128);
    assert_eq!(config.registration_fee, 75u128);
    assert_eq!(config.token, Some(token));
    // `FeeConfig` does not carry the treasury; it is stored separately.
    let _ = treasury;
}

#[test]
fn test_execute_approve_verification_payload() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin1, admin2, admin3) = setup_two_of_three(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "ProposalApproveTarget");
    let cid = String::from_str(&env, "QmYwAPJhy5nTAQCj9g1s2bkss7jBlEd22bN2R4s5gR5PTa");
    client.request_verification(&project_id, &owner, &cid);

    let id = client.create_proposal(
        &admin1,
        &ProposalPayload::ApproveVerification(project_id),
        &0u64,
    );
    approve_and_execute(&client, &admin2, &admin3, id);

    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.verification_status, VerificationStatus::Verified);
}

#[test]
fn test_execute_reject_verification_payload() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin1, admin2, admin3) = setup_two_of_three(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "ProposalRejectTarget");
    let cid = String::from_str(&env, "QmYwAPJhy5nTAQCj9g1s2bkss7jBlEd22bN2R4s5gR5PTa");
    client.request_verification(&project_id, &owner, &cid);

    let id = client.create_proposal(
        &admin1,
        &ProposalPayload::RejectVerification(project_id),
        &0u64,
    );
    approve_and_execute(&client, &admin2, &admin3, id);

    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.verification_status, VerificationStatus::Rejected);
}

#[test]
fn test_execute_revoke_verification_payload() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin1, admin2, admin3) = setup_two_of_three(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "ProposalRevokeTarget");
    let cid = String::from_str(&env, "QmYwAPJhy5nTAQCj9g1s2bkss7jBlEd22bN2R4s5gR5PTa");
    client.request_verification(&project_id, &owner, &cid);

    // Verify it first, through a proposal, since direct admin calls are gated.
    let approve_id = client.create_proposal(
        &admin1,
        &ProposalPayload::ApproveVerification(project_id),
        &0u64,
    );
    approve_and_execute(&client, &admin2, &admin3, approve_id);
    assert_eq!(
        client.get_project(&project_id).unwrap().verification_status,
        VerificationStatus::Verified
    );

    let reason = String::from_str(&env, "QmYwAPJhy5nTAQCj9g1s2bkss7jBlEd22bN2R4s5gR5PTc");
    let revoke_id = client.create_proposal(
        &admin1,
        &ProposalPayload::RevokeVerification(project_id, reason),
        &0u64,
    );
    approve_and_execute(&client, &admin2, &admin3, revoke_id);

    let project = client.get_project(&project_id).unwrap();
    assert_ne!(project.verification_status, VerificationStatus::Verified);
}

// ─── Threshold interaction ───────────────────────────────────────────────────

#[test]
fn test_raising_threshold_blocks_an_already_approved_proposal() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin1, admin2, admin3) = setup_two_of_three(&env);

    // Proposal A reaches the current threshold of two but is not executed yet.
    let target = Address::generate(&env);
    let pending =
        client.create_proposal(&admin1, &ProposalPayload::AddAdmin(target.clone()), &0u64);
    client.approve_proposal(&admin2, &pending);
    assert_eq!(
        client.get_proposal(&pending).unwrap().status,
        ProposalStatus::Approved
    );

    // Proposal B raises the threshold to three and executes.
    let raise = client.create_proposal(&admin1, &ProposalPayload::SetThreshold(3), &0u64);
    client.approve_proposal(&admin2, &raise);
    client.execute_proposal(&admin3, &raise);
    assert_eq!(client.get_admin_approval_threshold(), 3);

    // Proposal A now has fewer approvals than the live threshold. Execution
    // re-checks the threshold at execution time rather than trusting the
    // status recorded when it was approved.
    assert!(client.try_execute_proposal(&admin3, &pending).is_err());
    assert!(!client.is_admin(&target));
}

// ─── Threshold downgrade supermajority protection ────────────────────────────

/// Exact-majority attempt: threshold is 3, proposal wants to lower to 2,
/// but only 3 admins approved (== current_threshold). Must be rejected because
/// the guard requires strictly MORE than current_threshold approvals.
#[test]
fn test_downgrade_threshold_rejected_when_approvals_equal_new_threshold() {
    let env = Env::default();
    env.mock_all_auths();

    // Four admins, threshold = 3.
    let (client, admin1) = crate::tests::fixtures::setup_contract(&env);
    let admin2 = Address::generate(&env);
    let admin3 = Address::generate(&env);
    let admin4 = Address::generate(&env);
    client.add_admin(&admin1, &admin2);
    client.add_admin(&admin1, &admin3);
    client.add_admin(&admin1, &admin4);
    client.set_admin_approval_threshold(&admin1, &3);
    assert_eq!(client.get_admin_approval_threshold(), 3);

    // Propose lowering from 3 to 2.
    let id = client.create_proposal(&admin1, &ProposalPayload::SetThreshold(2), &0u64);
    // admin1 (proposer) + admin2 + admin3 = 3 approvals, which equals the
    // current threshold of 3 but does NOT exceed it.  The supermajority guard
    // requires strictly more than current_threshold, so execution must be refused.
    client.approve_proposal(&admin2, &id);
    client.approve_proposal(&admin3, &id); // 3 approvals == current_threshold → Approved
    assert_eq!(
        client.get_proposal(&id).unwrap().status,
        ProposalStatus::Approved
    );

    let result = client.try_execute_proposal(&admin4, &id);
    assert_eq!(
        result,
        Err(Ok(
            crate::errors::ContractError::ThresholdDowngradeRequiresSupermajority
        ))
    );
    // Threshold must remain unchanged.
    assert_eq!(client.get_admin_approval_threshold(), 3);
}
/// have approved (> current_threshold of 3). Must succeed.
#[test]
fn test_downgrade_threshold_succeeds_with_supermajority() {
    let env = Env::default();
    env.mock_all_auths();

    // Five admins, threshold = 3.
    let (client, admin1) = crate::tests::fixtures::setup_contract(&env);
    let admin2 = Address::generate(&env);
    let admin3 = Address::generate(&env);
    let admin4 = Address::generate(&env);
    let admin5 = Address::generate(&env);
    client.add_admin(&admin1, &admin2);
    client.add_admin(&admin1, &admin3);
    client.add_admin(&admin1, &admin4);
    client.add_admin(&admin1, &admin5);
    client.set_admin_approval_threshold(&admin1, &3);
    assert_eq!(client.get_admin_approval_threshold(), 3);

    // Propose lowering from 3 to 2.
    let id = client.create_proposal(&admin1, &ProposalPayload::SetThreshold(2), &0u64);
    // admin1 (proposer) + admin2 + admin3 + admin4 = 4 approvals > current_threshold of 3.
    client.approve_proposal(&admin2, &id);
    client.approve_proposal(&admin3, &id);
    client.approve_proposal(&admin4, &id);

    // Execute — should succeed because 4 > current_threshold(3).
    client.execute_proposal(&admin5, &id);
    assert_eq!(client.get_admin_approval_threshold(), 2);
    assert_eq!(
        client.get_proposal(&id).unwrap().status,
        ProposalStatus::Executed
    );
}

/// Raising the threshold via a proposal does not trigger the supermajority
/// rule — only the live threshold needs to be met.
#[test]
fn test_raise_threshold_via_proposal_needs_only_current_threshold() {
    let env = Env::default();
    env.mock_all_auths();
    // Two-of-three setup from the helper.
    let (client, admin1, admin2, admin3) = setup_two_of_three(&env);

    // Propose raising threshold from 2 to 3 — only 2 approvals needed.
    let id = client.create_proposal(&admin1, &ProposalPayload::SetThreshold(3), &0u64);
    client.approve_proposal(&admin2, &id);
    // 2 approvals == current threshold of 2 → Approved; execute.
    client.execute_proposal(&admin3, &id);

    assert_eq!(client.get_admin_approval_threshold(), 3);
}

/// Threshold stays at 1 (single-admin). A SetThreshold(1) no-op via proposal
/// is allowed because new_threshold == current_threshold is not a downgrade.
#[test]
fn test_set_threshold_noop_via_proposal_is_allowed() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = crate::tests::fixtures::setup_contract(&env);

    // With threshold == 1 a proposal is immediately auto-approved on creation.
    let id = client.create_proposal(&admin, &ProposalPayload::SetThreshold(1), &0u64);
    assert_eq!(
        client.get_proposal(&id).unwrap().status,
        ProposalStatus::Approved
    );
    // Executing the no-op must succeed without triggering the supermajority guard.
    client.execute_proposal(&admin, &id);
    assert_eq!(client.get_admin_approval_threshold(), 1);
}

/// Verify that `set_admin_approval_threshold` (direct path) is blocked once
/// the threshold is already above 1, forcing all future changes through
/// the proposal system.
#[test]
fn test_direct_set_threshold_blocked_after_multisig_enabled() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin1, admin2, _admin3) = setup_two_of_three(&env);

    // Attempting to lower via the direct path must fail.
    let result = client.try_set_admin_approval_threshold(&admin1, &1);
    assert!(
        result.is_err(),
        "direct set_admin_approval_threshold must be blocked once threshold > 1"
    );
    assert_eq!(client.get_admin_approval_threshold(), 2);

    // Attempting to raise via the direct path must also fail.
    let result = client.try_set_admin_approval_threshold(&admin2, &3);
    assert!(
        result.is_err(),
        "even raising the threshold directly must go through the proposal system"
    );
    assert_eq!(client.get_admin_approval_threshold(), 2);
}
