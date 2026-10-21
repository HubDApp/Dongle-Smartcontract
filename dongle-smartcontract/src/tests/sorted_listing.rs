//! `list_projects_sorted` ordering and paging (#484).
//!
//! Newest/Oldest are served straight off the project ID space and the rated
//! modes select only the requested page, so these cover ordering, paging, and
//! archived projects for both paths.

use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::ProjectSortMode;
use soroban_sdk::{testutils::Address as _, Address, Env};

#[test]
fn oldest_and_newest_are_reverse_of_each_other() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let a = create_test_project(&client.mock_all_auths(), &owner, "Alpha");
    let b = create_test_project(&client.mock_all_auths(), &owner, "Beta");
    let c = create_test_project(&client.mock_all_auths(), &owner, "Gamma");

    let oldest = client.list_projects_sorted(&ProjectSortMode::Oldest, &0, &10);
    assert_eq!(oldest.len(), 3);
    assert_eq!(oldest.get(0).unwrap().id, a);
    assert_eq!(oldest.get(1).unwrap().id, b);
    assert_eq!(oldest.get(2).unwrap().id, c);

    let newest = client.list_projects_sorted(&ProjectSortMode::Newest, &0, &10);
    assert_eq!(newest.len(), 3);
    assert_eq!(newest.get(0).unwrap().id, c);
    assert_eq!(newest.get(1).unwrap().id, b);
    assert_eq!(newest.get(2).unwrap().id, a);
}

#[test]
fn start_index_pages_through_newest() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let a = create_test_project(&client.mock_all_auths(), &owner, "Alpha");
    let b = create_test_project(&client.mock_all_auths(), &owner, "Beta");
    let c = create_test_project(&client.mock_all_auths(), &owner, "Gamma");

    let first = client.list_projects_sorted(&ProjectSortMode::Newest, &0, &2);
    assert_eq!(first.len(), 2);
    assert_eq!(first.get(0).unwrap().id, c);
    assert_eq!(first.get(1).unwrap().id, b);

    let second = client.list_projects_sorted(&ProjectSortMode::Newest, &2, &2);
    assert_eq!(second.len(), 1);
    assert_eq!(second.get(0).unwrap().id, a);

    let past_end = client.list_projects_sorted(&ProjectSortMode::Newest, &3, &2);
    assert_eq!(past_end.len(), 0);
}

#[test]
fn archived_projects_are_skipped_while_paging() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let a = create_test_project(&client.mock_all_auths(), &owner, "Alpha");
    let b = create_test_project(&client.mock_all_auths(), &owner, "Beta");
    let c = create_test_project(&client.mock_all_auths(), &owner, "Gamma");

    client.mock_all_auths().archive_project(&b, &owner);

    let oldest = client.list_projects_sorted(&ProjectSortMode::Oldest, &0, &10);
    assert_eq!(oldest.len(), 2);
    assert_eq!(oldest.get(0).unwrap().id, a);
    assert_eq!(oldest.get(1).unwrap().id, c);

    // The archived project must not consume a slot in the offset either.
    let second_page = client.list_projects_sorted(&ProjectSortMode::Oldest, &1, &10);
    assert_eq!(second_page.len(), 1);
    assert_eq!(second_page.get(0).unwrap().id, c);
}

#[test]
fn empty_registry_returns_nothing() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);

    for mode in [
        ProjectSortMode::Newest,
        ProjectSortMode::Oldest,
        ProjectSortMode::HighestRated,
        ProjectSortMode::MostReviewed,
    ] {
        assert_eq!(client.list_projects_sorted(&mode, &0, &10).len(), 0);
    }
}

#[test]
fn rated_modes_page_without_dropping_or_repeating_projects() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let a = create_test_project(&client.mock_all_auths(), &owner, "Alpha");
    let b = create_test_project(&client.mock_all_auths(), &owner, "Beta");
    let c = create_test_project(&client.mock_all_auths(), &owner, "Gamma");

    // No reviews anywhere: every project ties, so paging still has to walk the
    // whole set exactly once across the two pages.
    let first = client.list_projects_sorted(&ProjectSortMode::HighestRated, &0, &2);
    let second = client.list_projects_sorted(&ProjectSortMode::HighestRated, &2, &2);
    assert_eq!(first.len(), 2);
    assert_eq!(second.len(), 1);

    let mut seen = [false; 3];
    for page in [first, second] {
        for i in 0..page.len() {
            let id = page.get(i).unwrap().id;
            let slot = if id == a {
                0
            } else if id == b {
                1
            } else if id == c {
                2
            } else {
                panic!("unexpected project id")
            };
            assert!(!seen[slot], "project returned on more than one page");
            seen[slot] = true;
        }
    }
    assert!(seen.iter().all(|s| *s));
}
