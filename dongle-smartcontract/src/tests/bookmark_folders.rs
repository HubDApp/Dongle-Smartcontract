
//! Tests for bookmark folders and smart folders (issue #815).
//!
//! Coverage:
//! - create / delete / rename folders
//! - nested folders (parent_id) and depth enforcement
//! - move bookmark between folders
//! - remove bookmark from folder
//! - get_folder_bookmarks pagination
//! - get_bookmark_folder (index query)
//! - create / delete / list smart folders
//! - get_smart_folder_bookmarks with filters (category, verification_status)
//! - auth enforcement (caller must be the user)

extern crate alloc;

use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::{SmartFolderFilter, VerificationStatus, VerificationStatusFilter};
use soroban_sdk::{testutils::Address as _, Address, Env, String};

// ── Folder CRUD ───────────────────────────────────────────────────────────────

#[test]
fn test_create_folder_success() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let user = Address::generate(&env);

    let folder_id = client
        .create_bookmark_folder(&user, &String::from_str(&env, "My Folder"), &None);

    assert_eq!(folder_id, 1);

    let folder = client.get_bookmark_folder_by_id(&user, &folder_id);
    assert!(folder.is_some());
    let f = folder.unwrap();
    assert_eq!(f.id, 1);
    assert_eq!(f.owner, user);
    assert_eq!(f.name, String::from_str(&env, "My Folder"));
    assert!(f.parent_id.is_none());
}

#[test]
fn test_create_multiple_folders_increments_ids() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let user = Address::generate(&env);

    let id1 = client.create_bookmark_folder(&user, &String::from_str(&env, "Folder 1"), &None);
    let id2 = client.create_bookmark_folder(&user, &String::from_str(&env, "Folder 2"), &None);
    let id3 = client.create_bookmark_folder(&user, &String::from_str(&env, "Folder 3"), &None);

    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
    assert_eq!(id3, 3);
}

#[test]
fn test_folder_ids_are_per_user() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);

    let id_u1 = client.create_bookmark_folder(&user1, &String::from_str(&env, "U1F"), &None);
    let id_u2 = client.create_bookmark_folder(&user2, &String::from_str(&env, "U2F"), &None);

    // Both start at 1 because the counter is per-user.
    assert_eq!(id_u1, 1);
    assert_eq!(id_u2, 1);

    // User 2 cannot see user 1's folder.
    assert!(client.get_bookmark_folder_by_id(&user2, &id_u1).is_none());
}

#[test]
fn test_delete_folder_success() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let user = Address::generate(&env);

    let folder_id = client.create_bookmark_folder(&user, &String::from_str(&env, "ToDelete"), &None);

    client.delete_bookmark_folder(&user, &folder_id);

    assert!(client
        .get_bookmark_folder_by_id(&user, &folder_id)
        .is_none());
}

#[test]
fn test_delete_nonexistent_folder_returns_error() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let user = Address::generate(&env);

    let result = client.try_delete_bookmark_folder(&user, &99u64);
    assert_eq!(result, Err(Ok(ContractError::FolderNotFound)));
}

#[test]
fn test_rename_folder_success() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let user = Address::generate(&env);

    let folder_id = client.create_bookmark_folder(&user, &String::from_str(&env, "OldName"), &None);

    client.rename_bookmark_folder(&user, &folder_id, &String::from_str(&env, "NewName"));

    let folder = client
        .get_bookmark_folder_by_id(&user, &folder_id)
        .unwrap();
    assert_eq!(folder.name, String::from_str(&env, "NewName"));
}

#[test]
fn test_rename_nonexistent_folder_returns_error() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let user = Address::generate(&env);

    let result = client.try_rename_bookmark_folder(
        &user,
        &99u64,
        &String::from_str(&env, "NewName"),
    );
    assert_eq!(result, Err(Ok(ContractError::FolderNotFound)));
}

#[test]
fn test_list_bookmark_folders_empty_initially() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let user = Address::generate(&env);

    let folders = client.list_bookmark_folders(&user);
    assert_eq!(folders.len(), 0);
}

#[test]
fn test_list_bookmark_folders_returns_all_folders() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let user = Address::generate(&env);

    client.create_bookmark_folder(&user, &String::from_str(&env, "A"), &None);
    client.create_bookmark_folder(&user, &String::from_str(&env, "B"), &None);
    client.create_bookmark_folder(&user, &String::from_str(&env, "C"), &None);

    let folders = client.list_bookmark_folders(&user);
    assert_eq!(folders.len(), 3);
}

#[test]
fn test_delete_removes_folder_from_list() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let user = Address::generate(&env);

    client.create_bookmark_folder(&user, &String::from_str(&env, "A"), &None);
    let b_id = client.create_bookmark_folder(&user, &String::from_str(&env, "B"), &None);
    client.create_bookmark_folder(&user, &String::from_str(&env, "C"), &None);

    client.delete_bookmark_folder(&user, &b_id);

    let folders = client.list_bookmark_folders(&user);
    assert_eq!(folders.len(), 2);
    // Verify B is not in the list
    for i in 0..folders.len() {
        let f = folders.get(i).unwrap();
        assert_ne!(f.id, b_id);
    }
}

// ── Nested folders ─────────────────────────────────────────────────────────────

#[test]
fn test_create_nested_folder_success() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let user = Address::generate(&env);

    let parent_id = client.create_bookmark_folder(&user, &String::from_str(&env, "Parent"), &None);
    let child_id = client.create_bookmark_folder(
        &user,
        &String::from_str(&env, "Child"),
        &Some(parent_id),
    );

    let child = client
        .get_bookmark_folder_by_id(&user, &child_id)
        .unwrap();
    assert_eq!(child.parent_id, Some(parent_id));
}

#[test]
fn test_list_child_folders() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let user = Address::generate(&env);

    let root_id = client.create_bookmark_folder(&user, &String::from_str(&env, "Root"), &None);
    client.create_bookmark_folder(&user, &String::from_str(&env, "Child1"), &Some(root_id));
    client.create_bookmark_folder(&user, &String::from_str(&env, "Child2"), &Some(root_id));
    // A root-level sibling — should NOT appear in child list.
    client.create_bookmark_folder(&user, &String::from_str(&env, "Sibling"), &None);

    let children = client.list_child_bookmark_folders(&user, &root_id);
    assert_eq!(children.len(), 2);
    for i in 0..children.len() {
        let c = children.get(i).unwrap();
        assert_eq!(c.parent_id, Some(root_id));
    }
}

#[test]
fn test_nested_folder_depth_limit_enforced() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let user = Address::generate(&env);

    // Create 5 levels (depth 0..4)
    let id0 = client.create_bookmark_folder(&user, &String::from_str(&env, "L0"), &None);
    let id1 = client.create_bookmark_folder(&user, &String::from_str(&env, "L1"), &Some(id0));
    let id2 = client.create_bookmark_folder(&user, &String::from_str(&env, "L2"), &Some(id1));
    let id3 = client.create_bookmark_folder(&user, &String::from_str(&env, "L3"), &Some(id2));
    let id4 = client.create_bookmark_folder(&user, &String::from_str(&env, "L4"), &Some(id3));

    // Adding a 6th level (depth 5) should fail with FolderDepthExceeded.
    let result = client.try_create_bookmark_folder(
        &user,
        &String::from_str(&env, "TooDeep"),
        &Some(id4),
    );
    assert_eq!(result, Err(Ok(ContractError::FolderDepthExceeded)));
}

#[test]
fn test_nested_folder_nonexistent_parent_returns_error() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let user = Address::generate(&env);

    let result = client.try_create_bookmark_folder(
        &user,
        &String::from_str(&env, "Orphan"),
        &Some(9999u64),
    );
    assert_eq!(result, Err(Ok(ContractError::FolderNotFound)));
}

// ── Move bookmarks between folders ───────────────────────────────────────────

#[test]
fn test_move_bookmark_to_folder_success() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "MovableProject");
    client.bookmark_project(&project_id, &user);

    let folder_id = client.create_bookmark_folder(&user, &String::from_str(&env, "Folder"), &None);

    client.move_bookmark_to_folder(&user, &project_id, &folder_id);

    // Verify index
    assert_eq!(
        client.get_bookmark_folder(&user, &project_id),
        Some(folder_id)
    );

    // Verify folder bookmark list
    let bms = client.get_folder_bookmarks(&user, &folder_id, &0, &10);
    assert_eq!(bms.len(), 1);
    assert_eq!(bms.get(0).unwrap(), project_id);
}

#[test]
fn test_move_bookmark_between_folders() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "MoveMe");
    client.bookmark_project(&project_id, &user);

    let folder_a = client.create_bookmark_folder(&user, &String::from_str(&env, "A"), &None);
    let folder_b = client.create_bookmark_folder(&user, &String::from_str(&env, "B"), &None);

    client.move_bookmark_to_folder(&user, &project_id, &folder_a);
    assert_eq!(
        client.get_bookmark_folder(&user, &project_id),
        Some(folder_a)
    );

    // Move to folder B.
    client.move_bookmark_to_folder(&user, &project_id, &folder_b);
    assert_eq!(
        client.get_bookmark_folder(&user, &project_id),
        Some(folder_b)
    );

    // Folder A should be empty.
    let bms_a = client.get_folder_bookmarks(&user, &folder_a, &0, &10);
    assert_eq!(bms_a.len(), 0);

    // Folder B should contain the project.
    let bms_b = client.get_folder_bookmarks(&user, &folder_b, &0, &10);
    assert_eq!(bms_b.len(), 1);
}

#[test]
fn test_move_bookmark_idempotent_same_folder() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "IdempotentMove");
    client.bookmark_project(&project_id, &user);

    let folder_id = client.create_bookmark_folder(&user, &String::from_str(&env, "F"), &None);

    client.move_bookmark_to_folder(&user, &project_id, &folder_id);
    // Moving to the same folder again should not error.
    client.move_bookmark_to_folder(&user, &project_id, &folder_id);

    // Folder should still have exactly one entry.
    let bms = client.get_folder_bookmarks(&user, &folder_id, &0, &10);
    assert_eq!(bms.len(), 1);
}

#[test]
fn test_move_unbookmarked_project_returns_error() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "NotBookmarked");
    // Do NOT bookmark.

    let folder_id = client.create_bookmark_folder(&user, &String::from_str(&env, "F"), &None);

    let result = client.try_move_bookmark_to_folder(&user, &project_id, &folder_id);
    assert_eq!(result, Err(Ok(ContractError::NotBookmarked)));
}

#[test]
fn test_move_to_nonexistent_folder_returns_error() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "InFolderExists");
    client.bookmark_project(&project_id, &user);

    let result = client.try_move_bookmark_to_folder(&user, &project_id, &9999u64);
    assert_eq!(result, Err(Ok(ContractError::FolderNotFound)));
}

#[test]
fn test_remove_bookmark_from_folder_success() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "RemoveFromFolder");
    client.bookmark_project(&project_id, &user);

    let folder_id = client.create_bookmark_folder(&user, &String::from_str(&env, "F"), &None);
    client.move_bookmark_to_folder(&user, &project_id, &folder_id);

    client.remove_bookmark_from_folder(&user, &project_id);

    // Index cleared.
    assert!(client.get_bookmark_folder(&user, &project_id).is_none());

    // Still bookmarked in the flat list.
    assert!(client.is_bookmarked(&project_id, &user));

    // Folder is empty.
    let bms = client.get_folder_bookmarks(&user, &folder_id, &0, &10);
    assert_eq!(bms.len(), 0);
}

#[test]
fn test_remove_bookmark_from_folder_not_in_any_folder_returns_error() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "NoFolder");
    client.bookmark_project(&project_id, &user);

    let result = client.try_remove_bookmark_from_folder(&user, &project_id);
    assert_eq!(result, Err(Ok(ContractError::FolderNotFound)));
}

#[test]
fn test_unbookmark_clears_folder_index() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "ClearOnUnbookmark");
    client.bookmark_project(&project_id, &user);

    let folder_id = client.create_bookmark_folder(&user, &String::from_str(&env, "F"), &None);
    client.move_bookmark_to_folder(&user, &project_id, &folder_id);

    // Unbookmarking should also clear the folder index.
    client.unbookmark_project(&project_id, &user);

    assert!(client.get_bookmark_folder(&user, &project_id).is_none());
    let bms = client.get_folder_bookmarks(&user, &folder_id, &0, &10);
    assert_eq!(bms.len(), 0);
}

#[test]
fn test_get_folder_bookmarks_pagination() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    let folder_id =
        client.create_bookmark_folder(&user, &String::from_str(&env, "Paginated"), &None);

    // Add 5 bookmarks and move them all into the folder.
    let names = ["P0", "P1", "P2", "P3", "P4"];
    for name in names {
        let pid = create_test_project(&client, &owner, name);
        client.bookmark_project(&pid, &user);
        client.move_bookmark_to_folder(&user, &pid, &folder_id);
    }

    let page1 = client.get_folder_bookmarks(&user, &folder_id, &0, &3);
    assert_eq!(page1.len(), 3);

    let page2 = client.get_folder_bookmarks(&user, &folder_id, &3, &3);
    assert_eq!(page2.len(), 2);

    let empty = client.get_folder_bookmarks(&user, &folder_id, &10, &3);
    assert_eq!(empty.len(), 0);
}

#[test]
fn test_delete_folder_retains_flat_bookmarks() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "RetainBookmark");
    client.bookmark_project(&project_id, &user);

    let folder_id = client.create_bookmark_folder(&user, &String::from_str(&env, "F"), &None);
    client.move_bookmark_to_folder(&user, &project_id, &folder_id);

    // Delete the folder — bookmarks must survive.
    client.delete_bookmark_folder(&user, &folder_id);

    assert!(client.is_bookmarked(&project_id, &user));
    // Index cleared.
    assert!(client.get_bookmark_folder(&user, &project_id).is_none());
}

// ── Smart Folders ─────────────────────────────────────────────────────────────

fn no_filter(env: &Env) -> SmartFolderFilter {
    SmartFolderFilter {
        category: None,
        tag: None,
        verification_status: VerificationStatusFilter::Any,
    }
}

#[test]
fn test_create_smart_folder_success() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let user = Address::generate(&env);

    let sf_id = client
        .create_smart_folder(&user, &String::from_str(&env, "All Bookmarks"), &no_filter(&env));

    assert_eq!(sf_id, 1);

    let sf = client.get_smart_folder(&user, &sf_id).unwrap();
    assert_eq!(sf.id, 1);
    assert_eq!(sf.owner, user);
}

#[test]
fn test_delete_smart_folder_success() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let user = Address::generate(&env);

    let sf_id = client
        .create_smart_folder(&user, &String::from_str(&env, "ToDelete"), &no_filter(&env));

    client.delete_smart_folder(&user, &sf_id);

    assert!(client.get_smart_folder(&user, &sf_id).is_none());
}

#[test]
fn test_delete_nonexistent_smart_folder_returns_error() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let user = Address::generate(&env);

    let result = client.try_delete_smart_folder(&user, &99u64);
    assert_eq!(result, Err(Ok(ContractError::SmartFolderNotFound)));
}

#[test]
fn test_list_smart_folders_empty_initially() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let user = Address::generate(&env);

    let sfs = client.list_smart_folders(&user);
    assert_eq!(sfs.len(), 0);
}

#[test]
fn test_list_smart_folders_returns_all() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let user = Address::generate(&env);

    let filter = no_filter(&env);
    client.create_smart_folder(&user, &String::from_str(&env, "SF-0"), &filter);
    client.create_smart_folder(&user, &String::from_str(&env, "SF-1"), &filter);
    client.create_smart_folder(&user, &String::from_str(&env, "SF-2"), &filter);

    assert_eq!(client.list_smart_folders(&user).len(), 3);
}

#[test]
fn test_smart_folder_match_all_returns_all_bookmarks() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    let p1 = create_test_project(&client, &owner, "SF-A");
    let p2 = create_test_project(&client, &owner, "SF-B");
    client.bookmark_project(&p1, &user);
    client.bookmark_project(&p2, &user);

    let sf_id = client
        .create_smart_folder(&user, &String::from_str(&env, "All"), &no_filter(&env));

    let result = client.get_smart_folder_bookmarks(&user, &sf_id, &0, &10);
    assert_eq!(result.len(), 2);
}

#[test]
fn test_smart_folder_category_filter() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    use crate::types::ProjectRegistrationParams;

    // Register one DeFi and one NFT project.
    let defi_params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "DeFiProject"),
        slug: String::from_str(&env, "defi-project"),
        description: String::from_str(&env, "A DeFi project"),
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
    };
    let nft_params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "NFTProject"),
        slug: String::from_str(&env, "nft-project"),
        description: String::from_str(&env, "An NFT project"),
        category: String::from_str(&env, "NFT"),
        website: None,
        license: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
        repository_url: None,
    };

    let defi_id = client.register_project(&defi_params);
    let nft_id = client.register_project(&nft_params);

    client.bookmark_project(&defi_id, &user);
    client.bookmark_project(&nft_id, &user);

    let filter = SmartFolderFilter {
        category: Some(String::from_str(&env, "DeFi")),
        tag: None,
        verification_status: VerificationStatusFilter::Any,
    };
    let sf_id = client.create_smart_folder(&user, &String::from_str(&env, "DeFi Only"), &filter);

    let result = client.get_smart_folder_bookmarks(&user, &sf_id, &0, &10);
    assert_eq!(result.len(), 1);
    assert_eq!(result.get(0).unwrap(), defi_id);
}

#[test]
fn test_smart_folder_verification_status_filter() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    let p1 = create_test_project(&client, &owner, "Unverified1");
    let p2 = create_test_project(&client, &owner, "Unverified2");

    client.bookmark_project(&p1, &user);
    client.bookmark_project(&p2, &user);

    // Filter for Verified projects — none of these are verified.
    let verified_filter = SmartFolderFilter {
        category: None,
        tag: None,
        verification_status: VerificationStatusFilter::Is(VerificationStatus::Verified),
    };
    let sf_verified = client.create_smart_folder(
        &user,
        &String::from_str(&env, "Verified Only"),
        &verified_filter,
    );

    let result = client.get_smart_folder_bookmarks(&user, &sf_verified, &0, &10);
    assert_eq!(result.len(), 0);

    // Filter for Unverified — both should match.
    let unverified_filter = SmartFolderFilter {
        category: None,
        tag: None,
        verification_status: VerificationStatusFilter::Is(VerificationStatus::Unverified),
    };
    let sf_unverified = client.create_smart_folder(
        &user,
        &String::from_str(&env, "Unverified Only"),
        &unverified_filter,
    );

    let result2 = client.get_smart_folder_bookmarks(&user, &sf_unverified, &0, &10);
    assert_eq!(result2.len(), 2);
}

#[test]
fn test_smart_folder_get_nonexistent_returns_error() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let user = Address::generate(&env);

    let result = client.try_get_smart_folder_bookmarks(&user, &99u64, &0, &10);
    assert_eq!(result, Err(Ok(ContractError::SmartFolderNotFound)));
}

#[test]
fn test_smart_folder_pagination() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    let names = ["SP0", "SP1", "SP2", "SP3", "SP4", "SP5"];
    for name in names {
        let pid = create_test_project(&client, &owner, name);
        client.bookmark_project(&pid, &user);
    }

    let sf_id = client
        .create_smart_folder(&user, &String::from_str(&env, "All"), &no_filter(&env));

    let page1 = client.get_smart_folder_bookmarks(&user, &sf_id, &0, &4);
    assert_eq!(page1.len(), 4);

    let page2 = client.get_smart_folder_bookmarks(&user, &sf_id, &4, &4);
    assert_eq!(page2.len(), 2);
}

// ── Auth enforcement ──────────────────────────────────────────────────────────

#[test]
fn test_create_folder_unauthorized_fails() {
    let env = Env::default(); // No mock_all_auths
    let (client, _admin) = setup_contract(&env);
    let user = Address::generate(&env);

    let result = client
        .try_create_bookmark_folder(&user, &String::from_str(&env, "Unauthorized"), &None);
    assert!(result.is_err(), "should fail without auth");
}

#[test]
fn test_move_bookmark_unauthorized_fails() {
    let env = Env::default(); // No mock_all_auths
    let (client, _admin) = setup_contract(&env);
    let user = Address::generate(&env);

    let result = client.try_move_bookmark_to_folder(&user, &1u64, &1u64);
    assert!(result.is_err(), "should fail without auth");
}
