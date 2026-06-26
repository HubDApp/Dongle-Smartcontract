//! Tests for new features: minimum project age, reporting, tags, and social links

#![cfg(test)]

use crate::tests::fixtures::{create_test_project, setup_contract, setup_with_fees};
use crate::types::{ProjectRegistrationParams, ProjectUpdateParams};
use soroban_sdk::{testutils::{Address as _, Ledger}, Address, Env, Map, String, Vec};

#[test]
fn test_minimum_project_age_configuration() {
    let env = Env::default();
    let (client, admin) = setup_contract(&env);

    // Test default minimum age is 0
    let min_age = client.get_min_project_age();
    assert_eq!(min_age, 0);

    // Test admin can set minimum age
    let new_min_age = 86400u64; // 1 day
    client.mock_all_auths().set_min_project_age(&admin, &new_min_age);
    
    let updated_min_age = client.get_min_project_age();
    assert_eq!(updated_min_age, new_min_age);
}

#[test]
fn test_minimum_project_age_verification_check() {
    let env = Env::default();
    env.mock_all_auths();
    
    let (client, admin, _treasury) = setup_with_fees(&env, 1000);
    let user = Address::generate(&env);

    // Set minimum project age to 1 day
    let min_age = 86400u64;
    client.set_min_project_age(&admin, &min_age);

    // Create a project
    let project_id = create_test_project(&client, &user, "TestProject");

    // Try to request verification immediately (should fail)
    let evidence_cid = String::from_str(&env, "QmTestEvidence123456789012345678901234567890123456");
    
    // Pay verification fee first
    client.pay_fee(&user, &project_id, &None);
    
    let result = client.try_request_verification(&project_id, &user, &evidence_cid);
    assert!(result.is_err());

    // Fast forward time by 1 day
    env.ledger().with_mut(|li| {
        li.timestamp = li.timestamp + min_age;
    });

    // Now verification should work
    let result = client.try_request_verification(&project_id, &user, &evidence_cid);
    assert!(result.is_ok());
}

#[test]
fn test_project_reporting() {
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

    // Check if user has reported
    let has_reported = client.has_user_reported(&project_id, &reporter);
    assert!(has_reported);

    // Try to report again (should fail - duplicate)
    let result = client.try_report_project(&project_id, &reporter, &reason_cid);
    assert!(result.is_err());

    // Different user can report
    let reporter2 = Address::generate(&env);
    let result = client.try_report_project(&project_id, &reporter2, &reason_cid);
    assert!(result.is_ok());

    let count = client.get_project_report_count(&project_id);
    assert_eq!(count, 2);
}

#[test]
fn test_project_tags() {
    let env = Env::default();
    env.mock_all_auths();
    
    let (client, _admin) = setup_contract(&env);
    let user = Address::generate(&env);

    // Create tags
    let mut tags = Vec::new(&env);
    tags.push_back(String::from_str(&env, "defi"));
    tags.push_back(String::from_str(&env, "ethereum"));
    tags.push_back(String::from_str(&env, "smart-contracts"));

    // Create project with tags
    let params = ProjectRegistrationParams {
        owner: user.clone(),
        name: String::from_str(&env, "TaggedProject"),
        description: String::from_str(&env, "A project with tags"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: Some(tags.clone()),
        social_links: None,
    };

    let project_id = client.register_project(&params);
    let project = client.get_project(&project_id).unwrap();
    
    assert_eq!(project.tags, Some(tags.clone()));

    // Update tags
    let mut new_tags = Vec::new(&env);
    new_tags.push_back(String::from_str(&env, "nft"));
    new_tags.push_back(String::from_str(&env, "gaming"));

    let update_params = ProjectUpdateParams {
        project_id,
        caller: user.clone(),
        name: None,
        description: None,
        category: None,
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: Some(Some(new_tags.clone())),
        social_links: None,
    };

    client.update_project(&update_params);
    let updated_project = client.get_project(&project_id).unwrap();
    assert_eq!(updated_project.tags, Some(new_tags));

    // Remove tags
    let remove_params = ProjectUpdateParams {
        project_id,
        caller: user.clone(),
        name: None,
        description: None,
        category: None,
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: Some(None),
        social_links: None,
    };

    client.update_project(&remove_params);
    let final_project = client.get_project(&project_id).unwrap();
    assert_eq!(final_project.tags, None);
}

#[test]
fn test_project_social_links() {
    let env = Env::default();
    env.mock_all_auths();
    
    let (client, _admin) = setup_contract(&env);
    let user = Address::generate(&env);

    // Create social links
    let mut social_links = Map::new(&env);
    social_links.set(
        String::from_str(&env, "github"),
        String::from_str(&env, "https://github.com/example/project"),
    );
    social_links.set(
        String::from_str(&env, "twitter"),
        String::from_str(&env, "https://twitter.com/example"),
    );
    social_links.set(
        String::from_str(&env, "discord"),
        String::from_str(&env, "https://discord.gg/example"),
    );

    // Create project with social links
    let params = ProjectRegistrationParams {
        owner: user.clone(),
        name: String::from_str(&env, "SocialProject"),
        description: String::from_str(&env, "A project with social links"),
        category: String::from_str(&env, "Social"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: Some(social_links.clone()),
    };

    let project_id = client.register_project(&params);
    let project = client.get_project(&project_id).unwrap();
    
    assert_eq!(project.social_links, Some(social_links.clone()));

    // Update social links
    let mut new_social_links = Map::new(&env);
    new_social_links.set(
        String::from_str(&env, "telegram"),
        String::from_str(&env, "https://t.me/example"),
    );

    let update_params = ProjectUpdateParams {
        project_id,
        caller: user.clone(),
        name: None,
        description: None,
        category: None,
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: Some(Some(new_social_links.clone())),
    };

    client.update_project(&update_params);
    let updated_project = client.get_project(&project_id).unwrap();
    assert_eq!(updated_project.social_links, Some(new_social_links));

    // Remove social links
    let remove_params = ProjectUpdateParams {
        project_id,
        caller: user.clone(),
        name: None,
        description: None,
        category: None,
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: Some(None),
    };

    client.update_project(&remove_params);
    let final_project = client.get_project(&project_id).unwrap();
    assert_eq!(final_project.social_links, None);
}

#[test]
fn test_list_projects_by_tag() {
    let env = Env::default();
    env.mock_all_auths();
    
    let (client, _admin) = setup_contract(&env);
    let user = Address::generate(&env);

    // Create projects with different tags
    let mut defi_tags = Vec::new(&env);
    defi_tags.push_back(String::from_str(&env, "defi"));
    defi_tags.push_back(String::from_str(&env, "ethereum"));

    let mut gaming_tags = Vec::new(&env);
    gaming_tags.push_back(String::from_str(&env, "gaming"));
    gaming_tags.push_back(String::from_str(&env, "nft"));

    let mut mixed_tags = Vec::new(&env);
    mixed_tags.push_back(String::from_str(&env, "defi"));
    mixed_tags.push_back(String::from_str(&env, "gaming"));

    // Project 1: DeFi only
    let params1 = ProjectRegistrationParams {
        owner: user.clone(),
        name: String::from_str(&env, "DeFiProject"),
        description: String::from_str(&env, "A DeFi project"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: Some(defi_tags),
        social_links: None,
    };
    client.register_project(&params1);

    // Project 2: Gaming only
    let params2 = ProjectRegistrationParams {
        owner: user.clone(),
        name: String::from_str(&env, "GamingProject"),
        description: String::from_str(&env, "A gaming project"),
        category: String::from_str(&env, "Gaming"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: Some(gaming_tags),
        social_links: None,
    };
    client.register_project(&params2);

    // Project 3: Mixed tags
    let params3 = ProjectRegistrationParams {
        owner: user.clone(),
        name: String::from_str(&env, "MixedProject"),
        description: String::from_str(&env, "A mixed project"),
        category: String::from_str(&env, "Mixed"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: Some(mixed_tags),
        social_links: None,
    };
    client.register_project(&params3);

    // Search for DeFi projects
    let defi_projects = client.list_projects_by_tag(&String::from_str(&env, "defi"), &0, &10);
    assert_eq!(defi_projects.len(), 2); // Project 1 and 3

    // Search for gaming projects
    let gaming_projects = client.list_projects_by_tag(&String::from_str(&env, "gaming"), &0, &10);
    assert_eq!(gaming_projects.len(), 2); // Project 2 and 3

    // Search for non-existent tag
    let empty_projects = client.list_projects_by_tag(&String::from_str(&env, "nonexistent"), &0, &10);
    assert_eq!(empty_projects.len(), 0);
}

#[test]
fn test_tag_validation() {
    let env = Env::default();
    env.mock_all_auths();
    
    let (client, _admin) = setup_contract(&env);
    let user = Address::generate(&env);

    // Test too many tags
    let mut too_many_tags = Vec::new(&env);
    for i in 0..15 { // MAX_TAGS_PER_PROJECT is 10
        let tag_name = if i < 10 {
            String::from_str(&env, "tag0")
        } else {
            String::from_str(&env, "tag1")
        };
        too_many_tags.push_back(tag_name);
    }

    let params = ProjectRegistrationParams {
        owner: user.clone(),
        name: String::from_str(&env, "TooManyTagsProject"),
        description: String::from_str(&env, "A project with too many tags"),
        category: String::from_str(&env, "Test"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: Some(too_many_tags),
        social_links: None,
    };

    let result = client.try_register_project(&params);
    assert!(result.is_err());

    // Test tag too long
    let mut long_tag = Vec::new(&env);
    long_tag.push_back(String::from_str(&env, "this_tag_is_way_too_long_and_exceeds_the_maximum_length_allowed"));

    let params2 = ProjectRegistrationParams {
        owner: user.clone(),
        name: String::from_str(&env, "LongTagProject"),
        description: String::from_str(&env, "A project with a long tag"),
        category: String::from_str(&env, "Test"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: Some(long_tag),
        social_links: None,
    };

    let result = client.try_register_project(&params2);
    assert!(result.is_err());
}

#[test]
fn test_social_links_validation() {
    let env = Env::default();
    env.mock_all_auths();
    
    let (client, _admin) = setup_contract(&env);
    let user = Address::generate(&env);

    // Test too many social links
    let mut too_many_links = Map::new(&env);
    for i in 0..15 { // MAX_SOCIAL_LINKS is 10
        let platform_name = if i < 10 {
            String::from_str(&env, "platform0")
        } else {
            String::from_str(&env, "platform1")
        };
        let url = if i < 10 {
            String::from_str(&env, "https://example0.com")
        } else {
            String::from_str(&env, "https://example1.com")
        };
        too_many_links.set(platform_name, url);
    }

    let params = ProjectRegistrationParams {
        owner: user.clone(),
        name: String::from_str(&env, "TooManyLinksProject"),
        description: String::from_str(&env, "A project with too many social links"),
        category: String::from_str(&env, "Test"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: Some(too_many_links),
    };

    let result = client.try_register_project(&params);
    assert!(result.is_err());

    // Test invalid URL format
    let mut invalid_links = Map::new(&env);
    invalid_links.set(
        String::from_str(&env, "github"),
        String::from_str(&env, "not-a-valid-url"),
    );

    let params2 = ProjectRegistrationParams {
        owner: user.clone(),
        name: String::from_str(&env, "InvalidUrlProject"),
        description: String::from_str(&env, "A project with invalid URL"),
        category: String::from_str(&env, "Test"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: Some(invalid_links),
    };

    let result = client.try_register_project(&params2);
    assert!(result.is_err());
}