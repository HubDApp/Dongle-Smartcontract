//! Tests for project archival feature (issue #121).

use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::{ProjectRegistrationParams, ProjectSortMode, VerificationStatus};
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};

fn register_tagged_project(
    client: &crate::DongleContractClient<'_>,
    env: &Env,
    owner: &Address,
    name: &str,
    slug: &str,
    tag: &str,
) -> u64 {
    let mut tags = Vec::new(env);
    tags.push_back(String::from_str(env, tag));
    client
        .mock_all_auths()
        .register_project(&ProjectRegistrationParams {
            owner: owner.clone(),
            name: String::from_str(env, name),
            slug: String::from_str(env, slug),
            description: String::from_str(env, "Tagged project description"),
            category: String::from_str(env, "DeFi"),
            website: None,
            license: None,
            logo_cid: None,
            metadata_cid: None,
            tags: Some(tags),
            social_links: None,
            launch_timestamp: None,
            bounty_url: None,
        })
}

#[test]
fn test_owner_can_archive_project() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "MyProject");
    client.archive_project(&project_id, &owner);

    let project = client.get_project(&project_id).unwrap();
    assert!(project.archived);
}

#[test]
fn test_unauthorized_cannot_archive_project() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let stranger = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "MyProject");

    let result = client.try_archive_project(&project_id, &stranger);
    assert_eq!(result, Err(Ok(ContractError::Unauthorized)));
}

#[test]
fn test_admin_can_force_archive_project() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "MyProject");
    client.archive_project(&project_id, &admin);

    let project = client.get_project(&project_id).unwrap();
    assert!(project.archived);
}

#[test]
fn test_archived_project_excluded_from_list_projects() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let id1 = create_test_project(&client, &owner, "ProjectA");
    let id2 = create_test_project(&client, &owner, "ProjectB");

    client.archive_project(&id1, &owner);

    let projects = client.list_projects(&0, &10);
    assert_eq!(projects.len(), 1);
    assert_eq!(projects.get(0).unwrap().id, id2);
}

#[test]
fn test_archiving_and_reactivating_updates_owner_project_index() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "IndexedProject");
    assert_eq!(client.get_projects_by_owner(&owner).len(), 1);

    client.archive_project(&project_id, &owner);
    assert_eq!(client.get_projects_by_owner(&owner).len(), 0);

    client.reactivate_project(&project_id, &owner);
    let projects = client.get_projects_by_owner(&owner);
    assert_eq!(projects.len(), 1);
    assert_eq!(projects.get(0).unwrap().id, project_id);
}

#[test]
fn test_archived_project_stays_out_of_active_index_after_transfer() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let old_owner = Address::generate(&env);
    let new_owner = Address::generate(&env);

    let project_id = create_test_project(&client, &old_owner, "ArchivedTransfer");
    client.archive_project(&project_id, &old_owner);
    client.initiate_transfer(&project_id, &old_owner, &new_owner);
    client.accept_transfer(&project_id, &new_owner);

    assert_eq!(client.get_projects_by_owner(&old_owner).len(), 0);
    assert_eq!(client.get_projects_by_owner(&new_owner).len(), 0);
}

#[test]
fn test_archived_project_excluded_from_list_by_category() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let id1 = create_test_project(&client, &owner, "ProjectA");
    let id2 = create_test_project(&client, &owner, "ProjectB");

    client.archive_project(&id1, &owner);

    let category = soroban_sdk::String::from_str(&env, "DeFi");
    let projects = client.list_projects_by_category(&category, &0, &10);
    assert_eq!(projects.len(), 1);
    assert_eq!(projects.get(0).unwrap().id, id2);
}

#[test]
fn test_archive_nonexistent_project_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let caller = Address::generate(&env);

    let result = client.try_archive_project(&999, &caller);
    assert_eq!(result, Err(Ok(ContractError::ProjectNotFound)));
}

/// Issue #172: reactivation must make a project eligible for discovery again.
/// `test_archiving_and_reactivating_updates_owner_project_index` above already
/// covers `get_projects_by_owner`; these cover the remaining listing APIs.
#[test]
fn test_reactivated_project_reappears_in_list_projects() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let id1 = create_test_project(&client, &owner, "ProjectA");
    let id2 = create_test_project(&client, &owner, "ProjectB");

    client.archive_project(&id1, &owner);
    assert_eq!(client.list_projects(&0, &10).len(), 1);

    client.reactivate_project(&id1, &owner);
    let projects = client.list_projects(&0, &10);
    assert_eq!(projects.len(), 2);
    let mut ids = Vec::new(&env);
    for p in projects.iter() {
        ids.push_back(p.id);
    }
    assert!(ids.contains(id1));
    assert!(ids.contains(id2));
}

#[test]
fn test_reactivated_project_reappears_in_list_projects_by_status() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "VerifiedProject");
    client.request_verification(
        &project_id,
        &owner,
        &String::from_str(&env, "QmYwAPJzv5CZsnAzt8auVZRnG8X1sC3yRyvCb4s46HoPa1"),
    );
    client.approve_verification(&project_id, &admin);

    client.archive_project(&project_id, &owner);
    assert_eq!(
        client
            .list_projects_by_status(&VerificationStatus::Verified, &0, &10)
            .len(),
        0
    );

    client.reactivate_project(&project_id, &owner);
    let projects = client.list_projects_by_status(&VerificationStatus::Verified, &0, &10);
    assert_eq!(projects.len(), 1);
    assert_eq!(projects.get(0).unwrap().id, project_id);
}

#[test]
fn test_reactivated_project_reappears_in_list_projects_by_category() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "CategoryProject");
    client.archive_project(&project_id, &owner);

    let category = String::from_str(&env, "DeFi");
    assert_eq!(
        client.list_projects_by_category(&category, &0, &10).len(),
        0
    );

    client.reactivate_project(&project_id, &owner);
    let projects = client.list_projects_by_category(&category, &0, &10);
    assert_eq!(projects.len(), 1);
    assert_eq!(projects.get(0).unwrap().id, project_id);
}

#[test]
fn test_reactivated_project_reappears_in_list_projects_by_tag() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let project_id = register_tagged_project(
        &client,
        &env,
        &owner,
        "TaggedProject",
        "tagged-project",
        "defi",
    );

    client.mock_all_auths().archive_project(&project_id, &owner);
    let tag = String::from_str(&env, "defi");
    assert_eq!(client.list_projects_by_tag(&tag, &0, &10).len(), 0);

    client
        .mock_all_auths()
        .reactivate_project(&project_id, &owner);
    let projects = client.list_projects_by_tag(&tag, &0, &10);
    assert_eq!(projects.len(), 1);
    assert_eq!(projects.get(0).unwrap().id, project_id);
}

#[test]
fn test_reactivated_project_reappears_in_list_projects_sorted() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "SortedProject");
    client.archive_project(&project_id, &owner);
    assert_eq!(
        client
            .list_projects_sorted(&ProjectSortMode::Newest, &0, &10)
            .len(),
        0
    );

    client.reactivate_project(&project_id, &owner);
    let projects = client.list_projects_sorted(&ProjectSortMode::Newest, &0, &10);
    assert_eq!(projects.len(), 1);
    assert_eq!(projects.get(0).unwrap().id, project_id);
}
