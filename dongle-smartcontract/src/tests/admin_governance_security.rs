//! Comprehensive test suite for Admin Action Auditing, Role-Based Access Control,
//! Sensitive Action Multi-Admin Approval Workflows, and Emergency Timelock Delay Bypass.

use crate::constants::TIMELOCK_MIN_DELAY;
use crate::errors::ContractError;
use crate::types::{
    AdminActionCategory, AdminPermission, AdminRole, ApprovalWorkflow, SensitiveActionPayload,
    SensitiveActionType, SensitiveProposalStatus,
};
use crate::DongleContract;
use crate::DongleContractClient;
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{Address, Env, String, Vec};

fn setup(env: &Env) -> (DongleContractClient<'_>, Address) {
    let contract_id = env.register(DongleContract, ());
    let client = DongleContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.mock_all_auths().initialize(&admin);
    (client, admin)
}

#[test]
fn test_admin_action_audit_and_filtering() {
    let env = Env::default();
    env.ledger().with_mut(|li| li.timestamp = 1000000); // 1,000,000 seconds
    let (client, admin) = setup(&env);

    // 1. Pause and unpause (Pause category)
    client.mock_all_auths().pause(&admin);
    client.mock_all_auths().unpause(&admin);

    // 2. Add an admin (Config category)
    let admin2 = Address::generate(&env);
    client.mock_all_auths().add_admin(&admin, &admin2);

    // Verify activity report
    let report = client.get_admin_activity_report(&admin, &0, &2000000);
    assert_eq!(report.admin, admin);
    assert_eq!(report.total_actions, 3);
    assert_eq!(report.pause_count, 2);
    assert_eq!(report.config_count, 1);
    assert_eq!(report.verification_count, 0);

    // Filter by Pause category
    let pause_report =
        client.get_admin_activity_report_filtered(&admin, &0, &2000000, &AdminActionCategory::Pause);
    assert_eq!(pause_report.actions.len(), 2);

    // Filter by Config category
    let config_report = client.get_admin_activity_report_filtered(
        &admin,
        &0,
        &2000000,
        &AdminActionCategory::Config,
    );
    assert_eq!(config_report.actions.len(), 1);

    // Filter by Verification category (should be empty)
    let verif_report = client.get_admin_activity_report_filtered(
        &admin,
        &0,
        &2000000,
        &AdminActionCategory::Verification,
    );
    assert_eq!(verif_report.actions.len(), 0);

    // Standard format exports
    let json_report = client.generate_admin_audit_report_json(&admin, &0, &2000000);
    assert!(json_report.len() > 0);

    let csv_report = client.generate_admin_audit_report_csv(&admin, &0, &2000000);
    assert!(csv_report.len() > 0);
}

#[test]
fn test_suspicious_activity_detection_bulk_and_unusual_hours() {
    let env = Env::default();
    // 01:00 UTC (3600 seconds into day) -> triggers unusual time detection
    env.ledger().with_mut(|li| li.timestamp = 3600);
    let (client, admin) = setup(&env);

    // Execute multiple actions rapidly in succession within 10 seconds (bulk operation)
    for _ in 0..5 {
        client.mock_all_auths().pause(&admin);
        client.mock_all_auths().unpause(&admin);
    }

    let report = client.get_admin_activity_report(&admin, &0, &10000);
    assert!(report.has_suspicious_activity);
    assert!(report.suspicious_reasons.len() >= 2);
}

#[test]
fn test_role_based_permissions_and_migration() {
    let env = Env::default();
    let (client, super_admin) = setup(&env);

    // Genesis admin is SuperAdmin
    let genesis_role = client.get_admin_role(&super_admin).unwrap();
    assert_eq!(genesis_role, AdminRole::SuperAdmin);
    assert!(client.has_admin_permission(&super_admin, &AdminPermission::EmergencyPause));
    assert!(client.has_admin_permission(&super_admin, &AdminPermission::ManageFees));
    assert!(client.has_admin_permission(&super_admin, &AdminPermission::ApproveVerification));

    // Add a new VerificationAdmin
    let verif_admin = Address::generate(&env);
    client
        .mock_all_auths()
        .add_admin_with_role(&super_admin, &verif_admin, &AdminRole::VerificationAdmin);

    let role = client.get_admin_role(&verif_admin).unwrap();
    assert_eq!(role, AdminRole::VerificationAdmin);
    assert!(client.has_admin_permission(&verif_admin, &AdminPermission::ApproveVerification));
    assert!(!client.has_admin_permission(&verif_admin, &AdminPermission::EmergencyPause));
    assert!(!client.has_admin_permission(&verif_admin, &AdminPermission::ManageFees));

    // VerificationAdmin attempting to pause contract fails with PermissionDenied
    let pause_res = client.mock_all_auths().try_pause(&verif_admin);
    assert_eq!(pause_res, Err(Ok(ContractError::PermissionDenied)));

    // Migration test: unassigned legacy admins default to SuperAdmin and are migrated
    let migrated_count = client.mock_all_auths().migrate_existing_admins(&super_admin);
    // Already has role so 0 new migrations
    assert_eq!(migrated_count, 0);
}

#[test]
fn test_sensitive_action_approval_workflow() {
    let env = Env::default();
    let (client, admin1) = setup(&env);

    let admin2 = Address::generate(&env);
    let admin3 = Address::generate(&env);
    client.mock_all_auths().add_admin(&admin1, &admin2);
    client.mock_all_auths().add_admin(&admin1, &admin3);

    // Configure 3-of-3 quorum workflow for EmergencyPause
    let workflow = ApprovalWorkflow {
        action_type: SensitiveActionType::EmergencyPause,
        required_quorum: 3,
        total_stages: 1,
        stage_delay: 0,
        is_active: true,
    };
    client
        .mock_all_auths()
        .set_approval_workflow(&admin1, &workflow);

    let retrieved_wf = client.get_approval_workflow(&SensitiveActionType::EmergencyPause);
    assert_eq!(retrieved_wf.required_quorum, 3);

    // Propose sensitive action to pause
    let prop_id = client.mock_all_auths().propose_sensitive_action(
        &admin1,
        &SensitiveActionType::EmergencyPause,
        &SensitiveActionPayload::EmergencyPause(true),
    );

    let proposal = client.get_sensitive_proposal(&prop_id).unwrap();
    assert_eq!(proposal.status, SensitiveProposalStatus::Pending);
    assert_eq!(proposal.approvals.len(), 1); // admin1 automatically approves

    // Admin 2 approves
    client
        .mock_all_auths()
        .approve_proposed_action(&admin2, &prop_id);
    let proposal_after_2 = client.get_sensitive_proposal(&prop_id).unwrap();
    assert_eq!(proposal_after_2.status, SensitiveProposalStatus::Pending);
    assert_eq!(proposal_after_2.approvals.len(), 2);

    // Duplicate vote fails
    let dup_res = client
        .mock_all_auths()
        .try_approve_proposed_action(&admin2, &prop_id);
    assert_eq!(dup_res, Err(Ok(ContractError::Unauthorized)));

    // Admin 3 approves -> quorum reached (3 of 3)
    client
        .mock_all_auths()
        .approve_proposed_action(&admin3, &prop_id);
    let proposal_after_3 = client.get_sensitive_proposal(&prop_id).unwrap();
    assert_eq!(proposal_after_3.status, SensitiveProposalStatus::Approved);

    // Execute the action
    assert!(!client.is_paused());
    client
        .mock_all_auths()
        .execute_proposed_action(&admin1, &prop_id);
    assert!(client.is_paused());

    let final_proposal = client.get_sensitive_proposal(&prop_id).unwrap();
    assert_eq!(final_proposal.status, SensitiveProposalStatus::Executed);
}

#[test]
fn test_sensitive_action_rejection() {
    let env = Env::default();
    let (client, admin1) = setup(&env);
    let admin2 = Address::generate(&env);
    client.mock_all_auths().add_admin(&admin1, &admin2);

    let workflow = ApprovalWorkflow {
        action_type: SensitiveActionType::LargeFeeAdjustment,
        required_quorum: 2,
        total_stages: 1,
        stage_delay: 0,
        is_active: true,
    };
    client
        .mock_all_auths()
        .set_approval_workflow(&admin1, &workflow);

    let treasury = Address::generate(&env);
    let prop_id = client.mock_all_auths().propose_sensitive_action(
        &admin1,
        &SensitiveActionType::LargeFeeAdjustment,
        &SensitiveActionPayload::LargeFeeAdjustment(None, 5000, 2000, treasury),
    );

    let reason = String::from_str(&env, "Fee increase too large for current market conditions");
    client
        .mock_all_auths()
        .reject_proposed_action(&admin2, &prop_id, &reason);

    let proposal = client.get_sensitive_proposal(&prop_id).unwrap();
    assert_eq!(proposal.status, SensitiveProposalStatus::Rejected);
}

#[test]
fn test_emergency_timelock_delay_bypass() {
    let env = Env::default();
    let (client, admin1) = setup(&env);
    let admin2 = Address::generate(&env);
    let admin3 = Address::generate(&env);
    client.mock_all_auths().add_admin(&admin1, &admin2);
    client.mock_all_auths().add_admin(&admin1, &admin3);

    // Configure bypass threshold: 2 of 3 admins
    client
        .mock_all_auths()
        .set_emergency_bypass_threshold(&admin1, &2);
    assert_eq!(client.get_emergency_bypass_threshold(), 2);

    // Schedule a fee change with 2 days delay
    let treasury = Address::generate(&env);
    let execution_time = env.ledger().timestamp() + TIMELOCK_MIN_DELAY + 86400;
    let action_id = client.mock_all_auths().schedule_set_fee(
        &admin1,
        &None,
        &2500u128,
        &1000u128,
        &treasury,
        &execution_time,
    );

    // Attempting to execute immediately fails because timelock delay has not expired
    let exec_fail = client.mock_all_auths().try_execute_set_fee(&admin1, &action_id);
    assert_eq!(exec_fail, Err(Ok(ContractError::TimelockNotExpired)));

    // Propose emergency bypass with justification CID
    let cid = String::from_str(&env, "QmEmergencySecurityExploitMitigationJustificationCID12345");
    let bypass_id = client
        .mock_all_auths()
        .propose_emergency_bypass(&admin1, &action_id, &cid);

    let bypass_prop = client.get_emergency_bypass(&bypass_id).unwrap();
    assert_eq!(bypass_prop.timelock_id, action_id);
    assert!(!bypass_prop.approved);
    assert_eq!(bypass_prop.approvals.len(), 1);

    // Admin 2 votes to approve -> threshold of 2 is met
    client
        .mock_all_auths()
        .approve_emergency_bypass(&admin2, &bypass_id);

    let bypass_prop_approved = client.get_emergency_bypass(&bypass_id).unwrap();
    assert!(bypass_prop_approved.approved);

    // Execute the emergency bypass
    assert!(!client.is_timelock_bypassed(&action_id));
    client
        .mock_all_auths()
        .execute_emergency_bypass(&admin1, &bypass_id);
    assert!(client.is_timelock_bypassed(&action_id));

    // Now execute_set_fee succeeds IMMEDIATELY without waiting for timelock delay!
    client.mock_all_auths().execute_set_fee(&admin1, &action_id);

    let fee_config = client.get_fee_config();
    assert_eq!(fee_config.verification_fee, 2500u128);

    // Verify audit trail contains the bypass proposal
    let bypasses = client.list_emergency_bypasses(&0, &10);
    assert_eq!(bypasses.len(), 1);
    assert_eq!(bypasses.get(0).unwrap().id, bypass_id);
    assert!(bypasses.get(0).unwrap().executed);
}
