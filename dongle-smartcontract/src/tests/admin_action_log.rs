use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::{AdminActionEntry, AdminActionType};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env, String, Vec,
};

fn setup(env: &Env) -> (crate::DongleContractClient<'_>, Address) {
    setup_contract(env)
}

#[test]
fn test_log_admin_added() {
    let env = Env::default();
    env.ledger().with_mut(|li| li.timestamp = 1000000);
    let (client, admin) = setup(&env);
    let new_admin = Address::generate(&env);

    client.mock_all_auths().add_admin(&admin, &new_admin);

    let count = client.get_admin_action_log_count();
    assert_eq!(count, 1);

    let entry = client.get_admin_action_log_entry(&1).unwrap();
    assert_eq!(entry.admin, admin);
    assert_eq!(entry.action_type, AdminActionType::AdminAdded);
    assert_eq!(entry.target_address, Some(new_admin));
    assert_eq!(entry.target_id, None);
    assert_eq!(entry.reason_cid, None);
    assert!(entry.timestamp > 0);
}

#[test]
fn test_log_admin_removed() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let admin2 = Address::generate(&env);

    client.mock_all_auths().add_admin(&admin, &admin2);
    client.mock_all_auths().remove_admin(&admin, &admin2);

    // Entry 1: admin added, Entry 2: admin removed
    let entry = client.get_admin_action_log_entry(&2).unwrap();
    assert_eq!(entry.admin, admin);
    assert_eq!(entry.action_type, AdminActionType::AdminRemoved);
    assert_eq!(entry.target_address, Some(admin2));
    assert_eq!(entry.target_id, None);
}

#[test]
fn test_log_fee_changed() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let treasury = Address::generate(&env);

    client
        .mock_all_auths()
        .set_fee(&admin, &None, &1000u128, &500u128, &treasury);

    let entry = client.get_admin_action_log_entry(&1).unwrap();
    assert_eq!(entry.admin, admin);
    assert_eq!(entry.action_type, AdminActionType::FeeChanged);
    assert_eq!(entry.target_id, None);
    assert_eq!(entry.target_address, None);
}

#[test]
fn test_log_min_project_age_set() {
    let env = Env::default();
    env.ledger().with_mut(|li| li.timestamp = 1000000);
    let (client, admin) = setup(&env);

    client
        .mock_all_auths()
        .set_min_project_age(&admin, &86400u64);

    let entry = client.get_admin_action_log_entry(&1).unwrap();
    assert_eq!(entry.admin, admin);
    assert_eq!(entry.action_type, AdminActionType::MinProjectAgeSet);
}

#[test]
fn test_log_verification_approve_reject() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| li.timestamp = 1000000);
    let (client, _admin) = setup(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "TestProj");

    let admin_clone = Address::generate(&env);
    client.add_admin(&_admin, &admin_clone);

    let evidence = String::from_str(
        &env,
        "QmTestEvidenceCid1234567890123456789012345678901234567890",
    );
    client.request_verification(&project_id, &owner, &evidence);

    client.approve_verification(&project_id, &admin_clone);

    // Entry 1 = admin_added, Entry 2 = verification_approved
    let entry = client.get_admin_action_log_entry(&2).unwrap();
    assert_eq!(entry.admin, admin_clone);
    assert_eq!(entry.action_type, AdminActionType::VerificationApproved);
    assert_eq!(entry.target_id, Some(project_id));
}

#[test]
fn test_log_verification_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| li.timestamp = 1000000);
    let (client, _admin) = setup(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "TestProj2");

    let admin_clone = Address::generate(&env);
    client.add_admin(&_admin, &admin_clone);

    let evidence = String::from_str(
        &env,
        "QmTestEvidenceCid1234567890123456789012345678901234567890",
    );
    client.request_verification(&project_id, &owner, &evidence);

    client.reject_verification(&project_id, &admin_clone);

    // Entry 1 = admin_added, Entry 2 = verification_rejected
    let entry = client.get_admin_action_log_entry(&2).unwrap();
    assert_eq!(entry.admin, admin_clone);
    assert_eq!(entry.action_type, AdminActionType::VerificationRejected);
    assert_eq!(entry.target_id, Some(project_id));
}

#[test]
fn test_list_admin_actions_pagination() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    // Create 5 actions
    for i in 0..5u64 {
        let new_admin = Address::generate(&env);
        client.mock_all_auths().add_admin(&admin, &new_admin);
    }

    let all = client.list_admin_actions(&0, &10);
    assert_eq!(all.len(), 5);

    let page1 = client.list_admin_actions(&0, &3);
    assert_eq!(page1.len(), 3);

    let page2 = client.list_admin_actions(&3, &3);
    assert_eq!(page2.len(), 2);

    // Most recent first: entries 5, 4, 3, 2, 1
    assert_eq!(page1.get(0).unwrap().id, 5);
    assert_eq!(page1.get(1).unwrap().id, 4);
    assert_eq!(page1.get(2).unwrap().id, 3);
    assert_eq!(page2.get(0).unwrap().id, 2);
    assert_eq!(page2.get(1).unwrap().id, 1);
}

#[test]
fn test_list_admin_actions_empty() {
    let env = Env::default();
    let (client, _admin) = setup(&env);

    let entries = client.list_admin_actions(&0, &10);
    assert_eq!(entries.len(), 0);
}

#[test]
fn test_get_admin_action_log_count() {
    let env = Env::default();
    let (client, admin) = setup(&env);

    assert_eq!(client.get_admin_action_log_count(), 0);

    let new_admin = Address::generate(&env);
    client.mock_all_auths().add_admin(&admin, &new_admin);

    assert_eq!(client.get_admin_action_log_count(), 1);
}

#[test]
fn test_get_log_entry_not_found() {
    let env = Env::default();
    let (client, _admin) = setup(&env);

    let entry = client.get_admin_action_log_entry(&999u64);
    assert_eq!(entry, None);
}

#[test]
fn test_log_verification_revoked() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().with_mut(|li| li.timestamp = 1000000);
    let (client, _admin) = setup(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "TestProj3");

    let admin_clone = Address::generate(&env);
    client.add_admin(&_admin, &admin_clone);

    let evidence = String::from_str(
        &env,
        "QmTestEvidenceCid1234567890123456789012345678901234567890",
    );
    client.request_verification(&project_id, &owner, &evidence);

    client.approve_verification(&project_id, &admin_clone);

    client.revoke_verification(
        &project_id,
        &admin_clone,
        &String::from_str(&env, "Policy violation"),
    );

    // Entry 1 = admin_added, Entry 2 = approved, Entry 3 = revoked
    let entry = client.get_admin_action_log_entry(&3).unwrap();
    assert_eq!(entry.admin, admin_clone);
    assert_eq!(entry.action_type, AdminActionType::VerificationRevoked);
    assert_eq!(entry.target_id, Some(project_id));
    assert_eq!(
        entry.reason_cid,
        Some(String::from_str(&env, "Policy violation"))
    );
}

#[test]
fn test_infallible_logging_does_not_block_admin_add() {
    let env = Env::default();
    let (client, admin) = setup(&env);
    let new_admin = Address::generate(&env);

    // Action should succeed
    let result = client.mock_all_auths().try_add_admin(&admin, &new_admin);
    assert!(result.is_ok());

    assert!(client.is_admin(&new_admin));
}
