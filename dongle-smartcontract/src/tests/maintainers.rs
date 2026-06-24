#![cfg(test)]

use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::ProjectUpdateParams;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

#[test]
fn test_owner_can_add_and_remove_maintainers() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "TestProject");

    let maintainer = Address::generate(&env);

    // Verify initially empty
    let list = client.get_maintainers(&project_id);
    assert_eq!(list.len(), 0);

    // Owner adds maintainer
    client
        .mock_all_auths()
        .add_maintainer(&project_id, &owner, &maintainer);

    // Verify added
    let list = client.get_maintainers(&project_id);
    assert_eq!(list.len(), 1);
    assert_eq!(list.get(0).unwrap(), maintainer);

    // Get project and verify maintainers field is populated
    let proj = client.get_project(&project_id).unwrap();
    assert_eq!(proj.maintainers.unwrap().len(), 1);

    // Owner removes maintainer
    client
        .mock_all_auths()
        .remove_maintainer(&project_id, &owner, &maintainer);

    // Verify removed
    let list = client.get_maintainers(&project_id);
    assert_eq!(list.len(), 0);
}

#[test]
fn test_maintainer_can_update_metadata() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "TestProject");

    let maintainer = Address::generate(&env);
    client
        .mock_all_auths()
        .add_maintainer(&project_id, &owner, &maintainer);

    // Maintainer updates metadata
    let new_desc = String::from_str(&env, "Updated by Maintainer");
    let update_params = ProjectUpdateParams {
        project_id,
        caller: maintainer.clone(),
        name: None,
        slug: None,
        description: Some(new_desc.clone()),
        category: None,
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
    };

    let updated_proj = client.mock_all_auths().update_project(&update_params);
    assert_eq!(updated_proj.description, new_desc);
}

#[test]
fn test_maintainer_cannot_manage_maintainers_or_transfer_ownership() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "TestProject");

    let maintainer = Address::generate(&env);
    client
        .mock_all_auths()
        .add_maintainer(&project_id, &owner, &maintainer);

    let other_user = Address::generate(&env);

    // Maintainer tries to add a maintainer -> fails
    let res = client
        .mock_all_auths()
        .try_add_maintainer(&project_id, &maintainer, &other_user);
    assert_eq!(res, Err(Ok(ContractError::Unauthorized)));

    // Maintainer tries to remove a maintainer -> fails
    let res = client
        .mock_all_auths()
        .try_remove_maintainer(&project_id, &maintainer, &maintainer);
    assert_eq!(res, Err(Ok(ContractError::Unauthorized)));

    // Maintainer tries to transfer ownership -> fails
    let res = client
        .mock_all_auths()
        .try_initiate_transfer(&project_id, &maintainer, &other_user);
    assert_eq!(res, Err(Ok(ContractError::Unauthorized)));
}

#[test]
fn test_unauthorized_user_cannot_do_anything() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "TestProject");

    let stranger = Address::generate(&env);
    let other_user = Address::generate(&env);

    // Stranger tries to update metadata -> fails
    let update_params = ProjectUpdateParams {
        project_id,
        caller: stranger.clone(),
        name: None,
        slug: None,
        description: Some(String::from_str(&env, "Hacked")),
        category: None,
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
    };
    let res = client.mock_all_auths().try_update_project(&update_params);
    assert_eq!(res, Err(Ok(ContractError::Unauthorized)));

    // Stranger tries to add maintainer -> fails
    let res = client
        .mock_all_auths()
        .try_add_maintainer(&project_id, &stranger, &other_user);
    assert_eq!(res, Err(Ok(ContractError::Unauthorized)));

    // Stranger tries to remove maintainer -> fails
    let res = client
        .mock_all_auths()
        .try_remove_maintainer(&project_id, &stranger, &other_user);
    assert_eq!(res, Err(Ok(ContractError::Unauthorized)));

    // Stranger tries to transfer ownership -> fails
    let res = client
        .mock_all_auths()
        .try_initiate_transfer(&project_id, &stranger, &other_user);
    assert_eq!(res, Err(Ok(ContractError::Unauthorized)));
}

#[test]
fn test_duplicate_maintainer_fails() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "TestProject");

    let maintainer = Address::generate(&env);
    client
        .mock_all_auths()
        .add_maintainer(&project_id, &owner, &maintainer);

    // Adding again fails
    let res = client
        .mock_all_auths()
        .try_add_maintainer(&project_id, &owner, &maintainer);
    assert_eq!(res, Err(Ok(ContractError::AlreadyLinked)));
}

#[test]
fn test_remove_missing_maintainer_fails() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "TestProject");

    let maintainer = Address::generate(&env);

    // Removing non-existent fails
    let res = client
        .mock_all_auths()
        .try_remove_maintainer(&project_id, &owner, &maintainer);
    assert_eq!(res, Err(Ok(ContractError::AdminNotFound)));
}

#[test]
fn test_owner_does_not_lose_ownership_privileges() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "TestProject");

    let maintainer = Address::generate(&env);
    client
        .mock_all_auths()
        .add_maintainer(&project_id, &owner, &maintainer);

    // Verify owner can still update project metadata
    let new_desc = String::from_str(&env, "Updated by Owner");
    let update_params = ProjectUpdateParams {
        project_id,
        caller: owner.clone(),
        name: None,
        slug: None,
        description: Some(new_desc.clone()),
        category: None,
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
    };
    let updated_proj = client.mock_all_auths().update_project(&update_params);
    assert_eq!(updated_proj.description, new_desc);

    // Verify owner can still transfer ownership
    let new_owner = Address::generate(&env);
    let res = client
        .mock_all_auths()
        .try_initiate_transfer(&project_id, &owner, &new_owner);
    assert!(res.is_ok());
}
