//! Tests for collection visibility, privacy, and share links (#819).

#![cfg(test)]

use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use soroban_sdk::{testutils::Address as _, Address, Env, String};

#[test]
fn test_create_collection_defaults_to_public() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let id = client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "DeFi Gems"),
        &String::from_str(&env, "Top DeFi protocols"),
    );

    let collection = client
        .get_collection(&id)
        .expect("public collection should be found");
    assert_eq!(collection.id, id);
    assert_eq!(collection.owner, admin);
    assert_eq!(collection.is_public, true);
    assert_eq!(collection.name, String::from_str(&env, "DeFi Gems"));
}

#[test]
fn test_create_collection_with_explicit_visibility() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let id = client.mock_all_auths().create_collection_vis(
        &admin,
        &String::from_str(&env, "Secret Projects"),
        &String::from_str(&env, "Unreleased protocols"),
        &false,
    );

    // Public getter returns None for private collections
    assert!(client.get_collection(&id).is_none());

    // Owner can access via get_collection_for_caller
    let col = client
        .mock_all_auths()
        .get_collection_for_caller(&admin, &id);
    assert_eq!(col.id, id);
    assert_eq!(col.is_public, false);
}

#[test]
fn test_create_user_collection() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let user = Address::generate(&env);

    let id = client.mock_all_auths().create_user_collection(
        &user,
        &String::from_str(&env, "My Private Stash"),
        &String::from_str(&env, "Only for me"),
        &false,
    );

    // Public getter returns None
    assert!(client.get_collection(&id).is_none());

    // User can access their own collection
    let col = client
        .mock_all_auths()
        .get_collection_for_caller(&user, &id);
    assert_eq!(col.owner, user);
    assert_eq!(col.is_public, false);
}

#[test]
fn test_toggle_collection_visibility() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let id = client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "Public Collection"),
        &String::from_str(&env, "description"),
    );

    assert!(client.get_collection(&id).is_some());

    // Toggle to private
    let new_vis = client
        .mock_all_auths()
        .toggle_collection_visibility(&admin, &id);
    assert_eq!(new_vis, false);
    assert!(client.get_collection(&id).is_none());

    // Toggle back to public
    let new_vis2 = client
        .mock_all_auths()
        .toggle_collection_visibility(&admin, &id);
    assert_eq!(new_vis2, true);
    assert!(client.get_collection(&id).is_some());
}

#[test]
fn test_set_collection_visibility() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let id = client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "Public Collection"),
        &String::from_str(&env, "description"),
    );

    client
        .mock_all_auths()
        .set_collection_visibility(&admin, &id, &false);
    assert!(client.get_collection(&id).is_none());

    client
        .mock_all_auths()
        .set_collection_visibility(&admin, &id, &true);
    assert!(client.get_collection(&id).is_some());
}

#[test]
fn test_non_owner_cannot_toggle_visibility() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let stranger = Address::generate(&env);

    let id = client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "Admin Collection"),
        &String::from_str(&env, "description"),
    );

    let result = client
        .mock_all_auths()
        .try_toggle_collection_visibility(&stranger, &id);
    assert_eq!(result, Err(Ok(ContractError::NotCollectionOwner)));
}

#[test]
fn test_public_collections_discoverable_and_searchable() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let id1 = client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "Public One"),
        &String::from_str(&env, "desc 1"),
    );

    let id2 = client.mock_all_auths().create_collection_vis(
        &admin,
        &String::from_str(&env, "Private Two"),
        &String::from_str(&env, "desc 2"),
        &false,
    );

    let id3 = client.mock_all_auths().create_collection(
        &admin,
        &String::from_str(&env, "Public Three"),
        &String::from_str(&env, "desc 3"),
    );

    let public_cols = client.list_collections(&0, &10);
    assert_eq!(public_cols.len(), 2);
    assert_eq!(public_cols.get(0).unwrap().id, id1);
    assert_eq!(public_cols.get(1).unwrap().id, id3);

    // Private collection is excluded from list_collections
    assert!(!public_cols.iter().any(|c| c.id == id2));
}

#[test]
fn test_private_collections_only_for_owner() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let user = Address::generate(&env);
    let stranger = Address::generate(&env);

    let pub_id = client.mock_all_auths().create_user_collection(
        &user,
        &String::from_str(&env, "User Public"),
        &String::from_str(&env, "desc"),
        &true,
    );

    let priv_id = client.mock_all_auths().create_user_collection(
        &user,
        &String::from_str(&env, "User Private"),
        &String::from_str(&env, "desc"),
        &false,
    );

    // Stranger cannot get private collection
    let res = client
        .mock_all_auths()
        .try_get_collection_for_caller(&stranger, &priv_id);
    assert_eq!(res, Err(Ok(ContractError::CollectionPrivate)));

    // Stranger can get public collection
    let pub_col = client
        .mock_all_auths()
        .get_collection_for_caller(&stranger, &pub_id);
    assert_eq!(pub_col.id, pub_id);

    // Owner can list all their own collections (both public and private)
    let user_cols = client
        .mock_all_auths()
        .list_user_collections(&user, &0, &10);
    assert_eq!(user_cols.len(), 2);

    // Admin can also view private collection
    let admin_view = client
        .mock_all_auths()
        .get_collection_for_caller(&admin, &priv_id);
    assert_eq!(admin_view.id, priv_id);
}

#[test]
fn test_share_link_generation_and_access() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let user = Address::generate(&env);

    let priv_id = client.mock_all_auths().create_user_collection(
        &user,
        &String::from_str(&env, "Confidential Portfolio"),
        &String::from_str(&env, "Private shared via link"),
        &false,
    );

    // Generate share link
    let share_link = client
        .mock_all_auths()
        .generate_collection_share_link(&user, &priv_id);

    // Access via share link URL
    let col = client
        .get_collection_by_share_token(&priv_id, &share_link);
    assert_eq!(col.id, priv_id);
    assert_eq!(col.name, String::from_str(&env, "Confidential Portfolio"));

    // Revoke share link
    client
        .mock_all_auths()
        .revoke_collection_share_link(&user, &priv_id);

    // Subsequent access fails
    let revoked_res = client
        .try_get_collection_by_share_token(&priv_id, &share_link);
    assert_eq!(revoked_res, Err(Ok(ContractError::ShareTokenNotFound)));
}

#[test]
fn test_invalid_share_token_rejected() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let user = Address::generate(&env);

    let priv_id = client.mock_all_auths().create_user_collection(
        &user,
        &String::from_str(&env, "Confidential"),
        &String::from_str(&env, "Private"),
        &false,
    );

    let _share_link = client
        .mock_all_auths()
        .generate_collection_share_link(&user, &priv_id);

    let wrong_token = String::from_str(&env, "wrong_token_123");
    let result = client.try_get_collection_by_share_token(&priv_id, &wrong_token);
    assert_eq!(result, Err(Ok(ContractError::InvalidShareToken)));
}

#[test]
fn test_non_owner_cannot_generate_or_revoke_share_link() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let user = Address::generate(&env);
    let stranger = Address::generate(&env);

    let priv_id = client.mock_all_auths().create_user_collection(
        &user,
        &String::from_str(&env, "Confidential"),
        &String::from_str(&env, "Private"),
        &false,
    );

    let gen_res = client
        .mock_all_auths()
        .try_generate_collection_share_link(&stranger, &priv_id);
    assert_eq!(gen_res, Err(Ok(ContractError::NotCollectionOwner)));

    let rev_res = client
        .mock_all_auths()
        .try_revoke_collection_share_link(&stranger, &priv_id);
    assert_eq!(rev_res, Err(Ok(ContractError::NotCollectionOwner)));
}

#[test]
fn test_list_collection_projects_visibility() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let stranger = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "Alpha");

    let priv_id = client.mock_all_auths().create_collection_vis(
        &admin,
        &String::from_str(&env, "Private Collection"),
        &String::from_str(&env, "Private"),
        &false,
    );

    client
        .mock_all_auths()
        .add_project_to_collection(&admin, &priv_id, &project_id);

    // Public list_collection_projects returns empty for private collection
    let public_projects = client.list_collection_projects(&priv_id, &0, &10);
    assert_eq!(public_projects.len(), 0);

    // Owner/Admin can list projects with auth
    let admin_projects = client
        .mock_all_auths()
        .list_col_projects_for_caller(&admin, &priv_id, &0, &10);
    assert_eq!(admin_projects.len(), 1);
    assert_eq!(admin_projects.get(0).unwrap(), project_id);

    // Stranger cannot list projects
    let stranger_projects = client
        .mock_all_auths()
        .try_list_col_projects_for_caller(&stranger, &priv_id, &0, &10);
    assert_eq!(stranger_projects, Err(Ok(ContractError::CollectionPrivate)));
}
