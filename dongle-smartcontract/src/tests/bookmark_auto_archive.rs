#![cfg(test)]

use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::{BookmarkConfig, BookmarkExpirationPolicy};
use soroban_sdk::{testutils::Address as _, testutils::Ledger as _, Address, Env};

#[test]
fn test_global_bookmark_config() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, admin) = setup_contract(&env);

    let initial = client.get_bookmark_config();
    assert_eq!(initial.auto_archive_after_days, None);

    client.set_bookmark_config(&admin, &BookmarkConfig {
        auto_archive_after_days: Some(30),
    });

    let updated = client.get_bookmark_config();
    assert_eq!(updated.auto_archive_after_days, Some(30));
}

#[test]
fn test_user_and_per_bookmark_policies() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    let p1 = create_test_project(&client, &owner, "Project1");

    client.bookmark_project(&p1, &user);

    // User policy
    client.set_user_bookmark_policy(&user, &BookmarkExpirationPolicy {
        auto_archive_after_days: Some(60),
    });
    let policy = client.get_user_bookmark_policy(&user);
    assert_eq!(policy, Some(BookmarkExpirationPolicy { auto_archive_after_days: Some(60) }));

    // Per-bookmark override
    client.set_bookmark_expiration_policy(&user, &p1, &Some(14));
    let override_days = client.get_bookmark_expiration_policy(&user, &p1);
    assert_eq!(override_days, Some(14));
}

#[test]
fn test_lazy_cleanup_and_restore_archived_bookmark() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let user = Address::generate(&env);

    let p1 = create_test_project(&client, &owner, "P1");
    let p2 = create_test_project(&client, &owner, "P2");

    // Bookmark both projects at timestamp 1000
    env.ledger().set_timestamp(1_000);
    client.bookmark_project(&p1, &user);
    client.bookmark_project(&p2, &user);

    // Set global auto-archive policy to 7 days
    client.set_bookmark_config(&admin, &BookmarkConfig {
        auto_archive_after_days: Some(7),
    });

    // Advance time by 8 days (8 * 86400 = 691200 seconds)
    env.ledger().set_timestamp(1_000 + 691_200);

    // Getting user bookmarks triggers lazy cleanup
    let active = client.get_user_bookmarks(&user, &0, &10);
    assert_eq!(active.len(), 0, "both should be auto-archived");

    // Check get_archived_bookmarks
    let archived = client.get_archived_bookmarks(&user, &0, &10);
    assert_eq!(archived.len(), 2);
    assert!(archived.contains(p1));
    assert!(archived.contains(p2));

    // Restore p1
    client.restore_archived_bookmark(&user, &p1);

    let active_after = client.get_user_bookmarks(&user, &0, &10);
    assert_eq!(active_after.len(), 1);
    assert_eq!(active_after.get(0).unwrap(), p1);

    let archived_after = client.get_archived_bookmarks(&user, &0, &10);
    assert_eq!(archived_after.len(), 1);
    assert_eq!(archived_after.get(0).unwrap(), p2);
}
