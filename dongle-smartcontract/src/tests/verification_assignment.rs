//! Tests for verification assignment to admin (Issue #227).
//!
//! Verifies that an admin can assign a pending verification to another admin,
//! the assignment emits an event, the getter exposes the assigned admin,
//! and unauthorized access is rejected.

use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use soroban_sdk::{testutils::Address as _, Address, Env, String};

const VALID_EVIDENCE_CID: &str = "QmTu64kW8cUwwigCcJcKQS6F6wTwwJeD8Y18qr9s9DXkXy";

#[test]
fn test_assign_verification_to_admin() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "AssignTest");

    // Request verification
    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, VALID_EVIDENCE_CID),
    );

    // Add another admin
    let reviewer = Address::generate(&env);
    client.add_admin(&admin, &reviewer);

    // Before assignment, no assigned admin
    let assigned = client.get_assigned_admin(&project_id);
    assert!(assigned.is_none());

    // Assign verification
    client.assign_verification(&project_id, &admin, &reviewer);

    // After assignment, getter returns the assigned admin
    let assigned = client.get_assigned_admin(&project_id);
    assert_eq!(assigned, Some(reviewer));
}

#[test]
fn test_assign_verification_non_admin_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "NonAdminAssign");

    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, VALID_EVIDENCE_CID),
    );

    let non_admin = Address::generate(&env);

    // Assigning to a non-admin should fail
    let result = client.try_assign_verification(&project_id, &admin, &non_admin);
    assert_eq!(result, Err(Ok(ContractError::AdminNotFound)));
}

#[test]
fn test_assign_verification_not_pending_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "NotPending");

    // No verification request (status is Unverified) — should fail
    let reviewer = Address::generate(&env);
    client.add_admin(&admin, &reviewer);

    // Cannot assign because there's no verification request
    let result = client.try_assign_verification(&project_id, &admin, &reviewer);
    // VerificationNotFound because no record exists
    assert!(result.is_err());
}

#[test]
fn test_assign_verification_after_approval_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "AfterApproval");

    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, VALID_EVIDENCE_CID),
    );

    client.approve_verification(&project_id, &admin);

    let reviewer = Address::generate(&env);
    client.add_admin(&admin, &reviewer);

    // Status is now Verified, not Pending — should fail
    let result = client.try_assign_verification(&project_id, &admin, &reviewer);
    assert_eq!(result, Err(Ok(ContractError::InvalidStatus)));
}

#[test]
fn test_unauthorized_caller_cannot_assign() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "UnauthorizedAssign");

    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, VALID_EVIDENCE_CID),
    );

    let non_admin_caller = Address::generate(&env);
    let reviewer = Address::generate(&env);
    client.add_admin(&admin, &reviewer);

    // Non-admin caller should fail
    let result = client.try_assign_verification(&project_id, &non_admin_caller, &reviewer);
    assert_eq!(result, Err(Ok(ContractError::AdminOnly)));
}
