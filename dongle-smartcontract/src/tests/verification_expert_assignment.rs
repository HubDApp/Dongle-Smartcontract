//! Tests for verification request assignment to specialized admins,
//! expertise routing, accept/decline workflows, assignment history, and SLA escalation.

use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::VerificationAssignmentStatus;
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    Address, Env, String, Vec,
};

const VALID_EVIDENCE_CID: &str = "QmTu64kW8cUwwigCcJcKQS6F6wTwwJeD8Y18qr9s9DXkXy";

#[test]
fn test_set_and_query_admin_expertise() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);

    let specialist = Address::generate(&env);
    client.add_admin(&admin, &specialist);

    let mut expertise = Vec::new(&env);
    expertise.push_back(String::from_str(&env, "defi"));
    expertise.push_back(String::from_str(&env, "security"));

    // Set expertise
    client.set_admin_expertise(&admin, &specialist, &expertise);

    // Query expertise for the specialist
    let queried = client.get_admin_expertise(&specialist);
    assert_eq!(queried.len(), 2);
    assert_eq!(queried.get(0).unwrap(), String::from_str(&env, "defi"));
    assert_eq!(queried.get(1).unwrap(), String::from_str(&env, "security"));

    // Query admins by expertise
    let defi_admins = client.get_admins_by_expertise(&String::from_str(&env, "defi"));
    assert_eq!(defi_admins.len(), 1);
    assert_eq!(defi_admins.get(0).unwrap(), specialist);

    let gaming_admins = client.get_admins_by_expertise(&String::from_str(&env, "gaming"));
    assert_eq!(gaming_admins.len(), 0);
}

#[test]
fn test_assign_request_with_expertise() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "ExpertiseAssignProj");

    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, VALID_EVIDENCE_CID),
    );

    let defi_specialist = Address::generate(&env);
    client.add_admin(&admin, &defi_specialist);

    let mut expertise = Vec::new(&env);
    expertise.push_back(String::from_str(&env, "defi"));
    client.set_admin_expertise(&admin, &defi_specialist, &expertise);

    // Assigning requiring "security" should fail because defi_specialist lacks it
    let res = client.try_assign_verification_expertise(
        &project_id,
        &admin,
        &defi_specialist,
        &String::from_str(&env, "security"),
    );
    assert_eq!(res, Err(Ok(ContractError::AdminLacksExpertise)));

    // Assigning requiring "defi" succeeds
    let assignment_id = client.assign_verification_expertise(
        &project_id,
        &admin,
        &defi_specialist,
        &String::from_str(&env, "defi"),
    );
    assert!(assignment_id > 0);

    // Active assignment is tracked
    let current = client.get_current_assignment(&project_id).unwrap();
    assert_eq!(current.assignee, defi_specialist);
    assert_eq!(current.status, VerificationAssignmentStatus::Assigned);
    assert_eq!(current.expertise, Some(String::from_str(&env, "defi")));
    assert_eq!(
        client.get_assigned_admin(&project_id),
        Some(defi_specialist)
    );
}

#[test]
fn test_route_verification_to_expert() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "RouteProj");

    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, VALID_EVIDENCE_CID),
    );

    let expert1 = Address::generate(&env);
    let expert2 = Address::generate(&env);
    client.add_admin(&admin, &expert1);
    client.add_admin(&admin, &expert2);

    let mut exp = Vec::new(&env);
    exp.push_back(String::from_str(&env, "nft"));
    client.set_admin_expertise(&admin, &expert1, &exp);
    client.set_admin_expertise(&admin, &expert2, &exp);

    // Route automatically
    let selected =
        client.route_verification_to_expert(&project_id, &admin, &String::from_str(&env, "nft"));
    assert!(selected == expert1 || selected == expert2);

    let current = client.get_current_assignment(&project_id).unwrap();
    assert_eq!(current.assignee, selected);
}

#[test]
fn test_admin_accept_assignment() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "AcceptProj");

    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, VALID_EVIDENCE_CID),
    );

    let reviewer = Address::generate(&env);
    client.add_admin(&admin, &reviewer);

    let mut exp = Vec::new(&env);
    exp.push_back(String::from_str(&env, "security"));
    client.set_admin_expertise(&admin, &reviewer, &exp);

    client.assign_verification_expertise(
        &project_id,
        &admin,
        &reviewer,
        &String::from_str(&env, "security"),
    );

    // Another admin cannot accept the assignment
    let other_admin = Address::generate(&env);
    client.add_admin(&admin, &other_admin);
    let wrong_accept = client.try_accept_verification_assignment(&project_id, &other_admin);
    assert_eq!(wrong_accept, Err(Ok(ContractError::NotAssignedAdmin)));

    // Assigned admin accepts
    client.accept_verification_assignment(&project_id, &reviewer);

    let current = client.get_current_assignment(&project_id).unwrap();
    assert_eq!(current.status, VerificationAssignmentStatus::Accepted);
    assert!(current.responded_at.is_some());
}

#[test]
fn test_admin_decline_assignment_and_reassign() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "DeclineProj");

    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, VALID_EVIDENCE_CID),
    );

    let reviewer1 = Address::generate(&env);
    let reviewer2 = Address::generate(&env);
    client.add_admin(&admin, &reviewer1);
    client.add_admin(&admin, &reviewer2);

    let mut exp = Vec::new(&env);
    exp.push_back(String::from_str(&env, "audit"));
    client.set_admin_expertise(&admin, &reviewer1, &exp);
    client.set_admin_expertise(&admin, &reviewer2, &exp);

    client.assign_verification_expertise(
        &project_id,
        &admin,
        &reviewer1,
        &String::from_str(&env, "audit"),
    );

    // Reviewer 1 declines
    client.decline_verification_assignment(
        &project_id,
        &reviewer1,
        &String::from_str(&env, "Capacity full"),
    );

    // Verification is now unassigned
    assert_eq!(client.get_assigned_admin(&project_id), None);

    // History tracks the declined assignment
    let history = client.get_assignment_history(&project_id);
    assert_eq!(history.len(), 1);
    assert_eq!(
        history.get(0).unwrap().status,
        VerificationAssignmentStatus::Declined
    );
    assert_eq!(
        history.get(0).unwrap().decline_reason,
        Some(String::from_str(&env, "Capacity full"))
    );

    // Now reassign to reviewer 2
    client.assign_verification_expertise(
        &project_id,
        &admin,
        &reviewer2,
        &String::from_str(&env, "audit"),
    );

    assert_eq!(
        client.get_assigned_admin(&project_id),
        Some(reviewer2.clone())
    );

    // Reviewer 2 accepts
    client.accept_verification_assignment(&project_id, &reviewer2);

    // History now contains both entries
    let updated_history = client.get_assignment_history(&project_id);
    assert_eq!(updated_history.len(), 2);
    assert_eq!(updated_history.get(0).unwrap().assignee, reviewer1);
    assert_eq!(updated_history.get(1).unwrap().assignee, reviewer2);
    assert_eq!(
        updated_history.get(1).unwrap().status,
        VerificationAssignmentStatus::Accepted
    );
}

#[test]
fn test_sla_escalation_lifecycle() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "SlaProj");

    // Configure SLA to 3600 seconds (1 hour)
    client.set_verification_sla(&admin, &3600);
    assert_eq!(client.get_verification_sla(), 3600);

    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, VALID_EVIDENCE_CID),
    );

    let reviewer = Address::generate(&env);
    client.add_admin(&admin, &reviewer);

    let mut exp = Vec::new(&env);
    exp.push_back(String::from_str(&env, "security"));
    client.set_admin_expertise(&admin, &reviewer, &exp);

    client.assign_verification_expertise(
        &project_id,
        &admin,
        &reviewer,
        &String::from_str(&env, "security"),
    );

    // SLA is not breached yet
    assert_eq!(client.is_assignment_sla_breached(&project_id), false);
    let remaining = client.get_assignment_sla_remaining(&project_id);
    assert!(remaining.is_some());

    // Escalating before SLA deadline should fail
    let premature_esc = client.try_escalate_verification_assignment(
        &admin,
        &project_id,
        &String::from_str(&env, "Too slow"),
    );
    assert_eq!(premature_esc, Err(Ok(ContractError::SlaNotBreached)));

    // Fast-forward ledger time past SLA (3601 seconds later)
    env.ledger().set_timestamp(env.ledger().timestamp() + 3601);

    // SLA is now breached
    assert_eq!(client.is_assignment_sla_breached(&project_id), true);
    assert_eq!(client.get_assignment_sla_remaining(&project_id), Some(0));

    // Admin escalates
    client.escalate_verification_assignment(
        &admin,
        &project_id,
        &String::from_str(&env, "Overdue SLA breach"),
    );

    // Request is unassigned to allow supervisor or emergency re-routing
    assert_eq!(client.get_assigned_admin(&project_id), None);

    let history = client.get_assignment_history(&project_id);
    assert_eq!(history.len(), 1);
    let entry = history.get(0).unwrap();
    assert_eq!(entry.status, VerificationAssignmentStatus::Escalated);
    assert_eq!(
        entry.escalation_reason,
        Some(String::from_str(&env, "Overdue SLA breach"))
    );
}
