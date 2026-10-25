//! Issue #970 — Comprehensive authorization testing.
//!
//! Verifies that every access-control boundary in the contract is enforced:
//!
//! * Admin-only functions reject callers who are not admins.
//! * Owner-only functions reject callers who are not the project owner.
//! * Multi-sig/threshold requirements are enforced for admin removal when
//!   the admin count would drop to zero.
//! * Timelock: operations that require a delay cannot be executed before
//!   the delay has elapsed.
//! * Pause state: mutating operations are rejected while the contract is
//!   paused.

use crate::errors::ContractError;
use crate::types::{ProjectRegistrationParams, ProjectUpdateParams};
use crate::{DongleContract, DongleContractClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    Address, Env, String,
};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn setup(env: &Env) -> (DongleContractClient<'_>, Address) {
    let contract_id = env.register(DongleContract, ());
    let client = DongleContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.mock_all_auths().initialize(&admin);
    (client, admin)
}

fn register_project(client: &DongleContractClient<'_>, owner: &Address, name: &str) -> u64 {
    let env = &client.env;
    let slug = name.to_lowercase().replace(' ', "-");
    client
        .mock_all_auths()
        .register_project(&ProjectRegistrationParams {
            owner: owner.clone(),
            name: String::from_str(env, name),
            slug: String::from_str(env, &slug),
            description: String::from_str(env, "Description"),
            category: String::from_str(env, "DeFi"),
            website: None,
            license: None,
            logo_cid: None,
            metadata_cid: None,
            tags: None,
            social_links: None,
            launch_timestamp: None,
            bounty_url: None,
            repository_url: None,
            language_code: None,
            verification_requirements: None,
        })
}

// ── Admin-only: add_admin ─────────────────────────────────────────────────────

#[test]
fn auth_add_admin_rejected_for_non_admin_caller() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let stranger = Address::generate(&env);
    let target = Address::generate(&env);

    let result = client.mock_all_auths().try_add_admin(&stranger, &target);
    assert_eq!(
        result,
        Err(Ok(ContractError::AdminOnly)),
        "add_admin must reject a non-admin caller with AdminOnly"
    );
    assert!(
        !client.is_admin(&target),
        "the target must not have been added as admin"
    );
}

// ── Admin-only: remove_admin ──────────────────────────────────────────────────

#[test]
fn auth_remove_admin_rejected_for_non_admin_caller() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let stranger = Address::generate(&env);

    let result = client.mock_all_auths().try_remove_admin(&stranger, &admin);
    assert_eq!(
        result,
        Err(Ok(ContractError::AdminOnly)),
        "remove_admin must reject a non-admin caller"
    );
    assert!(
        client.is_admin(&admin),
        "the admin must remain registered after a failed removal attempt"
    );
}

// ── Admin-only: set_featured ──────────────────────────────────────────────────

#[test]
fn auth_set_featured_rejected_for_non_admin_caller() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);
    let stranger = Address::generate(&env);
    let project_id = register_project(&client, &owner, "AuthFeatured970");

    let result = client
        .mock_all_auths()
        .try_set_featured(&stranger, &project_id, &true);
    assert_eq!(
        result,
        Err(Ok(ContractError::AdminOnly)),
        "set_featured must reject a non-admin caller"
    );
}

// ── Admin-only: approve_verification ─────────────────────────────────────────

#[test]
fn auth_approve_verification_rejected_for_non_admin_caller() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);
    let stranger = Address::generate(&env);
    let project_id = register_project(&client, &owner, "AuthApprVer970");

    client.mock_all_auths().set_min_project_age(&admin, &0);
    let evidence = String::from_str(&env, "QmEvidence970ApprVerTest12345678901234567890");
    client
        .mock_all_auths()
        .request_verification(&project_id, &owner, &evidence);

    let result = client
        .mock_all_auths()
        .try_approve_verification(&project_id, &stranger);
    assert_eq!(
        result,
        Err(Ok(ContractError::AdminOnly)),
        "approve_verification must reject a non-admin caller"
    );
}

// ── Admin-only: reject_verification ──────────────────────────────────────────

#[test]
fn auth_reject_verification_rejected_for_non_admin_caller() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);
    let stranger = Address::generate(&env);
    let project_id = register_project(&client, &owner, "AuthRejVer970");

    client.mock_all_auths().set_min_project_age(&admin, &0);
    let evidence = String::from_str(&env, "QmEvidence970RejVerTest1234567890123456789012");
    client
        .mock_all_auths()
        .request_verification(&project_id, &owner, &evidence);

    let result = client
        .mock_all_auths()
        .try_reject_verification(&project_id, &stranger);
    assert_eq!(
        result,
        Err(Ok(ContractError::AdminOnly)),
        "reject_verification must reject a non-admin caller"
    );
}

// ── Admin-only: hide_review ───────────────────────────────────────────────────

#[test]
fn auth_hide_review_rejected_for_non_admin_caller() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let stranger = Address::generate(&env);
    let project_id = register_project(&client, &owner, "AuthHideRev970");

    client
        .mock_all_auths()
        .add_review(&project_id, &reviewer, &4, &None);

    let result = client
        .mock_all_auths()
        .try_hide_review(&project_id, &reviewer, &stranger);
    assert_eq!(
        result,
        Err(Ok(ContractError::AdminOnly)),
        "hide_review must reject a non-admin caller"
    );
}

// ── Admin-only: admin_delete_review ──────────────────────────────────────────

#[test]
fn auth_admin_delete_review_rejected_for_non_admin_caller() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let stranger = Address::generate(&env);
    let project_id = register_project(&client, &owner, "AuthAdmDelRev970");

    client
        .mock_all_auths()
        .add_review(&project_id, &reviewer, &4, &None);

    let result = client
        .mock_all_auths()
        .try_admin_delete_review(&project_id, &reviewer, &stranger);
    assert_eq!(
        result,
        Err(Ok(ContractError::AdminOnly)),
        "admin_delete_review must reject a non-admin caller"
    );
}

// ── Owner-only: update_project ────────────────────────────────────────────────

#[test]
fn auth_update_project_rejected_for_non_owner_caller() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let owner = Address::generate(&env);
    let attacker = Address::generate(&env);
    let project_id = register_project(&client, &owner, "AuthUpdProj970");

    let result = client.mock_all_auths().try_update_project(&ProjectUpdateParams {
        project_id,
        caller: attacker.clone(),
        name: Some(String::from_str(&env, "Hijacked")),
        slug: None,
        description: None,
        category: None,
        website: None,
        license: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
        repository_url: None,
        language_code: None,
        verification_requirements: None,
    });

    assert_eq!(
        result,
        Err(Ok(ContractError::Unauthorized)),
        "update_project must reject a caller who is not the project owner"
    );

    let project = client.get_project(&project_id).unwrap();
    assert_eq!(
        project.name,
        String::from_str(&env, "AuthUpdProj970"),
        "project name must be unchanged after a rejected update"
    );
}

// ── Owner-only: archive_project ───────────────────────────────────────────────

#[test]
fn auth_archive_project_rejected_for_non_owner_caller() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let owner = Address::generate(&env);
    let attacker = Address::generate(&env);
    let project_id = register_project(&client, &owner, "AuthArcProj970");

    let result = client
        .mock_all_auths()
        .try_archive_project(&project_id, &attacker);
    assert_eq!(
        result,
        Err(Ok(ContractError::Unauthorized)),
        "archive_project must reject a caller who is not the project owner"
    );
}

// ── Owner-only: initiate_transfer ────────────────────────────────────────────

#[test]
fn auth_initiate_transfer_rejected_for_non_owner_caller() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let owner = Address::generate(&env);
    let attacker = Address::generate(&env);
    let recipient = Address::generate(&env);
    let project_id = register_project(&client, &owner, "AuthTransProj970");

    let result = client
        .mock_all_auths()
        .try_initiate_transfer(&project_id, &attacker, &recipient);
    assert_eq!(
        result,
        Err(Ok(ContractError::Unauthorized)),
        "initiate_transfer must reject a non-owner caller"
    );
}

// ── Owner-only: delete_review ─────────────────────────────────────────────────

#[test]
fn auth_delete_review_rejected_for_non_reviewer_caller() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let owner = Address::generate(&env);
    let reviewer = Address::generate(&env);
    let attacker = Address::generate(&env);
    let project_id = register_project(&client, &owner, "AuthDelRev970");

    client
        .mock_all_auths()
        .add_review(&project_id, &reviewer, &5, &None);

    let result = client
        .mock_all_auths()
        .try_delete_review(&project_id, &attacker);
    assert_eq!(
        result,
        Err(Ok(ContractError::ReviewNotFound)),
        "delete_review must reject a caller who has not submitted a review"
    );

    // Original review must still exist.
    let reviews = client.get_reviews(&project_id, &None, &None);
    assert_eq!(
        reviews.reviews.len(),
        1,
        "the original review must remain after a failed deletion by a non-reviewer"
    );
}

// ── Multi-sig: cannot remove last admin ──────────────────────────────────────

/// The contract must never be left without at least one admin.
/// Attempting to remove the sole admin must return `CannotRemoveLastAdmin`.
#[test]
fn auth_multi_sig_cannot_remove_last_admin() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    let result = client.mock_all_auths().try_remove_admin(&admin, &admin);
    assert_eq!(
        result,
        Err(Ok(ContractError::CannotRemoveLastAdmin)),
        "removing the last admin must return CannotRemoveLastAdmin"
    );

    assert_eq!(
        client.get_admin_count(),
        1,
        "admin count must remain 1 after a failed last-admin removal"
    );
    assert!(
        client.is_admin(&admin),
        "the sole admin must still be registered after a failed removal"
    );
}

/// A second admin can remove the first, but only if at least one admin
/// remains after the operation.
#[test]
fn auth_multi_sig_second_admin_can_remove_first_leaving_one_admin() {
    let env = Env::default();
    let (client, admin1) = setup(&env);
    let admin2 = Address::generate(&env);

    client.mock_all_auths().add_admin(&admin1, &admin2);
    assert_eq!(client.get_admin_count(), 2);

    // admin2 removes admin1 — this leaves admin2 as the sole admin, which is valid.
    client.mock_all_auths().remove_admin(&admin2, &admin1);

    assert_eq!(
        client.get_admin_count(),
        1,
        "admin count must drop to 1 after valid removal"
    );
    assert!(
        client.is_admin(&admin2),
        "admin2 must still be registered as the remaining admin"
    );
    assert!(
        !client.is_admin(&admin1),
        "admin1 must have been removed"
    );
}

// ── Timelock enforcement ──────────────────────────────────────────────────────

/// `request_verification` enforces a minimum project age (set via
/// `set_min_project_age`).  A project registered at ledger time T must not
/// be verifiable until at least T + min_age seconds have elapsed.
#[test]
fn auth_timelock_verification_request_blocked_before_min_project_age() {
    let env = Env::default();

    env.ledger().set(LedgerInfo {
        timestamp: 1_000_000,
        protocol_version: 22,
        sequence_number: 1,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 16,
        min_persistent_entry_ttl: 4096,
        max_entry_ttl: 6_312_000,
    });

    let contract_id = env.register(DongleContract, ());
    let client = DongleContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.mock_all_auths().initialize(&admin);

    // Require projects to be at least 7 days old before verification.
    let min_age_secs: u64 = 7 * 24 * 3600; // 604 800 s
    client
        .mock_all_auths()
        .set_min_project_age(&admin, &min_age_secs);

    let owner = Address::generate(&env);
    let project_id = register_project(&client, &owner, "AuthTimelock970");

    // Attempt to request verification immediately (project age = 0 s < 604 800 s).
    let evidence = String::from_str(&env, "QmEvidence970TimelockTest1234567890123456789012");
    let result_too_early = client
        .mock_all_auths()
        .try_request_verification(&project_id, &owner, &evidence);

    assert_eq!(
        result_too_early,
        Err(Ok(ContractError::ProjectTooYoung)),
        "request_verification must be blocked before min_project_age has elapsed"
    );

    // Advance the ledger past the minimum age.
    env.ledger().set(LedgerInfo {
        timestamp: 1_000_000 + min_age_secs + 1,
        protocol_version: 22,
        sequence_number: 2,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 16,
        min_persistent_entry_ttl: 4096,
        max_entry_ttl: 6_312_000,
    });

    let result_after_delay = client
        .mock_all_auths()
        .try_request_verification(&project_id, &owner, &evidence);

    assert!(
        result_after_delay.is_ok(),
        "request_verification must succeed once min_project_age has elapsed"
    );
}

// ── Pause state authorization ─────────────────────────────────────────────────

/// While the contract is paused, mutating operations must be rejected.
/// `register_project` is used as the representative mutating call.
#[test]
fn auth_pause_state_blocks_register_project() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    client.mock_all_auths().pause(&admin);

    let owner = Address::generate(&env);
    let result = client.mock_all_auths().try_register_project(
        &ProjectRegistrationParams {
            owner: owner.clone(),
            name: String::from_str(&env, "PausedProject970"),
            slug: String::from_str(&env, "pausedproject970"),
            description: String::from_str(&env, "Should be blocked"),
            category: String::from_str(&env, "DeFi"),
            website: None,
            license: None,
            logo_cid: None,
            metadata_cid: None,
            tags: None,
            social_links: None,
            launch_timestamp: None,
            bounty_url: None,
            repository_url: None,
            language_code: None,
            verification_requirements: None,
        },
    );

    assert!(
        result.is_err(),
        "register_project must be blocked while the contract is paused"
    );
}

/// After unpausing the contract, the same call that was rejected while
/// paused must now succeed.
#[test]
fn auth_pause_state_unblock_after_unpause() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    client.mock_all_auths().pause(&admin);
    client.mock_all_auths().unpause(&admin);

    let owner = Address::generate(&env);
    let result = client.mock_all_auths().try_register_project(
        &ProjectRegistrationParams {
            owner: owner.clone(),
            name: String::from_str(&env, "UnpausedProject970"),
            slug: String::from_str(&env, "unpausedproject970"),
            description: String::from_str(&env, "Should succeed"),
            category: String::from_str(&env, "DeFi"),
            website: None,
            license: None,
            logo_cid: None,
            metadata_cid: None,
            tags: None,
            social_links: None,
            launch_timestamp: None,
            bounty_url: None,
            repository_url: None,
            language_code: None,
            verification_requirements: None,
        },
    );

    assert!(
        result.is_ok(),
        "register_project must succeed after the contract has been unpaused"
    );
}

/// Only an admin may pause the contract.
#[test]
fn auth_pause_contract_rejected_for_non_admin_caller() {
    let env = Env::default();
    let (client, _admin) = setup(&env);
    let stranger = Address::generate(&env);

    let result = client.mock_all_auths().try_pause(&stranger);
    assert_eq!(
        result,
        Err(Ok(ContractError::AdminOnly)),
        "pause_contract must reject a non-admin caller"
    );
}

/// Only an admin may unpause the contract.
#[test]
fn auth_unpause_contract_rejected_for_non_admin_caller() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let stranger = Address::generate(&env);

    client.mock_all_auths().pause(&admin);

    let result = client.mock_all_auths().try_unpause(&stranger);
    assert_eq!(
        result,
        Err(Ok(ContractError::AdminOnly)),
        "unpause_contract must reject a non-admin caller"
    );
}

// ── Owner-only: request_verification belongs to project owner ────────────────

#[test]
fn auth_request_verification_rejected_for_non_owner_caller() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);
    let stranger = Address::generate(&env);
    let project_id = register_project(&client, &owner, "AuthVerReq970Own");

    client.mock_all_auths().set_min_project_age(&admin, &0);
    let evidence = String::from_str(&env, "QmEvidence970OwnTest12345678901234567890123456");

    let result = client
        .mock_all_auths()
        .try_request_verification(&project_id, &stranger, &evidence);
    assert_eq!(
        result,
        Err(Ok(ContractError::Unauthorized)),
        "request_verification must reject a caller who is not the project owner"
    );
}

// ── Admin vs owner separation ─────────────────────────────────────────────────

/// An admin who is NOT the project owner must not be able to update
/// project metadata through `update_project`.
#[test]
fn auth_admin_cannot_update_project_they_do_not_own() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);
    let project_id = register_project(&client, &owner, "AuthAdminUpd970");

    // Admin tries to update the project (admin ≠ owner).
    let result = client.mock_all_auths().try_update_project(&ProjectUpdateParams {
        project_id,
        caller: admin.clone(),
        name: Some(String::from_str(&env, "AdminHijack")),
        slug: None,
        description: None,
        category: None,
        website: None,
        license: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
        repository_url: None,
        language_code: None,
        verification_requirements: None,
    });

    assert_eq!(
        result,
        Err(Ok(ContractError::Unauthorized)),
        "admin must not be able to update a project they do not own"
    );
}

/// The project owner (who is not an admin) must not be able to call
/// admin-only operations such as `approve_verification`.
#[test]
fn auth_project_owner_cannot_call_admin_only_approve_verification() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);
    let project_id = register_project(&client, &owner, "AuthOwnerApp970");

    client.mock_all_auths().set_min_project_age(&admin, &0);
    let evidence = String::from_str(&env, "QmEvidence970OwnAppTest123456789012345678901234");
    client
        .mock_all_auths()
        .request_verification(&project_id, &owner, &evidence);

    // Owner (not admin) tries to approve their own verification.
    let result = client
        .mock_all_auths()
        .try_approve_verification(&project_id, &owner);
    assert_eq!(
        result,
        Err(Ok(ContractError::AdminOnly)),
        "project owner must not be able to self-approve their own verification"
    );
}

// ── Admin cannot archive a project they do not own ────────────────────────────

/// An admin who is not the project owner must not be able to archive
/// the project through \`archive_project\`.
#[test]
fn auth_admin_cannot_archive_project_they_do_not_own() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);
    let project_id = register_project(&client, &owner, "AuthAdmArc970");

    let result = client
        .mock_all_auths()
        .try_archive_project(&project_id, &admin);
    assert_eq!(
        result,
        Err(Ok(ContractError::Unauthorized)),
        "admin must not be able to archive a project they do not own"
    );
}

// ── Stranger cannot request verification on another owner's project ───────────

/// A random address that is neither admin nor owner must be rejected when
/// calling \`request_verification\`.
#[test]
fn auth_stranger_cannot_request_verification_for_any_project() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let owner = Address::generate(&env);
    let stranger = Address::generate(&env);
    let project_id = register_project(&client, &owner, "AuthStrVerReq970");

    client.mock_all_auths().set_min_project_age(&admin, &0);
    let evidence = String::from_str(&env, "QmEvidence970StrangerTest12345678901234567890ab");

    let result = client
        .mock_all_auths()
        .try_request_verification(&project_id, &stranger, &evidence);
    assert_eq!(
        result,
        Err(Ok(ContractError::Unauthorized)),
        "a stranger must not be able to request verification for another owner's project"
    );
}
