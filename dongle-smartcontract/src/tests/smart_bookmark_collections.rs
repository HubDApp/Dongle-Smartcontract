//! Tests for Smart Bookmark Collections (#914).
//!
//! Covers:
//! - SmartBookmarkCollection creation with filtering rules (tag match, category, date range)
//! - Auto-population of bookmarked projects
//! - refresh_smart_bookmark_collection endpoint
//! - Nesting within user bookmark folders
//! - Rule application and membership consistency

use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::{ProjectRegistrationParams, SmartBookmarkRules};
use crate::DongleContractClient;
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};

fn register_custom_project(
    client: &DongleContractClient<'_>,
    owner: &Address,
    name: &str,
    category: &str,
    tags: Option<Vec<String>>,
) -> u64 {
    let env = &client.env;
    let slug = name.to_lowercase().replace(' ', "-");
    client.mock_all_auths().register_project(&ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(env, name),
        slug: String::from_str(env, &slug),
        description: String::from_str(env, "Description for testing"),
        category: String::from_str(env, category),
        website: None,
        license: None,
        logo_cid: None,
        metadata_cid: None,
        tags,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
        repository_url: None,
    })
}

#[test]
fn test_create_smart_bookmark_collection_with_category_and_tag() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_contract(&env);

    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    let mut defi_tags = Vec::new(&env);
    defi_tags.push_back(String::from_str(&env, "dex"));
    defi_tags.push_back(String::from_str(&env, "amm"));

    let mut nft_tags = Vec::new(&env);
    nft_tags.push_back(String::from_str(&env, "art"));

    let p1 = register_custom_project(&client, &owner, "Dex Project", "DeFi", Some(defi_tags));
    let p2 = register_custom_project(&client, &owner, "Lend Project", "DeFi", None);
    let p3 = register_custom_project(&client, &owner, "NFT Project", "NFT", Some(nft_tags));

    // User bookmarks p1, p2, and p3
    client.bookmark_project(&p1, &user);
    client.bookmark_project(&p2, &user);
    client.bookmark_project(&p3, &user);

    // Rule: category == DeFi and tag == dex
    let rules = SmartBookmarkRules {
        category: Some(String::from_str(&env, "DeFi")),
        tag: Some(String::from_str(&env, "dex")),
        min_date: None,
        max_date: None,
        parent_id: None,
    };

    let col_id = client
        .create_smart_bookmark_collection(&user, &String::from_str(&env, "DeFi DEXes"), &rules);

    let col = client.get_smart_bookmark_collection(&col_id).unwrap();
    assert_eq!(col.id, col_id);
    assert_eq!(col.owner, user);
    assert_eq!(col.name, String::from_str(&env, "DeFi DEXes"));
    assert_eq!(col.project_ids.len(), 1);
    assert_eq!(col.project_ids.get(0).unwrap(), p1);
}

#[test]
fn test_refresh_smart_bookmark_collection_membership_consistency() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_contract(&env);

    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    let p1 = register_custom_project(&client, &owner, "DeFi Alpha", "DeFi", None);
    let p2 = register_custom_project(&client, &owner, "DeFi Beta", "DeFi", None);

    // User initially only bookmarks p1
    client.bookmark_project(&p1, &user);

    let rules = SmartBookmarkRules {
        category: Some(String::from_str(&env, "DeFi")),
        tag: None,
        min_date: None,
        max_date: None,
        parent_id: None,
    };

    let col_id = client
        .create_smart_bookmark_collection(&user, &String::from_str(&env, "DeFi All"), &rules);

    let col = client.get_smart_bookmark_collection(&col_id).unwrap();
    assert_eq!(col.project_ids.len(), 1);
    assert_eq!(col.project_ids.get(0).unwrap(), p1);

    // Now user bookmarks p2
    client.bookmark_project(&p2, &user);

    // Refresh smart bookmark collection
    let refreshed_ids = client.refresh_smart_bookmark_collection(&col_id);
    assert_eq!(refreshed_ids.len(), 2);
    assert!(refreshed_ids.contains(p1));
    assert!(refreshed_ids.contains(p2));

    // Verify stored state is updated
    let col_after = client.get_smart_bookmark_collection(&col_id).unwrap();
    assert_eq!(col_after.project_ids.len(), 2);
}

#[test]
fn test_nesting_smart_collection_in_user_bookmark_folder() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_contract(&env);

    let user = Address::generate(&env);

    // Create user bookmark folder
    let folder_id =
        client.create_bookmark_folder(&user, &String::from_str(&env, "Root Folder"), &None);

    let rules = SmartBookmarkRules {
        category: None,
        tag: None,
        min_date: None,
        max_date: None,
        parent_id: Some(folder_id),
    };

    let col_id = client
        .create_smart_bookmark_collection(&user, &String::from_str(&env, "Nested Smart"), &rules);

    let col = client.get_smart_bookmark_collection(&col_id).unwrap();
    assert_eq!(col.parent_id, Some(folder_id));
}

#[test]
fn test_nesting_fails_if_parent_folder_does_not_exist() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _) = setup_contract(&env);

    let user = Address::generate(&env);

    let rules = SmartBookmarkRules {
        category: None,
        tag: None,
        min_date: None,
        max_date: None,
        parent_id: Some(999), // nonexistent
    };

    let result = client.try_create_smart_bookmark_collection(
        &user,
        &String::from_str(&env, "Invalid Nested"),
        &rules,
    );
    assert_eq!(result, Err(Ok(ContractError::FolderNotFound)));
}
