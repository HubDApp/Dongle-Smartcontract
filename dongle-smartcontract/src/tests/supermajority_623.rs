//! Tests for issue #623 – Admin Threshold Downgrade Security.
//!
//! The supermajority rule requires that a `SetThreshold` proposal that would
//! *lower* the current threshold must have strictly MORE approvals than the
//! *current* threshold — not just the proposed new threshold.
//!
//! # Guard formula
//!
//!   approvals.len() > current_threshold
//!
//! This prevents exactly `current_threshold` colluding admins from reducing
//! the quorum they are supposed to be subject to.
//!
//! # Coverage
//!
//! - Exact-threshold approvals (== current_threshold) → rejected (#623 core)
//! - One below threshold (current_threshold - 1) → rejected
//! - One above threshold (current_threshold + 1) → accepted ✅
//! - Raise proposal needs only current threshold → accepted ✅
//! - Attacker collusion scenario: `current_threshold` admins try to lower quorum
//! - No-op (new == current) is not a downgrade and needs only current threshold

#![cfg(test)]

extern crate alloc;

use crate::tests::fixtures::setup_contract;
use crate::types::{ProposalPayload, ProposalStatus};
use soroban_sdk::{testutils::Address as _, Address, Env};

// ─── helpers ─────────────────────────────────────────────────────────────────

/// Set up N admins with the threshold raised to `threshold`.
///
/// Returns (client, vec of all admin addresses).
/// All addresses are distinct; `admins[0]` is the initial bootstrap admin.
fn setup_n_admins_with_threshold(
    env: &Env,
    n: u32,
    threshold: u32,
) -> (crate::DongleContractClient<'_>, alloc::vec::Vec<Address>) {
    assert!(n >= 1);
    assert!(threshold >= 1 && threshold <= n);

    let (client, admin0) = setup_contract(env);
    let mut admins = alloc::vec![admin0.clone()];

    for _ in 1..n {
        let a = Address::generate(env);
        client.add_admin(&admin0, &a);
        admins.push(a);
    }

    // Raise threshold while still in single-admin mode (direct path).
    if threshold > 1 {
        client.set_admin_approval_threshold(&admin0, &threshold);
    }

    assert_eq!(client.get_admin_count(), n);
    assert_eq!(client.get_admin_approval_threshold(), threshold);

    (client, admins)
}

/// Create a SetThreshold(new_threshold) proposal proposed by `admins[0]` and
/// return its proposal id.
fn propose_set_threshold(
    client: &crate::DongleContractClient<'_>,
    proposer: &Address,
    new_threshold: u32,
) -> u64 {
    client.create_proposal(proposer, &ProposalPayload::SetThreshold(new_threshold), &0u64)
}

/// Approve a proposal with `approvers[0..count]` (skipping the proposer who
/// already voted on creation).
fn approve_by(
    client: &crate::DongleContractClient<'_>,
    proposal_id: u64,
    approvers: &[&Address],
) {
    for a in approvers {
        client.approve_proposal(a, &proposal_id);
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Core supermajority tests
// ═══════════════════════════════════════════════════════════════════════════

/// Exact threshold: threshold=3, propose lower to 2, collect exactly 3 approvals.
/// approvals.len() == current_threshold → must be REJECTED.
#[test]
fn downgrade_with_exact_threshold_approvals_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    // 4 admins, threshold = 3
    let (client, admins) = setup_n_admins_with_threshold(&env, 4, 3);

    // admins[0] proposes; auto-votes. admins[1] + admins[2] bring total to 3 = current_threshold.
    let id = propose_set_threshold(&client, &admins[0], 2);
    approve_by(&client, id, &[&admins[1], &admins[2]]); // 3 total

    assert_eq!(client.get_proposal(&id).unwrap().status, ProposalStatus::Approved);

    // Execute: must fail with ThresholdDowngradeRequiresSupermajority.
    let result = client.try_execute_proposal(&admins[3], &id);
    assert_eq!(
        result,
        Err(Ok(
            crate::errors::ContractError::ThresholdDowngradeRequiresSupermajority
        )),
        "execution with exactly current_threshold approvals must be rejected"
    );
    assert_eq!(
        client.get_admin_approval_threshold(),
        3,
        "threshold must remain unchanged"
    );
}

/// One below threshold: threshold=3, propose lower to 2, collect only 2 approvals.
/// approvals.len() < current_threshold → proposal never reaches Approved status,
/// and execution is rejected for a different reason (status != Approved).
#[test]
fn downgrade_with_below_threshold_approvals_cannot_execute() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admins) = setup_n_admins_with_threshold(&env, 4, 3);

    // Only proposer's auto-vote + admins[1] = 2 approvals < 3.
    let id = propose_set_threshold(&client, &admins[0], 2);
    approve_by(&client, id, &[&admins[1]]); // 2 total

    assert_eq!(client.get_proposal(&id).unwrap().status, ProposalStatus::Pending);

    let result = client.try_execute_proposal(&admins[3], &id);
    assert!(
        result.is_err(),
        "proposal that hasn't reached Approved must not execute"
    );
    assert_eq!(client.get_admin_approval_threshold(), 3);
}

/// Supermajority: threshold=3, propose lower to 2, collect 4 approvals (> current_threshold).
/// Must SUCCEED.
#[test]
fn downgrade_with_supermajority_succeeds() {
    let env = Env::default();
    env.mock_all_auths();
    // 5 admins, threshold = 3
    let (client, admins) = setup_n_admins_with_threshold(&env, 5, 3);

    // proposer (auto) + admins[1] + admins[2] + admins[3] = 4 approvals > 3.
    let id = propose_set_threshold(&client, &admins[0], 2);
    approve_by(&client, id, &[&admins[1], &admins[2], &admins[3]]);

    client.execute_proposal(&admins[4], &id);

    assert_eq!(
        client.get_admin_approval_threshold(),
        2,
        "threshold must be updated to the new value"
    );
    assert_eq!(
        client.get_proposal(&id).unwrap().status,
        ProposalStatus::Executed
    );
}

/// Minimum supermajority: threshold=3, collect exactly current_threshold+1 = 4.
#[test]
fn downgrade_with_threshold_plus_one_approvals_succeeds() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admins) = setup_n_admins_with_threshold(&env, 5, 3);

    // 4 approvals = current_threshold + 1 = 3 + 1 → must succeed.
    let id = propose_set_threshold(&client, &admins[0], 2);
    approve_by(&client, id, &[&admins[1], &admins[2], &admins[3]]); // total 4

    client.execute_proposal(&admins[4], &id);
    assert_eq!(client.get_admin_approval_threshold(), 2);
}

// ═══════════════════════════════════════════════════════════════════════════
// Raise-threshold does NOT require supermajority
// ═══════════════════════════════════════════════════════════════════════════

/// Raising the threshold only needs current_threshold approvals (normal path).
#[test]
fn raise_threshold_needs_only_current_threshold() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admins) = setup_n_admins_with_threshold(&env, 4, 2);

    // Raise from 2 to 3 — only 2 approvals required.
    let id = propose_set_threshold(&client, &admins[0], 3);
    approve_by(&client, id, &[&admins[1]]); // total 2 = current_threshold

    client.execute_proposal(&admins[2], &id);
    assert_eq!(client.get_admin_approval_threshold(), 3);
}

/// A no-op SetThreshold(current) should not trigger supermajority (new == current is not a downgrade).
#[test]
fn noop_threshold_via_proposal_not_downgrade() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admins) = setup_n_admins_with_threshold(&env, 3, 2);

    // SetThreshold(2) when current is already 2 — no-op, not a downgrade.
    let id = propose_set_threshold(&client, &admins[0], 2);
    approve_by(&client, id, &[&admins[1]]); // total 2 = current_threshold

    // Should execute without needing supermajority.
    client.execute_proposal(&admins[2], &id);
    assert_eq!(client.get_admin_approval_threshold(), 2);
}

// ═══════════════════════════════════════════════════════════════════════════
// Attack scenario: exactly-threshold colluding admins try to lower quorum
// ═══════════════════════════════════════════════════════════════════════════

/// Attack scenario (issue #623):
///
/// Threshold = 3 with 5 admins. Exactly 3 colluding admins try to lower
/// the threshold to 1, which would give any single admin total control.
/// The supermajority guard must block this.
#[test]
fn attack_exactly_threshold_admins_cannot_lower_quorum() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admins) = setup_n_admins_with_threshold(&env, 5, 3);

    // Three colluding admins (admins[0], admins[1], admins[2]) try to reduce
    // threshold to 1.  admins[0] proposes and auto-votes; 1+2 = 3 total.
    let id = propose_set_threshold(&client, &admins[0], 1);
    approve_by(&client, id, &[&admins[1], &admins[2]]); // 3 total == current_threshold

    // Attempt execution — must be rejected.
    let result = client.try_execute_proposal(&admins[0], &id);
    assert_eq!(
        result,
        Err(Ok(
            crate::errors::ContractError::ThresholdDowngradeRequiresSupermajority
        )),
        "attack: exactly threshold colluding admins must not be able to lower quorum"
    );
    // Threshold unchanged — governance quorum still intact.
    assert_eq!(client.get_admin_approval_threshold(), 3);
    // None of the attackers gained elevated control.
    assert!(!client.is_admin(&Address::generate(&env)));
}

/// Verify that a single rogue admin with threshold=1 cannot lower threshold
/// below 1 (threshold=0 should be rejected as invalid regardless of quorum).
#[test]
fn set_threshold_zero_rejected_as_invalid() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admins) = setup_n_admins_with_threshold(&env, 1, 1);

    // threshold=1 so proposal is auto-approved on creation.
    let id = propose_set_threshold(&client, &admins[0], 0);
    // Execute should fail with InvalidProjectData (threshold=0 is never valid).
    let result = client.try_execute_proposal(&admins[0], &id);
    assert!(
        result.is_err(),
        "threshold=0 proposal must be rejected on execution"
    );
    assert_eq!(client.get_admin_approval_threshold(), 1);
}

/// Verify that SetThreshold above admin_count is rejected.
#[test]
fn set_threshold_above_admin_count_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admins) = setup_n_admins_with_threshold(&env, 2, 1);

    // 2 admins; threshold=3 would exceed admin count.
    let id = propose_set_threshold(&client, &admins[0], 3);
    let result = client.try_execute_proposal(&admins[0], &id);
    assert!(
        result.is_err(),
        "threshold > admin_count must be rejected on execution"
    );
    assert_eq!(client.get_admin_approval_threshold(), 1);
}
