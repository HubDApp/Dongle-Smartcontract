//! Tests for issue #470 – `pay_fee` guards for archived projects and active
//! verification status.
//!
//! Before this fix `pay_fee` only verified that the caller was the project
//! owner. It did not check whether the project was archived or whether
//! verification was already in-flight (Pending) or complete (Verified). In
//! either case the owner could pay a fee that could never be consumed by
//! `request_verification`, permanently locking their tokens.
//!
//! ## What these tests verify
//!
//! | Scenario | Expected error |
//! |---|---|
//! | Project is archived | `AlreadyArchived` |
//! | Verification status is `Pending` | `InvalidStatus` |
//! | Verification status is `Verified` | `InvalidStatus` |
//! | Status is `Unverified` (happy path) | `Ok(())` |
//! | Status is `Rejected` (happy path) | `Ok(())` |

#![cfg(test)]

use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use soroban_sdk::{testutils::Address as _, Address, Env, String};

// A valid IPFS CIDv0 reused across tests.
const CID: &str = "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG";

// ── Shared setup ─────────────────────────────────────────────────────────────

/// Deploy the contract, configure a non-zero verification fee backed by a real
/// Stellar Asset Contract, and return everything needed by the individual tests.
///
/// `(client, admin, owner, token_address, treasury)`
fn setup_with_fee(
    env: &Env,
) -> (
    crate::DongleContractClient<'_>,
    Address,
    Address,
    Address,
    Address,
) {
    let (client, admin) = setup_contract(env);

    let token_admin = Address::generate(env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    let treasury = Address::generate(env);

    // verification_fee = 100, registration_fee = 0 so create_test_project
    // doesn't require a prior registration-fee payment.
    client
        .mock_all_auths()
        .set_fee(&admin, &Some(token.clone()), &100u128, &0u128, &treasury);

    let owner = Address::generate(env);
    (client, admin, owner, token, treasury)
}

/// Mint `amount` units of `token` into `to`.
fn mint(env: &Env, token: &Address, to: &Address, amount: i128) {
    soroban_sdk::token::StellarAssetClient::new(env, token).mint(to, &amount);
}

// ── Guard: archived project ───────────────────────────────────────────────────

/// `pay_fee` on an archived project must return `AlreadyArchived`.
///
/// An archived project cannot transition to any verification state, so any
/// payment made against it would create an unclaimable payment record.
#[test]
fn pay_fee_archived_project_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, owner, token, _treasury) = setup_with_fee(&env);

    let project_id = create_test_project(&client, &owner, "ArchivedProject");

    // Archive the project.
    client.archive_project(&project_id, &owner);
    assert!(
        client.get_project(&project_id).unwrap().archived,
        "project must be archived before the payment attempt"
    );

    mint(&env, &token, &owner, 100);

    let err = client
        .try_pay_fee(&owner, &project_id, &Some(token.clone()))
        .unwrap_err()
        .unwrap();

    assert_eq!(
        err,
        ContractError::AlreadyArchived,
        "pay_fee on an archived project must return AlreadyArchived"
    );
}

/// Archiving the project also prevents `pay_fee` when the project previously
/// had a non-zero fee configured — token balance must remain unchanged.
#[test]
fn pay_fee_archived_project_does_not_debit_owner() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, owner, token, _treasury) = setup_with_fee(&env);
    let token_client = soroban_sdk::token::Client::new(&env, &token);

    let project_id = create_test_project(&client, &owner, "ArchivedNoDrainProject");

    client.archive_project(&project_id, &owner);
    mint(&env, &token, &owner, 100);

    let balance_before = token_client.balance(&owner);

    let _ = client.try_pay_fee(&owner, &project_id, &Some(token.clone()));

    assert_eq!(
        token_client.balance(&owner),
        balance_before,
        "owner balance must not change when pay_fee is rejected for an archived project"
    );
}

// ── Guard: Pending verification status ───────────────────────────────────────

/// `pay_fee` while the project already has a `Pending` verification request
/// must return `InvalidStatus`.
///
/// A new payment at this point would produce an orphaned record: the existing
/// pending request will consume (or reject) the current payment slot, leaving
/// the new payment with nothing to bind to.
#[test]
fn pay_fee_pending_verification_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, owner, token, _treasury) = setup_with_fee(&env);

    let project_id = create_test_project(&client, &owner, "PendingProject");

    // Pay the initial fee and submit a verification request → status = Pending.
    mint(&env, &token, &owner, 200);
    client.pay_fee(&owner, &project_id, &Some(token.clone()));
    client.request_verification(&project_id, &owner, &String::from_str(&env, CID));

    let project = client.get_project(&project_id).unwrap();
    assert_eq!(
        project.verification_status,
        crate::types::VerificationStatus::Pending,
        "project must be Pending before the second payment attempt"
    );

    // Attempt to pay again while Pending — must be rejected.
    let err = client
        .try_pay_fee(&owner, &project_id, &Some(token.clone()))
        .unwrap_err()
        .unwrap();

    assert_eq!(
        err,
        ContractError::InvalidStatus,
        "pay_fee while verification is Pending must return InvalidStatus"
    );
}

// ── Guard: Verified verification status ──────────────────────────────────────

/// `pay_fee` while the project is already `Verified` must return
/// `InvalidStatus`.
///
/// A verified project does not need another verification fee payment until the
/// current verification is revoked or expires. Accepting a payment in this
/// state would create an orphaned record.
#[test]
fn pay_fee_verified_project_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, owner, token, _treasury) = setup_with_fee(&env);

    let project_id = create_test_project(&client, &owner, "VerifiedProject");

    // Pay fee, request verification, approve → status = Verified.
    mint(&env, &token, &owner, 200);
    client.pay_fee(&owner, &project_id, &Some(token.clone()));
    client.request_verification(&project_id, &owner, &String::from_str(&env, CID));
    client.approve_verification(&project_id, &admin);

    let project = client.get_project(&project_id).unwrap();
    assert_eq!(
        project.verification_status,
        crate::types::VerificationStatus::Verified,
        "project must be Verified before the second payment attempt"
    );

    // Attempt to pay a second time while already Verified — must be rejected.
    let err = client
        .try_pay_fee(&owner, &project_id, &Some(token.clone()))
        .unwrap_err()
        .unwrap();

    assert_eq!(
        err,
        ContractError::InvalidStatus,
        "pay_fee while verification is Verified must return InvalidStatus"
    );
}

// ── Happy paths: Unverified and Rejected ─────────────────────────────────────

/// `pay_fee` on a fresh project (status `Unverified`) must succeed.
///
/// This is the primary intended workflow and must not be accidentally broken
/// by the new guards.
#[test]
fn pay_fee_unverified_project_accepted() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, owner, token, _treasury) = setup_with_fee(&env);

    let project_id = create_test_project(&client, &owner, "UnverifiedProject");

    assert_eq!(
        client.get_project(&project_id).unwrap().verification_status,
        crate::types::VerificationStatus::Unverified
    );

    mint(&env, &token, &owner, 100);
    client.pay_fee(&owner, &project_id, &Some(token.clone()));

    assert!(
        client.is_fee_paid(&project_id),
        "fee-paid flag must be set after successful pay_fee on an Unverified project"
    );
}

/// `pay_fee` on a project whose verification was previously `Rejected` must
/// succeed, because `Rejected → Pending` is a valid transition and the owner
/// should be able to re-submit.
#[test]
fn pay_fee_rejected_project_accepted() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, owner, token, _treasury) = setup_with_fee(&env);

    let project_id = create_test_project(&client, &owner, "RejectedProject");

    // Pay → request → reject: project is now Rejected.
    mint(&env, &token, &owner, 200);
    client.pay_fee(&owner, &project_id, &Some(token.clone()));
    client.request_verification(&project_id, &owner, &String::from_str(&env, CID));
    client.reject_verification(&project_id, &admin);

    assert_eq!(
        client.get_project(&project_id).unwrap().verification_status,
        crate::types::VerificationStatus::Rejected,
        "project must be Rejected before the re-payment"
    );

    // Re-pay: must succeed.
    client.pay_fee(&owner, &project_id, &Some(token.clone()));

    assert!(
        client.is_fee_paid(&project_id),
        "fee-paid flag must be set after successful re-payment on a Rejected project"
    );
}

// ── Guard: fee-paid flag is NOT set when a guard fires ────────────────────────

/// When the archived guard fires, the fee-paid flag must remain `false` — no
/// partial state must be written before the guard check.
#[test]
fn pay_fee_archived_flag_not_set_on_rejection() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, owner, token, _treasury) = setup_with_fee(&env);

    let project_id = create_test_project(&client, &owner, "ArchivedFlagCheck");
    client.archive_project(&project_id, &owner);
    mint(&env, &token, &owner, 100);

    let _ = client.try_pay_fee(&owner, &project_id, &Some(token.clone()));

    assert!(
        !client.is_fee_paid(&project_id),
        "fee-paid flag must remain false when pay_fee is rejected due to archiving"
    );
}

/// When the Pending guard fires, the fee-paid flag must remain `false` after
/// the second payment attempt (the first payment was already consumed by
/// `request_verification`).
#[test]
fn pay_fee_pending_flag_not_set_on_rejection() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, owner, token, _treasury) = setup_with_fee(&env);

    let project_id = create_test_project(&client, &owner, "PendingFlagCheck");

    mint(&env, &token, &owner, 200);
    client.pay_fee(&owner, &project_id, &Some(token.clone()));
    client.request_verification(&project_id, &owner, &String::from_str(&env, CID));
    // Flag is now false (consumed by request_verification).
    assert!(!client.is_fee_paid(&project_id));

    // Attempt second payment while Pending.
    let _ = client.try_pay_fee(&owner, &project_id, &Some(token.clone()));

    assert!(
        !client.is_fee_paid(&project_id),
        "fee-paid flag must remain false when pay_fee is rejected due to Pending status"
    );
}
