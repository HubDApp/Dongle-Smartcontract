//! Tests for the changelog admin-approval workflow (#930).

use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::{ChangelogSortMode, ChangelogStatus};
use crate::DongleContractClient;
use soroban_sdk::{testutils::Address as _, vec, Address, Env, String};

const CID_A: &str = "bafybeiboz75hbx2qg7g4j4vqaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const CID_B: &str = "bafybeiboz75hbx2qg7g4j4vqbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const CID_C: &str = "bafybeiboz75hbx2qg7g4j4vqccccccccccccccccccccccccccccccc";

fn submit(env: &Env, client: &DongleContractClient<'_>, project_id: u64, owner: &Address, cid: &str) -> u64 {
    client.submit_changelog_for_approval(
        &project_id,
        owner,
        &String::from_str(env, cid),
        &None,
        &None,
        &None,
    )
}

#[test]
fn test_submitted_entry_is_draft_and_hidden() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Alpha");

    let id = submit(&env, &client, project_id, &owner, CID_A);

    let entry = client.get_changelog_entry(&id).unwrap();
    assert_eq!(entry.status, ChangelogStatus::Draft);
    assert_eq!(entry.published_at, None);
    assert_eq!(client.get_changelog_count(&project_id), 0);
    assert_eq!(
        client.get_project_changelog(&project_id, &0, &10, &ChangelogSortMode::Newest).len(),
        0
    );
    assert_eq!(client.get_pending_changelog_entries(&0, &10).len(), 1);
}

#[test]
fn test_non_owner_cannot_submit() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let stranger = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Alpha");

    let result = client.try_submit_changelog_for_approval(
        &project_id,
        &stranger,
        &String::from_str(&env, CID_A),
        &None,
        &None,
        &None,
    );
    assert_eq!(result, Err(Ok(ContractError::Unauthorized)));
}

#[test]
fn test_batch_approve_then_owner_publishes() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Alpha");

    let a = submit(&env, &client, project_id, &owner, CID_A);
    let b = submit(&env, &client, project_id, &owner, CID_B);

    let result = client.approve_changelog_entries(&admin, &vec![&env, a, b, 999], &false);
    assert_eq!(result.succeeded, vec![&env, a, b]);
    assert_eq!(result.failed, vec![&env, 999]);

    let entry = client.get_changelog_entry(&a).unwrap();
    assert_eq!(entry.status, ChangelogStatus::Approved);
    assert_eq!(entry.approved_by, Some(admin.clone()));
    assert_eq!(client.get_changelog_count(&project_id), 0);
    assert_eq!(client.get_pending_changelog_entries(&0, &10).len(), 0);

    let published = client.publish_changelog_entries(&owner, &vec![&env, a, b]);
    assert_eq!(published.succeeded.len(), 2);
    assert_eq!(client.get_changelog_count(&project_id), 2);
    assert_eq!(
        client.get_changelog_entry(&b).unwrap().status,
        ChangelogStatus::Published
    );
}

#[test]
fn test_approve_and_publish_in_one_step() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Alpha");

    let id = submit(&env, &client, project_id, &owner, CID_A);
    client.approve_changelog_entries(&admin, &vec![&env, id], &true);

    let entry = client.get_changelog_entry(&id).unwrap();
    assert_eq!(entry.status, ChangelogStatus::Published);
    assert!(entry.published_at.is_some());
    assert_eq!(client.get_changelog_count(&project_id), 1);
}

#[test]
fn test_drafts_cannot_be_published_and_approved_cannot_be_reapproved() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Alpha");

    let id = submit(&env, &client, project_id, &owner, CID_A);
    let result = client.publish_changelog_entries(&owner, &vec![&env, id]);
    assert_eq!(result.failed, vec![&env, id]);

    client.approve_changelog_entries(&admin, &vec![&env, id], &false);
    let again = client.approve_changelog_entries(&admin, &vec![&env, id], &false);
    assert_eq!(again.failed, vec![&env, id]);
}

#[test]
fn test_other_user_cannot_publish_approved_entry() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let stranger = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Alpha");

    let id = submit(&env, &client, project_id, &owner, CID_A);
    client.approve_changelog_entries(&admin, &vec![&env, id], &false);

    let result = client.publish_changelog_entries(&stranger, &vec![&env, id]);
    assert_eq!(result.failed, vec![&env, id]);
    // Admins may publish on the owner's behalf.
    let result = client.publish_changelog_entries(&admin, &vec![&env, id]);
    assert_eq!(result.succeeded, vec![&env, id]);
}

#[test]
fn test_batch_reject_deletes_drafts() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Alpha");

    let a = submit(&env, &client, project_id, &owner, CID_A);
    let b = submit(&env, &client, project_id, &owner, CID_B);
    let c = submit(&env, &client, project_id, &owner, CID_C);

    let result = client.reject_changelog_entries(&admin, &vec![&env, a, c]);
    assert_eq!(result.succeeded, vec![&env, a, c]);
    assert!(client.get_changelog_entry(&a).is_none());
    assert!(client.get_changelog_entry(&c).is_none());

    let pending = client.get_pending_changelog_entries(&0, &10);
    assert_eq!(pending.len(), 1);
    assert_eq!(pending.get(0).unwrap().id, b);
}

#[test]
fn test_batch_endpoints_are_admin_only_and_bounded() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Alpha");
    let id = submit(&env, &client, project_id, &owner, CID_A);

    assert_eq!(
        client.try_approve_changelog_entries(&owner, &vec![&env, id], &true),
        Err(Ok(ContractError::AdminOnly))
    );
    assert_eq!(
        client.try_reject_changelog_entries(&owner, &vec![&env, id]),
        Err(Ok(ContractError::AdminOnly))
    );
    assert_eq!(
        client.try_approve_changelog_entries(&admin, &soroban_sdk::Vec::new(&env), &true),
        Err(Ok(ContractError::InvalidChangelogBatch))
    );
}

#[test]
fn test_approval_required_closes_direct_path() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "Alpha");
    let cid = String::from_str(&env, CID_A);

    // Default: direct path publishes immediately.
    let id = client.add_changelog_entry(&project_id, &owner, &cid, &None, &None, &None);
    assert_eq!(
        client.get_changelog_entry(&id).unwrap().status,
        ChangelogStatus::Published
    );

    client.set_changelog_approval_required(&admin, &true);
    assert!(client.is_changelog_approval_required());
    let result = client.try_add_changelog_entry(
        &project_id,
        &owner,
        &String::from_str(&env, CID_B),
        &None,
        &None,
        &None,
    );
    assert_eq!(result, Err(Ok(ContractError::ChangelogApprovalRequired)));
}
