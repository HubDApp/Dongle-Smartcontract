#![cfg(test)]

use crate::tests::fixtures::{create_test_project, setup_contract};
use soroban_sdk::{testutils::Address as _, Address, Env, Vec};

#[test]
fn test_bookmark_project() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "BookmarkableProject");

    client.bookmark_project(&project_id, &user);

    let is_bm = client.is_bookmarked(&project_id, &user);
    assert!(is_bm, "project should be bookmarked");
}

#[test]
fn test_unbookmark_project() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "UnbookmarkableProject");

    client.bookmark_project(&project_id, &user);
    assert!(client.is_bookmarked(&project_id, &user));

    client.unbookmark_project(&project_id, &user);
    assert!(!client.is_bookmarked(&project_id, &user));
}

#[test]
fn test_duplicate_bookmark_returns_error() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "DupBookmarkProject");

    client.bookmark_project(&project_id, &user);

    let result = client.try_bookmark_project(&project_id, &user);
    assert!(result.is_err(), "duplicate bookmark should fail");
}

#[test]
fn test_unbookmark_missing_returns_error() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "UnbookmarkMissing");

    let result = client.try_unbookmark_project(&project_id, &user);
    assert!(result.is_err(), "unbookmark without bookmark should fail");
}

#[test]
fn test_get_user_bookmarks() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    let p1 = create_test_project(&client, &owner, "ProjectA");
    let p2 = create_test_project(&client, &owner, "ProjectB");
    let p3 = create_test_project(&client, &owner, "ProjectC");

    client.bookmark_project(&p1, &user);
    client.bookmark_project(&p2, &user);
    client.bookmark_project(&p3, &user);

    let bookmarks = client.get_user_bookmarks(&user, &0, &10);
    assert_eq!(bookmarks.len(), 3, "user should have 3 bookmarks");

    let mut ids = Vec::new(&env);
    for i in 0..bookmarks.len() {
        if let Some(id) = bookmarks.get(i) {
            ids.push_back(id);
        }
    }
    assert!(ids.contains(p1), "should contain project A");
    assert!(ids.contains(p2), "should contain project B");
    assert!(ids.contains(p3), "should contain project C");
}

#[test]
fn test_get_user_bookmarks_pagination() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    let names = ["Proj_0", "Proj_1", "Proj_2", "Proj_3"];
    for name in names {
        let project_id = create_test_project(&client, &owner, name);
        client.bookmark_project(&project_id, &user);
    }

    let page1 = client.get_user_bookmarks(&user, &0, &2);
    assert_eq!(page1.len(), 2);

    let page2 = client.get_user_bookmarks(&user, &2, &2);
    assert_eq!(page2.len(), 2);

    let empty = client.get_user_bookmarks(&user, &10, &2);
    assert_eq!(empty.len(), 0);
}

#[test]
fn test_bookmark_nonexistent_project_returns_error() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let user = Address::generate(&env);

    let result = client.try_bookmark_project(&999u64, &user);
    assert!(
        result.is_err(),
        "bookmarking nonexistent project should fail"
    );
}

#[test]
fn test_unbookmark_after_unbookmark_is_error() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "DoubleUnbookmark");

    client.bookmark_project(&project_id, &user);
    client.unbookmark_project(&project_id, &user);

    let result = client.try_unbookmark_project(&project_id, &user);
    assert!(result.is_err(), "second unbookmark should fail");
}

#[test]
fn test_bookmark_after_unbookmark_allows_rebookmark() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "RebookmarkProject");

    client.bookmark_project(&project_id, &user);
    client.unbookmark_project(&project_id, &user);
    assert!(!client.is_bookmarked(&project_id, &user));

    client.bookmark_project(&project_id, &user);
    assert!(client.is_bookmarked(&project_id, &user));
}

#[test]
fn test_multiple_users_bookmark_same_project() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "SharedBookmark");

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);

    client.bookmark_project(&project_id, &user1);
    client.bookmark_project(&project_id, &user2);

    assert!(client.is_bookmarked(&project_id, &user1));
    assert!(client.is_bookmarked(&project_id, &user2));
}

#[test]
fn test_bookmark_unauthorized_returns_error() {
    let env = Env::default();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "UnauthBookmark");

    // Do NOT call mock_all_auths - this tests unauthorized access
    let result = client.try_bookmark_project(&project_id, &user);
    assert!(result.is_err(), "unauthorized bookmark should fail");
}

#[test]
fn test_unbookmark_unauthorized_returns_error() {
    let env = Env::default();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    let project_id = create_test_project(&client, &owner, "UnauthUnbookmark");

    // Do NOT call mock_all_auths
    let result = client.try_unbookmark_project(&project_id, &user);
    assert!(result.is_err(), "unauthorized unbookmark should fail");
}
