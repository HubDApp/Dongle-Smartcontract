//! Verification fee refund tests (issue #472).
//!
//! Rejecting a verification request used to keep the requester's fee. It now
//! records a claimable `FeeRefundRecord`, settled later by `claim_fee_refund`.
//!
//! The refund is recorded rather than transferred inline because paying out of
//! the treasury requires `treasury.require_auth()`, which the rejecting admin
//! cannot generally supply.

#![cfg(test)]

use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::VerificationStatus;
use soroban_sdk::{testutils::Address as _, token, Address, Env, String};

const VERIFICATION_FEE: u128 = 100;
const MINTED: i128 = 1_000;

struct Fixture<'a> {
    client: crate::DongleContractClient<'a>,
    admin: Address,
    owner: Address,
    treasury: Address,
    token_address: Address,
    project_id: u64,
}

/// A project that has paid the verification fee and has a pending request.
fn setup_pending_request(env: &Env, fee: u128) -> Fixture<'_> {
    let (client, admin) = setup_contract(env);
    let owner = Address::generate(env);
    let treasury = Address::generate(env);

    let token_admin = Address::generate(env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();

    client.set_fee(
        &admin,
        &Some(token_address.clone()),
        &fee,
        &0u128,
        &treasury,
    );

    let project_id = create_test_project(&client, &owner, "RefundableProject");

    if fee > 0 {
        token::StellarAssetClient::new(env, &token_address).mint(&owner, &MINTED);
        client.pay_fee(&owner, &project_id, &Some(token_address.clone()));
    }

    let cid = String::from_str(env, "QmYwAPJhy5nTAQCj9g1s2bkss7jBlEd22bN2R4s5gR5PTa");
    client.request_verification(&project_id, &owner, &cid);

    Fixture {
        client,
        admin,
        owner,
        treasury,
        token_address,
        project_id,
    }
}

fn balance(env: &Env, token_address: &Address, who: &Address) -> i128 {
    token::Client::new(env, token_address).balance(who)
}

// ─── Recording the refund ────────────────────────────────────────────────────

#[test]
fn test_rejection_records_a_claimable_refund() {
    let env = Env::default();
    env.mock_all_auths();
    let f = setup_pending_request(&env, VERIFICATION_FEE);

    assert!(f.client.get_fee_refund(&f.project_id).is_none());

    f.client.reject_verification(&f.project_id, &f.admin);

    let refund = f
        .client
        .get_fee_refund(&f.project_id)
        .expect("rejection must leave a claimable refund");
    assert_eq!(refund.project_id, f.project_id);
    assert_eq!(refund.payer, f.owner);
    assert_eq!(refund.amount, VERIFICATION_FEE);
    assert_eq!(refund.token, Some(f.token_address.clone()));
    assert_eq!(refund.claimed_at, None, "must start unclaimed");
    assert_eq!(refund.created_at, env.ledger().timestamp());
}

#[test]
fn test_rejection_still_rejects_the_project() {
    let env = Env::default();
    env.mock_all_auths();
    let f = setup_pending_request(&env, VERIFICATION_FEE);

    f.client.reject_verification(&f.project_id, &f.admin);

    let project = f.client.get_project(&f.project_id).unwrap();
    assert_eq!(project.verification_status, VerificationStatus::Rejected);
}

#[test]
fn test_recording_a_refund_moves_no_tokens_yet() {
    let env = Env::default();
    env.mock_all_auths();
    let f = setup_pending_request(&env, VERIFICATION_FEE);

    let before = balance(&env, &f.token_address, &f.owner);
    f.client.reject_verification(&f.project_id, &f.admin);

    // The debt is recorded; settlement is a separate, treasury-authorized step.
    assert_eq!(balance(&env, &f.token_address, &f.owner), before);
    assert!(f.client.get_fee_refund(&f.project_id).is_some());
}

#[test]
fn test_zero_fee_rejection_records_no_refund() {
    let env = Env::default();
    env.mock_all_auths();
    let f = setup_pending_request(&env, 0);

    f.client.reject_verification(&f.project_id, &f.admin);

    // Rejection must still succeed on a fee-free deployment.
    assert_eq!(
        f.client
            .get_project(&f.project_id)
            .unwrap()
            .verification_status,
        VerificationStatus::Rejected
    );
    assert!(f.client.get_fee_refund(&f.project_id).is_none());
}

#[test]
fn test_approval_records_no_refund() {
    let env = Env::default();
    env.mock_all_auths();
    let f = setup_pending_request(&env, VERIFICATION_FEE);

    f.client.approve_verification(&f.project_id, &f.admin);

    assert!(
        f.client.get_fee_refund(&f.project_id).is_none(),
        "an approved request consumed its fee legitimately"
    );
}

// ─── Claiming ────────────────────────────────────────────────────────────────

#[test]
fn test_payer_can_claim_the_refund() {
    let env = Env::default();
    env.mock_all_auths();
    let f = setup_pending_request(&env, VERIFICATION_FEE);
    f.client.reject_verification(&f.project_id, &f.admin);

    let before = balance(&env, &f.token_address, &f.owner);
    f.client.claim_fee_refund(&f.owner, &f.project_id);

    assert_eq!(
        balance(&env, &f.token_address, &f.owner),
        before + VERIFICATION_FEE as i128
    );
}

#[test]
fn test_claim_marks_the_record_claimed() {
    let env = Env::default();
    env.mock_all_auths();
    let f = setup_pending_request(&env, VERIFICATION_FEE);
    f.client.reject_verification(&f.project_id, &f.admin);

    f.client.claim_fee_refund(&f.owner, &f.project_id);

    let refund = f.client.get_fee_refund(&f.project_id).unwrap();
    assert_eq!(refund.claimed_at, Some(env.ledger().timestamp()));
}

#[test]
fn test_refund_comes_out_of_the_treasury() {
    let env = Env::default();
    env.mock_all_auths();
    let f = setup_pending_request(&env, VERIFICATION_FEE);
    f.client.reject_verification(&f.project_id, &f.admin);

    let treasury_before = balance(&env, &f.token_address, &f.treasury);
    f.client.claim_fee_refund(&f.owner, &f.project_id);

    assert_eq!(
        balance(&env, &f.token_address, &f.treasury),
        treasury_before - VERIFICATION_FEE as i128
    );
}

#[test]
fn test_double_claim_is_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let f = setup_pending_request(&env, VERIFICATION_FEE);
    f.client.reject_verification(&f.project_id, &f.admin);

    f.client.claim_fee_refund(&f.owner, &f.project_id);
    let after_first = balance(&env, &f.token_address, &f.owner);

    assert!(f
        .client
        .try_claim_fee_refund(&f.owner, &f.project_id)
        .is_err());
    assert_eq!(
        balance(&env, &f.token_address, &f.owner),
        after_first,
        "a refused second claim must not move tokens"
    );
}

#[test]
fn test_claim_without_a_refund_is_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let f = setup_pending_request(&env, VERIFICATION_FEE);

    // Never rejected, so nothing is owed.
    assert!(f
        .client
        .try_claim_fee_refund(&f.owner, &f.project_id)
        .is_err());
}

#[test]
fn test_stranger_cannot_claim() {
    let env = Env::default();
    env.mock_all_auths();
    let f = setup_pending_request(&env, VERIFICATION_FEE);
    f.client.reject_verification(&f.project_id, &f.admin);

    let stranger = Address::generate(&env);
    assert!(f
        .client
        .try_claim_fee_refund(&stranger, &f.project_id)
        .is_err());
    assert!(f
        .client
        .get_fee_refund(&f.project_id)
        .unwrap()
        .claimed_at
        .is_none());
}

#[test]
fn test_admin_may_settle_but_funds_go_to_the_payer() {
    let env = Env::default();
    env.mock_all_auths();
    let f = setup_pending_request(&env, VERIFICATION_FEE);
    f.client.reject_verification(&f.project_id, &f.admin);

    let owner_before = balance(&env, &f.token_address, &f.owner);
    let admin_before = balance(&env, &f.token_address, &f.admin);

    f.client.claim_fee_refund(&f.admin, &f.project_id);

    assert_eq!(
        balance(&env, &f.token_address, &f.owner),
        owner_before + VERIFICATION_FEE as i128,
        "an admin settling on someone's behalf must not redirect the funds"
    );
    assert_eq!(balance(&env, &f.token_address, &f.admin), admin_before);
}

// ─── Re-request after rejection ──────────────────────────────────────────────

#[test]
fn test_second_rejection_accumulates_onto_an_unclaimed_refund() {
    let env = Env::default();
    env.mock_all_auths();
    let f = setup_pending_request(&env, VERIFICATION_FEE);
    f.client.reject_verification(&f.project_id, &f.admin);

    // Pay and re-request: Rejected -> Pending is a legal transition.
    f.client
        .pay_fee(&f.owner, &f.project_id, &Some(f.token_address.clone()));
    let cid = String::from_str(&env, "QmYwAPJhy5nTAQCj9g1s2bkss7jBlEd22bN2R4s5gR5PTb");
    f.client.request_verification(&f.project_id, &f.owner, &cid);

    // Rejecting again must not be blocked by the unsettled debt, and must not
    // overwrite it either — the owner is owed both fees.
    f.client.reject_verification(&f.project_id, &f.admin);

    let refund = f.client.get_fee_refund(&f.project_id).unwrap();
    assert_eq!(refund.amount, VERIFICATION_FEE * 2);
    assert_eq!(refund.claimed_at, None);
}

#[test]
fn test_accumulated_refund_pays_out_in_full() {
    let env = Env::default();
    env.mock_all_auths();
    let f = setup_pending_request(&env, VERIFICATION_FEE);
    f.client.reject_verification(&f.project_id, &f.admin);

    f.client
        .pay_fee(&f.owner, &f.project_id, &Some(f.token_address.clone()));
    let cid = String::from_str(&env, "QmYwAPJhy5nTAQCj9g1s2bkss7jBlEd22bN2R4s5gR5PTb");
    f.client.request_verification(&f.project_id, &f.owner, &cid);
    f.client.reject_verification(&f.project_id, &f.admin);

    let before = balance(&env, &f.token_address, &f.owner);
    f.client.claim_fee_refund(&f.owner, &f.project_id);

    assert_eq!(
        balance(&env, &f.token_address, &f.owner),
        before + (VERIFICATION_FEE * 2) as i128,
        "a single claim must settle the whole accumulated debt"
    );
}

#[test]
fn test_second_rejection_records_a_refund_once_the_first_is_claimed() {
    let env = Env::default();
    env.mock_all_auths();
    let f = setup_pending_request(&env, VERIFICATION_FEE);

    f.client.reject_verification(&f.project_id, &f.admin);
    f.client.claim_fee_refund(&f.owner, &f.project_id);

    f.client
        .pay_fee(&f.owner, &f.project_id, &Some(f.token_address.clone()));
    let cid = String::from_str(&env, "QmYwAPJhy5nTAQCj9g1s2bkss7jBlEd22bN2R4s5gR5PTb");
    f.client.request_verification(&f.project_id, &f.owner, &cid);
    f.client.reject_verification(&f.project_id, &f.admin);

    let refund = f.client.get_fee_refund(&f.project_id).unwrap();
    assert_eq!(
        refund.claimed_at, None,
        "a fresh debt replaces the settled one"
    );
    assert_eq!(refund.amount, VERIFICATION_FEE);

    let before = balance(&env, &f.token_address, &f.owner);
    f.client.claim_fee_refund(&f.owner, &f.project_id);
    assert_eq!(
        balance(&env, &f.token_address, &f.owner),
        before + VERIFICATION_FEE as i128
    );
}

#[test]
fn test_refunds_are_scoped_per_project() {
    let env = Env::default();
    env.mock_all_auths();
    let f = setup_pending_request(&env, VERIFICATION_FEE);

    let other_id = create_test_project(&f.client, &f.owner, "UnaffectedProject");
    f.client.reject_verification(&f.project_id, &f.admin);

    assert!(f.client.get_fee_refund(&f.project_id).is_some());
    assert!(f.client.get_fee_refund(&other_id).is_none());
}
