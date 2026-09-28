//! Regression coverage for the typed-error and admin-log guarantees behind
//! issues #458, #463, #465 and #466.
//!
//! # Why these tests exist
//!
//! Each of those issues reports a defect that is **already fixed in the
//! contract** — `initialize` returns `AlreadyInitialized` rather than
//! panicking, and `CannotLinkToSelf`, `TreasuryNotSet`, `FeeConfigNotSet`,
//! `InsufficientFee` and `AdminActionType::VerificationAssigned` are all
//! defined and wired up. What is missing is anything that *holds* them fixed.
//!
//! Checking the live suite (only the modules actually declared in
//! `tests/mod.rs` — `mod fee;` and `mod atomicity;` are commented out, so the
//! assertions in those files never compile) leaves:
//!
//! | Guarantee | Live coverage before this file |
//! |---|---|
//! | `AlreadyInitialized` | none |
//! | `CannotLinkToSelf` | project-link path only |
//! | `FeeConfigNotSet` | none |
//! | `TreasuryNotSet` | none |
//! | `VerificationAssigned` in the admin log | none |
//!
//! So reverting `initialize` to `panic!` — exactly what #458 describes —
//! would not fail a single test today. These pin the behaviour instead.

use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::AdminActionType;
use crate::ContractError;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

// ── #458: initialize returns a typed error instead of panicking ─────────────

/// A second `initialize` must return `AlreadyInitialized` as a value.
///
/// `try_initialize` is the point of the test: it can only observe a returned
/// `Err`. If `initialize` went back to `panic!`, the host would trap and this
/// would fail rather than reporting a comparison mismatch — which is the
/// distinction #458 is about.
#[test]
fn test_second_initialize_returns_already_initialized() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);

    let intruder = Address::generate(&env);
    let result = client.mock_all_auths().try_initialize(&intruder);

    assert_eq!(result, Err(Ok(ContractError::AlreadyInitialized)));
}

/// The rejected re-initialize must not disturb the existing admin set.
///
/// A panic would roll the transaction back and leave state intact too, so this
/// alone does not prove the typed-error path — it guards the other half: that
/// returning an error early did not partially overwrite the admin list.
#[test]
fn test_failed_reinitialize_leaves_admin_set_untouched() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let intruder = Address::generate(&env);
    let _ = client.mock_all_auths().try_initialize(&intruder);

    assert!(client.is_admin(&admin), "original admin must survive");
    assert!(
        !client.is_admin(&intruder),
        "a rejected initialize must not grant admin"
    );
    assert_eq!(client.get_admin_count(), 1);
}

// ── #463: CannotLinkToSelf on the duplicate-dispute path ────────────────────

/// Opening a duplicate dispute of a project against itself is rejected.
///
/// `CannotLinkToSelf` is raised from two places: `project_registry` (covered by
/// `linked_projects.rs`) and `dispute_registry::open_duplicate_dispute`, which
/// had no live coverage. This is the second path.
#[test]
fn test_duplicate_dispute_against_self_is_rejected() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let creator = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "SelfDisputeProject");
    let evidence_cid = String::from_str(&env, "Qm123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmn");

    let result = client.mock_all_auths().try_open_duplicate_dispute(
        &project_id,
        &project_id,
        &creator,
        &evidence_cid,
    );

    assert_eq!(result, Err(Ok(ContractError::CannotLinkToSelf)));
}

/// The rejected self-dispute must not have been recorded.
#[test]
fn test_rejected_self_dispute_creates_no_record() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let creator = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "SelfDisputeNoRecord");
    let evidence_cid = String::from_str(&env, "Qm123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmn");

    let _ = client.mock_all_auths().try_open_duplicate_dispute(
        &project_id,
        &project_id,
        &creator,
        &evidence_cid,
    );

    assert_eq!(
        client.get_disputes_for_project(&project_id).len(),
        0,
        "a rejected dispute must not be stored"
    );
}

// ── #465: fee config / treasury typed errors ────────────────────────────────

/// Paying a fee before any fee config exists returns `FeeConfigNotSet`.
///
/// The assertion for this lived in `tests/fee.rs`, which is not declared in
/// `tests/mod.rs` and therefore never compiled.
#[test]
fn test_pay_fee_without_config_returns_fee_config_not_set() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "NoFeeConfigProject");

    let result = client
        .mock_all_auths()
        .try_pay_fee(&owner, &project_id, &None);

    assert_eq!(result, Err(Ok(ContractError::FeeConfigNotSet)));
}

/// `set_fee` writes the fee config and the treasury together.
///
/// This is what keeps `TreasuryNotSet` (#465) unreachable from the public API:
/// no entry point stores a `FeeConfig` without also storing a `Treasury`, so
/// the guard in `execute_fee_payment` is defensive rather than a reachable
/// state. Pinning the invariant here means that if a future change ever splits
/// those two writes, the error becomes reachable and someone has to decide
/// about it deliberately instead of meeting it in production.
#[test]
fn test_set_fee_stores_config_and_treasury_together() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let treasury = Address::generate(&env);

    // Before set_fee: no config, and no treasury.
    assert_eq!(
        client.try_get_fee_config(),
        Err(Ok(ContractError::FeeConfigNotSet))
    );
    assert_eq!(client.get_config().treasury, None);

    client.set_fee(&admin, &None, &100u128, &50u128, &treasury);

    // After a single set_fee: both are present. Neither can exist without the
    // other, which is precisely why TreasuryNotSet has no reachable path.
    let config = client.get_fee_config();
    assert_eq!(config.verification_fee, 100u128);
    assert_eq!(config.registration_fee, 50u128);
    assert_eq!(client.get_config().treasury, Some(treasury));
}

// ── #466: VerificationAssigned reaches the admin action log ─────────────────

/// `assign_verification` records an `AdminActionType::VerificationAssigned`
/// entry naming the assigning admin and the project.
#[test]
fn test_assign_verification_records_admin_action() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    // The assignee must itself be an admin — assign_verification rejects a
    // non-admin with AdminNotFound.
    let assignee = Address::generate(&env);
    client.mock_all_auths().add_admin(&admin, &assignee);
    let project_id = create_test_project(&client, &owner, "AssignVerificationProject");
    let evidence_cid = String::from_str(&env, "Qm123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmn");

    client
        .mock_all_auths()
        .request_verification(&project_id, &owner, &evidence_cid);

    let before = client.get_admin_action_log_count();

    client
        .mock_all_auths()
        .assign_verification(&project_id, &admin, &assignee);

    assert_eq!(
        client.get_admin_action_log_count(),
        before + 1,
        "assigning verification must append exactly one log entry"
    );

    let entries = client.list_admin_actions(&0u32, &100u32);
    let assigned = entries
        .iter()
        .find(|e| e.action_type == AdminActionType::VerificationAssigned)
        .expect("VerificationAssigned entry must be present in the admin action log");

    assert_eq!(assigned.admin, admin, "entry must name the assigning admin");
    assert_eq!(
        assigned.target_id,
        Some(project_id),
        "entry must name the project the assignment was for"
    );
}

/// The assignment is also readable back through `get_assigned_admin`, so the
/// log entry and the stored record agree.
#[test]
fn test_assign_verification_sets_assigned_admin() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let assignee = Address::generate(&env);
    client.mock_all_auths().add_admin(&admin, &assignee);
    let project_id = create_test_project(&client, &owner, "AssignedAdminProject");
    let evidence_cid = String::from_str(&env, "Qm123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmn");

    client
        .mock_all_auths()
        .request_verification(&project_id, &owner, &evidence_cid);
    client
        .mock_all_auths()
        .assign_verification(&project_id, &admin, &assignee);

    assert_eq!(client.get_assigned_admin(&project_id), Some(assignee));
}
