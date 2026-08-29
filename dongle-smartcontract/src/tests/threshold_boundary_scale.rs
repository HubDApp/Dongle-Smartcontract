//! Tests for issues #689 (Proposal Threshold Validation Boundary Cases) and
//! #690 (Multisig Approval Supermajority Rule Edge Cases).
//!
//! `proposal_threshold.rs` and `supermajority_623.rs` already cover the core
//! supermajority-downgrade boundary (approvals == current_threshold rejected,
//! approvals == current_threshold + 1 accepted) at small admin counts (4-5
//! admins). This module fills the two gaps called out by the issues:
//!
//! - #690: the same boundary re-verified at 10 and 100 admins, so the
//!   `approvals.len() > current_threshold` comparison is proven to hold at
//!   scale rather than only for small, easy-to-miscount groups.
//! - #689: `set_admin_approval_threshold`'s own input validation
//!   (`threshold == 0`, `threshold > admin_count`, and the `threshold == 1`
//!   minimum-valid boundary), plus the identical validation inside
//!   `execute_proposal`'s `SetThreshold` branch, which was previously
//!   exercised only indirectly.
//!
//! # Guard formula (see also supermajority_623.rs's header)
//!
//! Direct path (`set_admin_approval_threshold`, single-admin fast path only):
//!   valid iff `1 <= threshold <= admin_count`
//!
//! Proposal path (`SetThreshold` in `execute_proposal`):
//!   valid iff `1 <= new_threshold <= admin_count`
//!   AND (`new_threshold >= current_threshold` OR `approvals.len() > current_threshold`)

#![cfg(test)]

extern crate alloc;

use crate::tests::fixtures::setup_contract;
use crate::types::{ProposalPayload, ProposalStatus};
use soroban_sdk::{testutils::Address as _, Address, Env};

/// Set up N admins with the threshold raised to `threshold`. Mirrors
/// supermajority_623.rs's helper of the same shape.
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

    if threshold > 1 {
        client.set_admin_approval_threshold(&admin0, &threshold);
    }

    assert_eq!(client.get_admin_count(), n);
    assert_eq!(client.get_admin_approval_threshold(), threshold);

    (client, admins)
}

fn propose_set_threshold(
    client: &crate::DongleContractClient<'_>,
    proposer: &Address,
    new_threshold: u32,
) -> u64 {
    client.create_proposal(proposer, &ProposalPayload::SetThreshold(new_threshold), &0u64)
}

fn approve_by(client: &crate::DongleContractClient<'_>, proposal_id: u64, approvers: &[&Address]) {
    for a in approvers {
        client.approve_proposal(a, &proposal_id);
    }
}

// ─── #690: supermajority boundary at scale ────────────────────────────────────

/// 10 admins, threshold 6. Exactly `current_threshold` approvals (== 6) must
/// still be rejected — the boundary doesn't drift as the admin set grows.
#[test]
fn downgrade_boundary_rejected_at_ten_admins() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admins) = setup_n_admins_with_threshold(&env, 10, 6);

    let id = propose_set_threshold(&client, &admins[0], 4);
    // proposer auto-vote (1) + 5 more = 6 == current_threshold.
    approve_by(&client, id, &[&admins[1], &admins[2], &admins[3], &admins[4], &admins[5]]);

    assert_eq!(client.get_proposal(&id).unwrap().status, ProposalStatus::Approved);

    let result = client.try_execute_proposal(&admins[9], &id);
    assert_eq!(
        result,
        Err(Ok(
            crate::errors::ContractError::ThresholdDowngradeRequiresSupermajority
        )),
        "10 admins, 6 approvals == current_threshold(6) must still be rejected"
    );
    assert_eq!(client.get_admin_approval_threshold(), 6);
}

/// Same 10-admin setup, one more approval (7 > 6) — must succeed.
#[test]
fn downgrade_boundary_accepted_one_above_at_ten_admins() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admins) = setup_n_admins_with_threshold(&env, 10, 6);

    let id = propose_set_threshold(&client, &admins[0], 4);
    // proposer auto-vote (1) + 6 more = 7 > current_threshold(6).
    approve_by(
        &client,
        id,
        &[&admins[1], &admins[2], &admins[3], &admins[4], &admins[5], &admins[6]],
    );

    client.execute_proposal(&admins[9], &id);
    assert_eq!(client.get_admin_approval_threshold(), 4);
    assert_eq!(client.get_proposal(&id).unwrap().status, ProposalStatus::Executed);
}

/// 100 admins, threshold 60. Exactly `current_threshold` approvals (== 60)
/// rejected, confirming the rule scales to a large admin set without
/// overflow or off-by-one drift from repeated increments.
#[test]
fn downgrade_boundary_rejected_at_hundred_admins() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admins) = setup_n_admins_with_threshold(&env, 100, 60);

    let id = propose_set_threshold(&client, &admins[0], 40);
    // proposer auto-vote (1) + 59 more = 60 == current_threshold.
    let approvers: alloc::vec::Vec<&Address> = admins[1..60].iter().collect();
    approve_by(&client, id, &approvers);

    assert_eq!(client.get_proposal(&id).unwrap().status, ProposalStatus::Approved);

    let result = client.try_execute_proposal(&admins[99], &id);
    assert_eq!(
        result,
        Err(Ok(
            crate::errors::ContractError::ThresholdDowngradeRequiresSupermajority
        )),
        "100 admins, 60 approvals == current_threshold(60) must still be rejected"
    );
    assert_eq!(client.get_admin_approval_threshold(), 60);
}

/// Same 100-admin setup, one more approval (61 > 60) — must succeed.
#[test]
fn downgrade_boundary_accepted_one_above_at_hundred_admins() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admins) = setup_n_admins_with_threshold(&env, 100, 60);

    let id = propose_set_threshold(&client, &admins[0], 40);
    // proposer auto-vote (1) + 60 more = 61 > current_threshold(60).
    let approvers: alloc::vec::Vec<&Address> = admins[1..61].iter().collect();
    approve_by(&client, id, &approvers);

    client.execute_proposal(&admins[99], &id);
    assert_eq!(client.get_admin_approval_threshold(), 40);
    assert_eq!(client.get_proposal(&id).unwrap().status, ProposalStatus::Executed);
}

// ─── #689: threshold validation boundary cases (direct path) ─────────────────

#[test]
fn direct_set_threshold_zero_is_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin1) = setup_contract(&env);

    let result = client.try_set_admin_approval_threshold(&admin1, &0);
    assert_eq!(
        result,
        Err(Ok(crate::errors::ContractError::InvalidProjectData)),
        "threshold of 0 must be rejected regardless of admin count"
    );
    assert_eq!(client.get_admin_approval_threshold(), 1);
}

#[test]
fn direct_set_threshold_above_admin_count_is_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin1) = setup_contract(&env);
    let admin2 = Address::generate(&env);
    client.add_admin(&admin1, &admin2);
    assert_eq!(client.get_admin_count(), 2);

    // 3 admins would be needed for threshold 3; only 2 exist.
    let result = client.try_set_admin_approval_threshold(&admin1, &3);
    assert_eq!(
        result,
        Err(Ok(crate::errors::ContractError::InvalidProjectData)),
        "threshold exceeding admin_count must be rejected"
    );
    assert_eq!(client.get_admin_approval_threshold(), 1);
}

#[test]
fn direct_set_threshold_equal_to_admin_count_is_accepted() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin1) = setup_contract(&env);
    let admin2 = Address::generate(&env);
    client.add_admin(&admin1, &admin2);
    assert_eq!(client.get_admin_count(), 2);

    // threshold == admin_count is the maximum valid value (full unanimity).
    client.set_admin_approval_threshold(&admin1, &2);
    assert_eq!(client.get_admin_approval_threshold(), 2);
}

#[test]
fn direct_set_threshold_of_one_is_the_minimum_valid_value() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin1) = setup_contract(&env);

    // Single-admin contracts start at threshold 1 already; setting it
    // explicitly to 1 (a no-op) must still succeed as the documented
    // minimum, not be treated as an invalid "no-op" edge case.
    client.set_admin_approval_threshold(&admin1, &1);
    assert_eq!(client.get_admin_approval_threshold(), 1);
}

// ─── #689: threshold validation boundary cases (proposal path) ───────────────

#[test]
fn proposal_set_threshold_zero_is_rejected_on_execute() {
    let env = Env::default();
    env.mock_all_auths();
    // 3 admins, threshold 2, so proposals are actually needed.
    let (client, admins) = setup_n_admins_with_threshold(&env, 3, 2);

    let id = propose_set_threshold(&client, &admins[0], 0);
    approve_by(&client, id, &[&admins[1]]);
    assert_eq!(client.get_proposal(&id).unwrap().status, ProposalStatus::Approved);

    let result = client.try_execute_proposal(&admins[2], &id);
    assert_eq!(
        result,
        Err(Ok(crate::errors::ContractError::InvalidProjectData)),
        "a SetThreshold(0) proposal must be rejected at execution even if approved"
    );
    assert_eq!(client.get_admin_approval_threshold(), 2);
}

#[test]
fn proposal_set_threshold_above_admin_count_is_rejected_on_execute() {
    let env = Env::default();
    env.mock_all_auths();
    // 3 admins, threshold 2.
    let (client, admins) = setup_n_admins_with_threshold(&env, 3, 2);

    let id = propose_set_threshold(&client, &admins[0], 4); // only 3 admins exist
    approve_by(&client, id, &[&admins[1]]);
    assert_eq!(client.get_proposal(&id).unwrap().status, ProposalStatus::Approved);

    let result = client.try_execute_proposal(&admins[2], &id);
    assert_eq!(
        result,
        Err(Ok(crate::errors::ContractError::InvalidProjectData)),
        "a SetThreshold proposal above admin_count must be rejected at execution"
    );
    assert_eq!(client.get_admin_approval_threshold(), 2);
}
