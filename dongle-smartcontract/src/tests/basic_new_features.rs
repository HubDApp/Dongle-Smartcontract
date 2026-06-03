//! Basic tests for new features to verify they compile and work

#![cfg(test)]

use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::ProjectRegistrationParams;
use soroban_sdk::{testutils::Address as _, Address, Env, Map, String, Vec};

#[test]
fn test_basic_project_with_tags_and_social_links() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let user = Address::generate(&env);

    // Create tags
    let mut tags = Vec::new(&env);
    tags.push_back(String::from_str(&env, "defi"));
    tags.push_back(String::from_str(&env, "ethereum"));

    // Create social links
    let mut social_links = Map::new(&env);
    social_links.set(
        String::from_str(&env, "github"),
        String::from_str(&env, "https://github.com/example/project"),
    );

    // Create project with tags and social links
    let params = ProjectRegistrationParams {
        owner: user.clone(),
        name: String::from_str(&env, "TestProject"),
        slug: String::from_str(&env, "testproject"),
        description: String::from_str(&env, "A test project"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: Some(tags.clone()),
        social_links: Some(social_links.clone()),
    };

    let project_id = client.register_project(&params);
    let project = client.get_project(&project_id).unwrap();

    assert_eq!(project.tags, Some(tags));
    assert_eq!(project.social_links, Some(social_links));
}

#[test]
fn test_basic_reporting() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, _admin) = setup_contract(&env);
    let user = Address::generate(&env);
    let reporter = Address::generate(&env);

    // Create a project
    let project_id = create_test_project(&client, &user, "TestProject");

    // Report the project
    let reason_cid = String::from_str(&env, "QmReportReason123456789012345678901234567890123456");
    let result = client.try_report_project(&project_id, &reporter, &reason_cid);
    assert!(result.is_ok());

    // Check report count
    let count = client.get_project_report_count(&project_id);
    assert_eq!(count, 1);
}

#[test]
fn test_basic_min_project_age() {
    let env = Env::default();
    env.mock_all_auths();

    let (client, admin) = setup_contract(&env);

    // Test default minimum age is 0
    let min_age = client.get_min_project_age();
    assert_eq!(min_age, 0);

    // Test admin can set minimum age
    let new_min_age = 86400u64; // 1 day
    let result = client.try_set_min_project_age(&admin, &new_min_age);
    assert!(result.is_ok());

    let updated_min_age = client.get_min_project_age();
    assert_eq!(updated_min_age, new_min_age);
}
