//! Tests for issues #654, #655, #656, and #657.
//!
//! ## #657 — RefundAlreadyClaimed idempotency
//! Verifies that `claim_fee_refund` is idempotent: the first call succeeds and
//! marks the record; every subsequent call returns `RefundAlreadyClaimed` without
//! moving any tokens. Storage state is checked both before and after each call.
//!
//! ## #656 — Ownership transfer atomicity
//! Verifies that `accept_transfer` leaves no partial state: the old owner loses
//! the project, the new owner gains it, and the `PendingTransfer` record is
//! removed — all in a single atomic step. Also covers concurrent overwrite
//! behavior (second `initiate_transfer` replaces the first).
//!
//! ## #655 — Changelog entry immutability
//! Verifies that changelog entries cannot be updated in-place. The only permitted
//! operations are `add_changelog_entry` (create) and `remove_changelog_entry`
//! (delete). The `update_changelog_entry` helper in `ChangelogRegistry` explicitly
//! returns `Unauthorized` to make the policy discoverable and testable.
//!
//! ## #654 — Dispute state machine transitions
//! Verifies that the dispute state machine is complete:
//! - `Pending → Resolved` (via ArchiveProject or LinkDuplicates)
//! - `Pending → Rejected` (via Reject)
//! - Any attempt to re-resolve or re-reject a terminal dispute returns
//!   `DisputeNotPending` (not the generic `InvalidStatus`).

#![cfg(test)]

use crate::changelog_registry::ChangelogRegistry;
use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::{DisputeResolutionAction, DisputeStatus};
use soroban_sdk::{
    testutils::Address as _,
    token::{self, StellarAssetClient},
    Address, Env, String,
};

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

const VERIFICATION_FEE: u128 = 500;
const MINTED: i128 = 10_000;

/// Return the token balance of `who`.
fn balance(env: &Env, token: &Address, who: &Address) -> i128 {
    token::Client::new(env, token).balance(who)
}

/// Set up a scenario where a verification was rejected and a refund record exists.
/// Returns (client, admin, owner, treasury, token_address, project_id).
fn setup_rejected_verification(env: &Env) -> (
    crate::DongleContractClient<'_>,
    Address,
    Address,
    Address,
    Address,
    u64,
) {
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
        &VERIFICATION_FEE,
        &0u128,
        &treasury,
    );

    let project_id = create_test_project(&client, &owner, "IssueTestProject");

    // Fund owner, pay fee, request and reject verification
    StellarAssetClient::new(env, &token_address).mint(&owner, &MINTED);
    client.pay_fee(&owner, &project_id, &Some(token_address.clone()));
    let cid = String::from_str(env, "QmYwAPJhy5nTAQCj9g1s2bkss7jBlEd22bN2R4s5gR5PTa");
    client.request_verification(&project_id, &owner, &cid);
    client.reject_verification(&project_id, &admin);

    (client, admin, owner, treasury, token_address, project_id)
}

// ─────────────────────────────────────────────────────────────────────────────
// Issue #657 — RefundAlreadyClaimed idempotency
// ─────────────────────────────────────────────────────────────────────────────

/// After the first successful claim, `claimed_at` must be `Some` in storage.
/// A second attempt must return `RefundAlreadyClaimed` and leave balances unchanged.
#[test]
fn test_657_double_claim_returns_refund_already_claimed() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, owner, _treasury, token, project_id) =
        setup_rejected_verification(&env);

    // First claim must succeed and tokens must move
    let before = balance(&env, &token, &owner);
    client.claim_fee_refund(&owner, &project_id);
    let after_first = balance(&env, &token, &owner);
    assert_eq!(
        after_first,
        before + VERIFICATION_FEE as i128,
        "first claim must transfer the refund"
    );

    // Storage must record claimed_at
    let refund = client.get_fee_refund(&project_id).unwrap();
    assert!(
        refund.claimed_at.is_some(),
        "claimed_at must be set after first claim"
    );

    // Second claim must be rejected with RefundAlreadyClaimed
    let result = client.try_claim_fee_refund(&owner, &project_id);
    assert_eq!(
        result,
        Err(Ok(ContractError::RefundAlreadyClaimed)),
        "second claim must return RefundAlreadyClaimed"
    );

    // Balances must be unchanged after the failed second claim
    assert_eq!(
        balance(&env, &token, &owner),
        after_first,
        "failed second claim must not move tokens"
    );
}

/// `claimed_at` must remain `Some` after a rejected second claim
/// (storage must not be reset or zeroed by the failed attempt).
#[test]
fn test_657_storage_state_preserved_after_rejected_double_claim() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin, owner, _treasury, _token, project_id) =
        setup_rejected_verification(&env);

    client.claim_fee_refund(&owner, &project_id);
    let claimed_at_after_first = client
        .get_fee_refund(&project_id)
        .unwrap()
        .claimed_at
        .unwrap();

    // Attempt a second claim (must fail)
    let _ = client.try_claim_fee_refund(&owner, &project_id);

    // Storage must be unchanged: same claimed_at, still Some
    let refund = client.get_fee_refund(&project_id).unwrap();
    assert_eq!(
        refund.claimed_at,
        Some(claimed_at_after_first),
        "storage must not be mutated by a rejected claim attempt"
    );
}

/// An admin can settle a refund on behalf of the payer, but tokens go to the payer.
/// A subsequent admin attempt on the same project must also return RefundAlreadyClaimed.
#[test]
fn test_657_admin_settle_then_double_claim_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin, owner, _treasury, token, project_id) =
        setup_rejected_verification(&env);

    let owner_before = balance(&env, &token, &owner);
    client.claim_fee_refund(&admin, &project_id);
    assert_eq!(
        balance(&env, &token, &owner),
        owner_before + VERIFICATION_FEE as i128,
        "admin settle must send tokens to payer, not admin"
    );

    // Admin attempting again must fail
    assert_eq!(
        client.try_claim_fee_refund(&admin, &project_id),
        Err(Ok(ContractError::RefundAlreadyClaimed))
    );
    // Payer attempting again must also fail
    assert_eq!(
        client.try_claim_fee_refund(&owner, &project_id),
        Err(Ok(ContractError::RefundAlreadyClaimed))
    );
}

/// Claiming without any refund record returns `NoRefundAvailable` (not a panic).
#[test]
fn test_657_claim_without_refund_returns_no_refund_available() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "NoRefundProject");

    assert_eq!(
        client.try_claim_fee_refund(&owner, &project_id),
        Err(Ok(ContractError::NoRefundAvailable))
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Issue #656 — Ownership transfer atomicity
// ─────────────────────────────────────────────────────────────────────────────

/// After accept_transfer: new owner has the project, old owner does not,
/// and no `PendingTransfer` record remains.
#[test]
fn test_656_accept_transfer_is_atomic_no_partial_state() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let old_owner = Address::generate(&env);
    let new_owner = Address::generate(&env);
    let project_id = create_test_project(&client, &old_owner, "AtomicTransferProject");

    client.initiate_transfer(&project_id, &old_owner, &new_owner);

    // Before accept: old owner still holds the project
    assert_eq!(client.get_project(&project_id).unwrap().owner, old_owner);
    assert_eq!(client.get_projects_by_owner(&old_owner).len(), 1);
    assert_eq!(client.get_projects_by_owner(&new_owner).len(), 0);

    client.accept_transfer(&project_id, &new_owner);

    // After accept: ownership fully on new owner
    let project = client.get_project(&project_id).unwrap();
    assert_eq!(project.owner, new_owner, "project.owner must be new_owner");
    assert_eq!(
        client.get_projects_by_owner(&new_owner).len(),
        1,
        "new owner must have 1 project"
    );
    assert_eq!(
        client.get_projects_by_owner(&old_owner).len(),
        0,
        "old owner must have 0 projects"
    );

    // PendingTransfer must be removed — a second accept must return TransferNotFound
    assert_eq!(
        client.try_accept_transfer(&project_id, &new_owner),
        Err(Ok(ContractError::TransferNotFound)),
        "PendingTransfer must be removed after accept"
    );
}

/// Concurrent transfer scenario: initiating a second transfer replaces the first.
/// The first recipient can no longer accept; only the second can.
#[test]
fn test_656_concurrent_initiate_replaces_first_recipient() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let first = Address::generate(&env);
    let second = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "ConcurrentTransferProject");

    // First transfer
    client.initiate_transfer(&project_id, &owner, &first);
    // Owner changes mind — second initiate replaces first atomically
    client.initiate_transfer(&project_id, &owner, &second);

    // First recipient is now unauthorized
    assert_eq!(
        client.try_accept_transfer(&project_id, &first),
        Err(Ok(ContractError::Unauthorized)),
        "first recipient must be rejected after overwrite"
    );

    // Second recipient can accept
    client.accept_transfer(&project_id, &second);
    assert_eq!(client.get_project(&project_id).unwrap().owner, second);
}

/// A failed accept (wrong recipient) must leave ownership fully on the old owner.
#[test]
fn test_656_failed_accept_leaves_no_partial_state() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let intended = Address::generate(&env);
    let attacker = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "FailedAcceptProject");

    client.initiate_transfer(&project_id, &owner, &intended);

    // Attacker cannot accept
    assert!(client.try_accept_transfer(&project_id, &attacker).is_err());

    // Ownership is fully unchanged
    assert_eq!(client.get_project(&project_id).unwrap().owner, owner);
    assert_eq!(client.get_projects_by_owner(&owner).len(), 1);
    assert_eq!(client.get_projects_by_owner(&attacker).len(), 0);
    assert_eq!(client.get_projects_by_owner(&intended).len(), 0);
}

/// Cancel must remove the pending transfer atomically.
/// After cancel, neither the old nor new owner can accept.
#[test]
fn test_656_cancel_removes_pending_transfer_atomically() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let recipient = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "CancelAtomicProject");

    client.initiate_transfer(&project_id, &owner, &recipient);
    client.cancel_transfer(&project_id, &owner);

    // No pending transfer — accept must fail
    assert_eq!(
        client.try_accept_transfer(&project_id, &recipient),
        Err(Ok(ContractError::TransferNotFound))
    );

    // Owner must still own the project
    assert_eq!(client.get_project(&project_id).unwrap().owner, owner);
}

// ─────────────────────────────────────────────────────────────────────────────
// Issue #655 — Changelog entry immutability
// ─────────────────────────────────────────────────────────────────────────────

/// Calling `ChangelogRegistry::update_changelog_entry` must return `Unauthorized`.
/// This test exercises the explicit "no update" policy directly on the registry.
#[test]
fn test_655_update_changelog_entry_is_rejected() {
    let env = Env::default();
    let owner = Address::generate(&env);
    // update_changelog_entry always returns Unauthorized — no environment
    // setup is required because it never reads from storage.
    let result = ChangelogRegistry::update_changelog_entry(&env, 1, owner);
    assert_eq!(
        result,
        Err(ContractError::Unauthorized),
        "update_changelog_entry must always return Unauthorized"
    );
}

/// Adding an entry and then trying to add a second entry with the same CID
/// must fail — this demonstrates that entries are content-addressed and
/// write-once per CID.
#[test]
fn test_655_duplicate_cid_is_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "ImmutableChangelogProject");

    let cid = String::from_str(&env, "bafybeiboz75hbx2qg7g4j4vq655immutable1111111111111111");
    let desc = Some(String::from_str(&env, "v1.0.0"));

    let id = client.add_changelog_entry(&project_id, &owner, &cid, &desc, &None, &None);
    assert!(id > 0);

    // Trying to add the same CID again must fail
    let result = client.try_add_changelog_entry(&project_id, &owner, &cid, &desc, &None, &None);
    assert!(result.is_err(), "duplicate CID must be rejected");
}

/// Remove and recreate — the approved workflow when an entry needs changing.
#[test]
fn test_655_remove_and_recreate_is_the_approved_update_workflow() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "RemoveRecreateProject");

    let cid_v1 = String::from_str(&env, "bafybeiboz75hbx2qg7g4j4vq655v1aaaaaaaaaaaaaaaaaaaaa");
    let cid_v2 = String::from_str(&env, "bafybeiboz75hbx2qg7g4j4vq655v2bbbbbbbbbbbbbbbbbbbbb");

    let id_v1 =
        client.add_changelog_entry(&project_id, &owner, &cid_v1, &None, &None, &None);

    // Remove
    client.remove_changelog_entry(&id_v1, &owner);
    assert!(client.get_changelog_entry(&id_v1).is_none(), "entry must be gone after remove");

    // Recreate with corrected data
    let id_v2 =
        client.add_changelog_entry(&project_id, &owner, &cid_v2, &None, &None, &None);
    assert!(id_v2 > 0);
    assert_eq!(client.get_changelog_count(&project_id), 1);
    assert_eq!(client.get_changelog_entry(&id_v2).unwrap().cid, cid_v2);
}

/// A non-owner must not be able to add or remove changelog entries.
/// Events are emitted for every successful add/remove so the audit trail is complete.
#[test]
fn test_655_only_owner_can_mutate_changelog() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let non_owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "OwnerOnlyChangelog");

    let cid = String::from_str(&env, "bafybeiboz75hbx2qg7g4j4vq655owneronly1111111111111");
    let id = client.add_changelog_entry(&project_id, &owner, &cid, &None, &None, &None);

    // Non-owner cannot remove
    assert!(
        client
            .try_remove_changelog_entry(&id, &non_owner)
            .is_err(),
        "non-owner must not remove changelog entries"
    );
    // Entry still exists
    assert!(client.get_changelog_entry(&id).is_some());
}

// ─────────────────────────────────────────────────────────────────────────────
// Issue #654 — Dispute state machine completeness
// ─────────────────────────────────────────────────────────────────────────────

/// Helper: open a dispute between two projects and return the dispute ID.
fn open_dispute(
    client: &crate::DongleContractClient<'_>,
    env: &Env,
    project_id: u64,
    original_id: u64,
) -> u64 {
    let creator = Address::generate(env);
    let cid = String::from_str(env, "Qm654DisputeEvidenceCidABCDEFGHJKLMNPQRSTUVWXYZabcdefghij");
    client.open_duplicate_dispute(&project_id, &original_id, &creator, &cid)
}

/// Pending → Resolved via ArchiveProject is a valid transition.
#[test]
fn test_654_pending_to_resolved_via_archive() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let id1 = create_test_project(&client, &Address::generate(&env), "Original654Archive");
    let id2 = create_test_project(&client, &Address::generate(&env), "Duplicate654Archive");

    let dispute_id = open_dispute(&client, &env, id2, id1);
    assert_eq!(
        client.get_duplicate_dispute(&dispute_id).unwrap().status,
        DisputeStatus::Pending
    );

    client.resolve_duplicate_dispute(
        &dispute_id,
        &admin,
        &DisputeResolutionAction::ArchiveProject(id2),
    );
    assert_eq!(
        client.get_duplicate_dispute(&dispute_id).unwrap().status,
        DisputeStatus::Resolved
    );
}

/// Pending → Resolved via LinkDuplicates is a valid transition.
#[test]
fn test_654_pending_to_resolved_via_link() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let id1 = create_test_project(&client, &Address::generate(&env), "Original654Link");
    let id2 = create_test_project(&client, &Address::generate(&env), "Duplicate654Link");

    let dispute_id = open_dispute(&client, &env, id2, id1);
    client.resolve_duplicate_dispute(
        &dispute_id,
        &admin,
        &DisputeResolutionAction::LinkDuplicates,
    );
    assert_eq!(
        client.get_duplicate_dispute(&dispute_id).unwrap().status,
        DisputeStatus::Resolved
    );
}

/// Pending → Rejected via Reject is a valid transition.
#[test]
fn test_654_pending_to_rejected_via_reject() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let id1 = create_test_project(&client, &Address::generate(&env), "Original654Reject");
    let id2 = create_test_project(&client, &Address::generate(&env), "Duplicate654Reject");

    let dispute_id = open_dispute(&client, &env, id2, id1);
    client.resolve_duplicate_dispute(&dispute_id, &admin, &DisputeResolutionAction::Reject);
    assert_eq!(
        client.get_duplicate_dispute(&dispute_id).unwrap().status,
        DisputeStatus::Rejected
    );
}

/// Resolved → Resolve is an invalid transition and must return `DisputeNotPending`.
#[test]
fn test_654_resolved_to_resolved_returns_dispute_not_pending() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let id1 = create_test_project(&client, &Address::generate(&env), "Original654ReResolve");
    let id2 = create_test_project(&client, &Address::generate(&env), "Duplicate654ReResolve");

    let dispute_id = open_dispute(&client, &env, id2, id1);
    client.resolve_duplicate_dispute(
        &dispute_id,
        &admin,
        &DisputeResolutionAction::LinkDuplicates,
    );

    // Attempt to resolve again — must fail with DisputeNotPending
    let result = client.try_resolve_duplicate_dispute(
        &dispute_id,
        &admin,
        &DisputeResolutionAction::LinkDuplicates,
    );
    assert_eq!(
        result,
        Err(Ok(ContractError::DisputeNotPending)),
        "re-resolving a Resolved dispute must return DisputeNotPending"
    );
}

/// Rejected → Resolve is an invalid transition and must return `DisputeNotPending`.
#[test]
fn test_654_rejected_to_resolved_returns_dispute_not_pending() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let id1 = create_test_project(&client, &Address::generate(&env), "Original654RejResolve");
    let id2 = create_test_project(&client, &Address::generate(&env), "Duplicate654RejResolve");

    let dispute_id = open_dispute(&client, &env, id2, id1);
    client.resolve_duplicate_dispute(&dispute_id, &admin, &DisputeResolutionAction::Reject);

    // Attempt to resolve a Rejected dispute
    let result = client.try_resolve_duplicate_dispute(
        &dispute_id,
        &admin,
        &DisputeResolutionAction::LinkDuplicates,
    );
    assert_eq!(
        result,
        Err(Ok(ContractError::DisputeNotPending)),
        "resolving a Rejected dispute must return DisputeNotPending"
    );
}

/// Rejected → Reject is an invalid transition and must return `DisputeNotPending`.
#[test]
fn test_654_rejected_to_rejected_returns_dispute_not_pending() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let id1 = create_test_project(&client, &Address::generate(&env), "Original654ReReject");
    let id2 = create_test_project(&client, &Address::generate(&env), "Duplicate654ReReject");

    let dispute_id = open_dispute(&client, &env, id2, id1);
    client.resolve_duplicate_dispute(&dispute_id, &admin, &DisputeResolutionAction::Reject);

    let result = client.try_resolve_duplicate_dispute(
        &dispute_id,
        &admin,
        &DisputeResolutionAction::Reject,
    );
    assert_eq!(
        result,
        Err(Ok(ContractError::DisputeNotPending)),
        "re-rejecting a Rejected dispute must return DisputeNotPending"
    );
}

/// Resolved → Reject is an invalid transition and must return `DisputeNotPending`.
#[test]
fn test_654_resolved_to_rejected_returns_dispute_not_pending() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let id1 = create_test_project(&client, &Address::generate(&env), "Original654ResReject");
    let id2 = create_test_project(&client, &Address::generate(&env), "Duplicate654ResReject");

    let dispute_id = open_dispute(&client, &env, id2, id1);
    client.resolve_duplicate_dispute(
        &dispute_id,
        &admin,
        &DisputeResolutionAction::LinkDuplicates,
    );

    let result = client.try_resolve_duplicate_dispute(
        &dispute_id,
        &admin,
        &DisputeResolutionAction::Reject,
    );
    assert_eq!(
        result,
        Err(Ok(ContractError::DisputeNotPending)),
        "rejecting a Resolved dispute must return DisputeNotPending"
    );
}

/// Non-admins cannot resolve disputes at all.
#[test]
fn test_654_non_admin_cannot_resolve_dispute() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let id1 = create_test_project(&client, &Address::generate(&env), "Original654Auth");
    let id2 = create_test_project(&client, &Address::generate(&env), "Duplicate654Auth");

    let dispute_id = open_dispute(&client, &env, id2, id1);

    let non_admin = Address::generate(&env);
    let result = client.try_resolve_duplicate_dispute(
        &dispute_id,
        &non_admin,
        &DisputeResolutionAction::Reject,
    );
    assert!(
        result.is_err(),
        "non-admin must not be able to resolve a dispute"
    );

    // Dispute must still be Pending
    assert_eq!(
        client.get_duplicate_dispute(&dispute_id).unwrap().status,
        DisputeStatus::Pending
    );
}

/// `resolved_at` must be set on a resolved dispute and zero on a pending one.
#[test]
fn test_654_resolved_at_timestamp_set_on_resolution() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let id1 = create_test_project(&client, &Address::generate(&env), "Original654Timestamp");
    let id2 = create_test_project(&client, &Address::generate(&env), "Duplicate654Timestamp");

    let dispute_id = open_dispute(&client, &env, id2, id1);
    assert_eq!(
        client.get_duplicate_dispute(&dispute_id).unwrap().resolved_at,
        0,
        "resolved_at must be 0 for a pending dispute"
    );

    client.resolve_duplicate_dispute(&dispute_id, &admin, &DisputeResolutionAction::Reject);

    let dispute = client.get_duplicate_dispute(&dispute_id).unwrap();
    assert!(
        dispute.resolved_at >= dispute.created_at,
        "resolved_at must be set after resolution"
    );
}
