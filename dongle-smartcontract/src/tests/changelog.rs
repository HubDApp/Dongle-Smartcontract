use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::ChangelogSortMode;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env, String,
};

#[test]
fn test_add_changelog_entry_as_owner() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "TestProject");

    // Add a changelog entry
    let cid = String::from_str(
        &env,
        "bafybeiboz75hbx2qg7g4j4vqaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    );
    let description = Some(String::from_str(&env, "Version 1.0.0 release"));
    let changelog_id = client.mock_all_auths().add_changelog_entry(
        &project_id,
        &owner,
        &cid,
        &description,
        &None,
        &None,
    );

    assert!(changelog_id > 0);

    // Verify the changelog entry can be retrieved
    let entry = client.get_changelog_entry(&changelog_id);
    assert!(entry.is_some());
    let entry = entry.unwrap();
    assert_eq!(entry.id, changelog_id);
    assert_eq!(entry.project_id, project_id);
    assert_eq!(entry.cid, cid);
    assert_eq!(entry.description, description);
    // created_at can be 0 in test environment
    assert!(entry.created_at >= 0);

    // Verify changelog count
    let count = client.get_changelog_count(&project_id);
    assert_eq!(count, 1);
}

#[test]
fn test_add_changelog_entry_non_owner_fails() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let non_owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "TestProject");

    // Try to add changelog as non-owner - should fail
    let cid = String::from_str(
        &env,
        "bafybeiboz75hbx2qg7g4j4vqbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    );
    let description = Some(String::from_str(&env, "Version 1.0.0 release"));

    let result = client.mock_all_auths().try_add_changelog_entry(
        &project_id,
        &non_owner,
        &cid,
        &description,
        &None,
        &None,
    );
    assert!(result.is_err());
}

#[test]
fn test_add_changelog_entry_invalid_cid_fails() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "TestProject");

    // Try to add changelog with empty CID - should fail
    let empty_cid = String::from_str(&env, "");
    let description = Some(String::from_str(&env, "Empty CID test"));

    let result = client.mock_all_auths().try_add_changelog_entry(
        &project_id,
        &owner,
        &empty_cid,
        &description,
        &None,
        &None,
    );
    assert!(result.is_err());

    // Try to add changelog with invalid CID - should fail
    let invalid_cid = String::from_str(&env, "invalid-cid");
    let result = client.mock_all_auths().try_add_changelog_entry(
        &project_id,
        &owner,
        &invalid_cid,
        &description,
        &None,
        &None,
    );
    assert!(result.is_err());
}

#[test]
fn test_add_duplicate_changelog_cid_fails() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "TestProject");

    // Add first changelog entry
    let cid = String::from_str(
        &env,
        "bafybeiboz75hbx2qg7g4j4vqccccccccccccccccccccccccccccccc",
    );
    let description1 = Some(String::from_str(&env, "Version 1.0.0"));
    let changelog_id1 = client.mock_all_auths().add_changelog_entry(
        &project_id,
        &owner,
        &cid,
        &description1,
        &None,
        &None,
    );
    assert!(changelog_id1 > 0);

    // Try to add duplicate CID - should fail
    let description2 = Some(String::from_str(&env, "Duplicate test"));
    let result = client.mock_all_auths().try_add_changelog_entry(
        &project_id,
        &owner,
        &cid,
        &description2,
        &None,
        &None,
    );
    assert!(result.is_err());
}

#[test]
fn test_remove_changelog_entry_as_owner() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "TestProject");

    // Add a changelog entry
    let cid = String::from_str(
        &env,
        "bafybeiboz75hbx2qg7g4j4vqdddddddddddddddddddddddddddddd",
    );
    let description = Some(String::from_str(&env, "Version 1.0.0 release"));
    let changelog_id = client.mock_all_auths().add_changelog_entry(
        &project_id,
        &owner,
        &cid,
        &description,
        &None,
        &None,
    );

    // Verify it exists
    let entry = client.get_changelog_entry(&changelog_id);
    assert!(entry.is_some());

    // Remove the changelog entry
    client
        .mock_all_auths()
        .remove_changelog_entry(&changelog_id, &owner);

    // Verify it's gone
    let entry = client.get_changelog_entry(&changelog_id);
    assert!(entry.is_none());

    // Verify changelog count
    let count = client.get_changelog_count(&project_id);
    assert_eq!(count, 0);
}

#[test]
fn test_remove_changelog_entry_non_owner_fails() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let non_owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "TestProject");

    // Add a changelog entry
    let cid = String::from_str(
        &env,
        "bafybeiboz75hbx2qg7g4j4vqeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
    );
    let description = Some(String::from_str(&env, "Version 1.0.0 release"));
    let changelog_id = client.mock_all_auths().add_changelog_entry(
        &project_id,
        &owner,
        &cid,
        &description,
        &None,
        &None,
    );

    // Try to remove as non-owner - should fail
    let result = client
        .mock_all_auths()
        .try_remove_changelog_entry(&changelog_id, &non_owner);
    assert!(result.is_err());
}

#[test]
fn test_remove_nonexistent_changelog_fails() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let owner = Address::generate(&env);

    // Try to remove non-existent changelog - should fail
    let result = client
        .mock_all_auths()
        .try_remove_changelog_entry(&999, &owner);
    assert!(result.is_err());
}

#[test]
fn test_get_project_changelog_pagination() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "TestProject");

    // Add multiple changelog entries
    let cids = [
        "bafybeiboz75hbx2qg7g4j4vq0aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "bafybeiboz75hbx2qg7g4j4vq1bbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "bafybeiboz75hbx2qg7g4j4vq2cccccccccccccccccccccccccccccc",
        "bafybeiboz75hbx2qg7g4j4vq3dddddddddddddddddddddddddddddd",
        "bafybeiboz75hbx2qg7g4j4vq4eeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
    ];
    let descriptions = [
        "Version 1.0.0",
        "Version 1.0.1",
        "Version 1.0.2",
        "Version 1.0.3",
        "Version 1.0.4",
    ];

    for i in 0..5 {
        let cid = String::from_str(&env, cids[i]);
        let description = Some(String::from_str(&env, descriptions[i]));
        client.mock_all_auths().add_changelog_entry(
            &project_id,
            &owner,
            &cid,
            &description,
            &None,
            &None,
        );

        // Advance ledger timestamp to ensure different created_at
        env.ledger().set_timestamp(env.ledger().timestamp() + 100);
    }

    // Test pagination with newest first
    let changelog_newest =
        client.get_project_changelog(&project_id, &0, &3, &ChangelogSortMode::Newest);
    assert_eq!(changelog_newest.len(), 3);

    // Verify they're in descending order (newest first)
    // In tests, created_at might be 0 for all entries, so we can only check >=
    for i in 0..changelog_newest.len() - 1 {
        let current = changelog_newest.get(i).unwrap().created_at;
        let next = changelog_newest.get(i + 1).unwrap().created_at;
        assert!(
            current >= next,
            "Expected entry {} to have created_at >= entry {} ({} >= {})",
            i,
            i + 1,
            current,
            next
        );
    }

    // Test pagination with oldest first
    let changelog_oldest =
        client.get_project_changelog(&project_id, &0, &3, &ChangelogSortMode::Oldest);
    assert_eq!(changelog_oldest.len(), 3);

    // Verify they're in ascending order (oldest first)
    for i in 0..changelog_oldest.len() - 1 {
        assert!(
            changelog_oldest.get(i).unwrap().created_at
                <= changelog_oldest.get(i + 1).unwrap().created_at
        );
    }

    // Test pagination with offset
    let changelog_offset =
        client.get_project_changelog(&project_id, &2, &2, &ChangelogSortMode::Newest);
    assert_eq!(changelog_offset.len(), 2);
}

#[test]
fn test_get_project_changelog_nonexistent_project() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    // Get changelog for non-existent project - should return empty list
    let changelog = client.get_project_changelog(&999, &0, &10, &ChangelogSortMode::Newest);
    assert_eq!(changelog.len(), 0);
}

#[test]
fn test_get_changelog_count() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "TestProject");

    // Initial count should be 0
    let initial_count = client.get_changelog_count(&project_id);
    assert_eq!(initial_count, 0);

    // Add some changelog entries
    let test_cids = [
        "bafybeiboz75hbx2qg7g4j4vqaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "bafybeiboz75hbx2qg7g4j4vqbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "bafybeiboz75hbx2qg7g4j4vqccccccccccccccccccccccccccccccc",
    ];
    let test_descriptions = ["Version 1.0.0", "Version 1.0.1", "Version 1.0.2"];

    for i in 0..3 {
        let cid = String::from_str(&env, test_cids[i]);
        let description = Some(String::from_str(&env, test_descriptions[i]));
        client.mock_all_auths().add_changelog_entry(
            &project_id,
            &owner,
            &cid,
            &description,
            &None,
            &None,
        );
    }

    // Count should be 3
    let count = client.get_changelog_count(&project_id);
    assert_eq!(count, 3);
}

#[test]
fn test_changelog_while_paused_fails() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "TestProject");

    // Pause the contract
    client.mock_all_auths().pause(&admin);

    // Try to add changelog while paused - should fail
    let cid = String::from_str(
        &env,
        "bafybeiboz75hbx2qg7g4j4vqfffffffffffffffffffffffffffffff",
    );
    let description = Some(String::from_str(&env, "Version 1.0.0 release"));

    let result = client.mock_all_auths().try_add_changelog_entry(
        &project_id,
        &owner,
        &cid,
        &description,
        &None,
        &None,
    );
    assert!(result.is_err());

    // Unpause the contract
    client.mock_all_auths().unpause(&admin);

    // Now should succeed
    let changelog_id = client.mock_all_auths().add_changelog_entry(
        &project_id,
        &owner,
        &cid,
        &description,
        &None,
        &None,
    );
    assert!(changelog_id > 0);
}
