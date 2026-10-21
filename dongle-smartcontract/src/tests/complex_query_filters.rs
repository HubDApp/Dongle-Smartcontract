#![cfg(test)]

use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::{
    FilterLogic, ProjectFilterCriteria, ProjectFilterSortBy, ProjectQueryFilter, VerificationStatus,
};
use soroban_sdk::{testutils::Address as _, Address, Env, String};

#[test]
fn test_query_projects_with_and_filter() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let p1 = create_test_project(&client, &owner, "AlphaDefi");
    let p2 = create_test_project(&client, &owner, "BetaGaming");

    // Filter by category "DeFi" AND min_rating 0
    let criteria = ProjectFilterCriteria {
        category: Some(String::from_str(&env, "DeFi")),
        min_rating: Some(0),
        verification_status: None,
        owner_address: None,
        featured_only: None,
    };
    let query_filter = ProjectQueryFilter {
        criteria,
        logic: FilterLogic::And,
        sort_by: Some(ProjectFilterSortBy::Name),
        ascending: Some(true),
    };

    let result = client.query_projects(&query_filter, &None, &10);
    assert_eq!(result.total_matched, 1);
    assert_eq!(result.projects.len(), 1);
    assert_eq!(result.projects.get(0).unwrap().id, p1);
}

#[test]
fn test_query_projects_with_or_filter() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner1 = Address::generate(&env);
    let owner2 = Address::generate(&env);

    let p1 = create_test_project(&client, &owner1, "DeFiOne");
    let p2 = create_test_project(&client, &owner2, "GamingTwo");

    // Filter by category "Gaming" OR owner == owner1
    let criteria = ProjectFilterCriteria {
        category: Some(String::from_str(&env, "Gaming")),
        min_rating: None,
        verification_status: None,
        owner_address: Some(owner1.clone()),
        featured_only: None,
    };
    let query_filter = ProjectQueryFilter {
        criteria,
        logic: FilterLogic::Or,
        sort_by: Some(ProjectFilterSortBy::RegistrationDate),
        ascending: Some(true),
    };

    let result = client.query_projects(&query_filter, &None, &10);
    assert_eq!(result.total_matched, 2);
    assert_eq!(result.projects.len(), 2);
}

#[test]
fn test_list_projects_filtered_cursor_pagination() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let p1 = create_test_project(&client, &owner, "Proj1");
    let p2 = create_test_project(&client, &owner, "Proj2");
    let p3 = create_test_project(&client, &owner, "Proj3");

    // Page 1: limit 2
    let page1 = client.list_projects_filtered(
        &None,
        &None,
        &None,
        &None,
        &None,
        &Some(FilterLogic::And),
        &Some(ProjectFilterSortBy::RegistrationDate),
        &None,
        &2,
    );

    assert_eq!(page1.projects.len(), 2);
    assert!(page1.has_more);
    assert!(page1.next_cursor.is_some());

    // Page 2: with cursor
    let page2 = client.list_projects_filtered(
        &None,
        &None,
        &None,
        &None,
        &None,
        &Some(FilterLogic::And),
        &Some(ProjectFilterSortBy::RegistrationDate),
        &page1.next_cursor,
        &2,
    );

    assert_eq!(page2.projects.len(), 1);
    assert_eq!(page2.projects.get(0).unwrap().id, p3);
    assert!(!page2.has_more);
    assert!(page2.next_cursor.is_none());
}
