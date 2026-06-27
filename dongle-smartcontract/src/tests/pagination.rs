//! Tests that validate pagination behavior for list endpoints.
//!
//! Covers: limit enforcement, input validation, deterministic ordering,
//! edge cases (empty, out-of-range, zero limit, over-max limit).

use crate::types::ProjectRegistrationParams;
use crate::DongleContract;
use crate::DongleContractClient;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

fn setup(env: &Env) -> (DongleContractClient<'_>, Address) {
    let contract_id = env.register(DongleContract, ());
    let client = DongleContractClient::new(env, &contract_id);
    let admin = Address::generate(env);
    client.mock_all_auths().initialize(&admin);
    (client, admin)
}

fn register_project(client: &DongleContractClient, owner: &Address, name: &str) -> u64 {
    let env = &client.env;
    let slug = name.to_lowercase().replace(' ', "-");
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(env, name),
        slug: String::from_str(env, &slug),
        description: String::from_str(env, "Description"),
        category: String::from_str(env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
    };
    client.mock_all_auths().register_project(&params)
}

// ── list_projects pagination ──────────────────────────────────────────────────

#[test]
fn test_list_projects_empty() {
    let env = Env::default();
    let (client, _) = setup(&env);

    let result = client.list_projects(&1, &10);
    assert_eq!(result.len(), 0);
}

#[test]
fn test_list_projects_basic_pagination() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let owner = Address::generate(&env);

    register_project(&client, &owner, "Alpha");
    register_project(&client, &owner, "Beta");
    register_project(&client, &owner, "Gamma");

    // First page: start=1, limit=2 → IDs 1,2
    let page1 = client.list_projects(&1, &2);
    assert_eq!(page1.len(), 2);
    assert_eq!(page1.get(0).unwrap().id, 1);
    assert_eq!(page1.get(1).unwrap().id, 2);

    // Second page: start=3, limit=2 → ID 3
    let page2 = client.list_projects(&3, &2);
    assert_eq!(page2.len(), 1);
    assert_eq!(page2.get(0).unwrap().id, 3);
}

#[test]
fn test_list_projects_deterministic_ordering() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let owner = Address::generate(&env);

    register_project(&client, &owner, "First");
    register_project(&client, &owner, "Second");
    register_project(&client, &owner, "Third");

    let result = client.list_projects(&1, &10);
    assert_eq!(result.len(), 3);

    // IDs must be in ascending order
    for i in 0..result.len() - 1 {
        assert!(
            result.get(i).unwrap().id < result.get(i + 1).unwrap().id,
            "list_projects must return projects in ascending ID order"
        );
    }
}

#[test]
fn test_list_projects_start_id_zero_treated_as_one() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let owner = Address::generate(&env);

    register_project(&client, &owner, "ZeroStart");

    // start_id=0 should be treated as 1 (first valid project ID)
    let result = client.list_projects(&0, &10);
    assert_eq!(result.len(), 1);
    assert_eq!(result.get(0).unwrap().id, 1);
}

#[test]
fn test_list_projects_start_id_beyond_count_returns_empty() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let owner = Address::generate(&env);

    register_project(&client, &owner, "OnlyOne");

    let result = client.list_projects(&100, &10);
    assert_eq!(result.len(), 0);
}

#[test]
fn test_list_projects_limit_zero_uses_max() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let owner = Address::generate(&env);

    for i in 0..5u32 {
        let name = match i {
            0 => "Proj0",
            1 => "Proj1",
            2 => "Proj2",
            3 => "Proj3",
            _ => "Proj4",
        };
        register_project(&client, &owner, name);
    }

    // limit=0 should fall back to MAX_PAGE_LIMIT (100), returning all 5
    let result = client.list_projects(&1, &0);
    assert_eq!(
        result.len(),
        5,
        "limit=0 should use max limit and return all projects"
    );
}

#[test]
fn test_list_projects_limit_over_max_clamped() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let owner = Address::generate(&env);

    for i in 0..5u32 {
        let name = match i {
            0 => "LimA",
            1 => "LimB",
            2 => "LimC",
            3 => "LimD",
            _ => "LimE",
        };
        register_project(&client, &owner, name);
    }

    // limit=9999 should be clamped to MAX_PAGE_LIMIT (100), returning all 5
    let result = client.list_projects(&1, &9999);
    assert_eq!(
        result.len(),
        5,
        "limit over max should be clamped to MAX_PAGE_LIMIT"
    );
}

#[test]
fn test_list_projects_exact_limit() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let owner = Address::generate(&env);

    register_project(&client, &owner, "ExactA");
    register_project(&client, &owner, "ExactB");
    register_project(&client, &owner, "ExactC");

    let result = client.list_projects(&1, &3);
    assert_eq!(result.len(), 3);
}

#[test]
fn test_list_projects_limit_larger_than_available() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let owner = Address::generate(&env);

    register_project(&client, &owner, "OnlyProject");

    // Requesting 50 but only 1 exists
    let result = client.list_projects(&1, &50);
    assert_eq!(result.len(), 1);
}

// ── list_reviews pagination ───────────────────────────────────────────────────

#[test]
fn test_list_reviews_empty_project() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let owner = Address::generate(&env);

    let project_id = register_project(&client, &owner, "NoReviews");

    let result = client.list_reviews(&project_id, &0, &10);
    assert_eq!(result.len(), 0);
}

#[test]
fn test_list_reviews_basic_pagination() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let owner = Address::generate(&env);

    let project_id = register_project(&client, &owner, "ReviewPaged");

    let reviewer1 = Address::generate(&env);
    let reviewer2 = Address::generate(&env);
    let reviewer3 = Address::generate(&env);

    client
        .mock_all_auths()
        .add_review(&project_id, &reviewer1, &5, &None);
    client
        .mock_all_auths()
        .add_review(&project_id, &reviewer2, &4, &None);
    client
        .mock_all_auths()
        .add_review(&project_id, &reviewer3, &3, &None);

    // First page: start=0, limit=2
    let page1 = client.list_reviews(&project_id, &0, &2);
    assert_eq!(page1.len(), 2);

    // Second page: start=2, limit=2
    let page2 = client.list_reviews(&project_id, &2, &2);
    assert_eq!(page2.len(), 1);
}

#[test]
fn test_list_reviews_deterministic_ordering() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let owner = Address::generate(&env);

    let project_id = register_project(&client, &owner, "OrderedReviews");

    let reviewer1 = Address::generate(&env);
    let reviewer2 = Address::generate(&env);
    let reviewer3 = Address::generate(&env);

    client
        .mock_all_auths()
        .add_review(&project_id, &reviewer1, &1, &None);
    client
        .mock_all_auths()
        .add_review(&project_id, &reviewer2, &2, &None);
    client
        .mock_all_auths()
        .add_review(&project_id, &reviewer3, &3, &None);

    let page1 = client.list_reviews(&project_id, &0, &10);
    let page2 = client.list_reviews(&project_id, &0, &10);

    // Same call must return same results (deterministic)
    assert_eq!(page1.len(), page2.len());
    for i in 0..page1.len() {
        assert_eq!(
            page1.get(i).unwrap().reviewer,
            page2.get(i).unwrap().reviewer,
            "list_reviews must be deterministic"
        );
    }
}

#[test]
fn test_list_reviews_start_beyond_count_returns_empty() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let owner = Address::generate(&env);

    let project_id = register_project(&client, &owner, "FewReviews");
    let reviewer = Address::generate(&env);

    client
        .mock_all_auths()
        .add_review(&project_id, &reviewer, &5, &None);

    // start=100 is beyond the single review
    let result = client.list_reviews(&project_id, &100, &10);
    assert_eq!(result.len(), 0);
}

#[test]
fn test_list_reviews_limit_zero_uses_max() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let owner = Address::generate(&env);

    let project_id = register_project(&client, &owner, "LimitZeroReviews");

    for _ in 0..3u32 {
        let reviewer = Address::generate(&env);
        client
            .mock_all_auths()
            .add_review(&project_id, &reviewer, &4, &None);
    }

    // limit=0 should fall back to MAX_PAGE_LIMIT (100), returning all 3
    let result = client.list_reviews(&project_id, &0, &0);
    assert_eq!(result.len(), 3, "limit=0 should use max limit");
}

#[test]
fn test_list_reviews_limit_over_max_clamped() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let owner = Address::generate(&env);

    let project_id = register_project(&client, &owner, "LimitOverMax");

    for _ in 0..3u32 {
        let reviewer = Address::generate(&env);
        client
            .mock_all_auths()
            .add_review(&project_id, &reviewer, &3, &None);
    }

    // limit=9999 should be clamped to MAX_PAGE_LIMIT (100), returning all 3
    let result = client.list_reviews(&project_id, &0, &9999);
    assert_eq!(result.len(), 3, "limit over max should be clamped");
}

#[test]
fn test_list_reviews_no_overlap_between_pages() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let owner = Address::generate(&env);

    let project_id = register_project(&client, &owner, "NoOverlap");

    let mut reviewers = soroban_sdk::Vec::new(&env);
    for _ in 0..4u32 {
        let reviewer = Address::generate(&env);
        client
            .mock_all_auths()
            .add_review(&project_id, &reviewer, &5, &None);
        reviewers.push_back(reviewer);
    }

    let page1 = client.list_reviews(&project_id, &0, &2);
    let page2 = client.list_reviews(&project_id, &2, &2);

    assert_eq!(page1.len(), 2);
    assert_eq!(page2.len(), 2);

    // No reviewer should appear in both pages
    for r1 in page1.iter() {
        for r2 in page2.iter() {
            assert_ne!(
                r1.reviewer, r2.reviewer,
                "same reviewer appeared in two different pages"
            );
        }
    }
}

#[test]
fn test_list_reviews_all_pages_cover_all_reviews() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let owner = Address::generate(&env);

    let project_id = register_project(&client, &owner, "AllPages");

    for _ in 0..5u32 {
        let reviewer = Address::generate(&env);
        client
            .mock_all_auths()
            .add_review(&project_id, &reviewer, &4, &None);
    }

    // Collect all reviews via pagination
    let mut all_reviews = soroban_sdk::Vec::new(&env);
    let mut start = 0u32;
    let page_size = 2u32;
    loop {
        let page = client.list_reviews(&project_id, &start, &page_size);
        let page_len = page.len();
        for r in page.iter() {
            all_reviews.push_back(r);
        }
        if page_len < page_size {
            break;
        }
        start += page_size;
    }

    assert_eq!(
        all_reviews.len(),
        5,
        "paginating through all pages should yield all 5 reviews"
    );
}

// ── list_projects_by_category pagination ────────────────────────────────────────

#[test]
fn test_list_projects_by_category_empty() {
    let env = Env::default();
    let (client, _) = setup(&env);

    let result = client.list_projects_by_category(&String::from_str(&env, "NonExistent"), &0, &10);
    assert_eq!(result.len(), 0);
}

#[test]
fn test_list_projects_by_category_basic() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let owner = Address::generate(&env);

    let params1 = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "ProjCat1"),
        slug: String::from_str(&env, "projcat1"),
        description: String::from_str(&env, "Description"),
        category: String::from_str(&env, "Web3"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
    };
    client.mock_all_auths().register_project(&params1);

    let params2 = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "ProjCat2"),
        slug: String::from_str(&env, "projcat2"),
        description: String::from_str(&env, "Description"),
        category: String::from_str(&env, "DeFi"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
    };
    client.mock_all_auths().register_project(&params2);

    let result_web3 = client.list_projects_by_category(&String::from_str(&env, "Web3"), &0, &10);
    assert_eq!(result_web3.len(), 1);
    assert_eq!(
        result_web3.get(0).unwrap().name,
        String::from_str(&env, "ProjCat1")
    );

    let result_defi = client.list_projects_by_category(&String::from_str(&env, "DeFi"), &0, &10);
    assert_eq!(result_defi.len(), 1);
    assert_eq!(
        result_defi.get(0).unwrap().name,
        String::from_str(&env, "ProjCat2")
    );
}

#[test]
fn test_list_projects_by_category_update_moves_project() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let owner = Address::generate(&env);

    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(&env, "ProjMove"),
        slug: String::from_str(&env, "projmove"),
        description: String::from_str(&env, "Description"),
        category: String::from_str(&env, "OldCat"),
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
    };
    let project_id = client.mock_all_auths().register_project(&params);

    // Verify it's in OldCat
    let result_old = client.list_projects_by_category(&String::from_str(&env, "OldCat"), &0, &10);
    assert_eq!(result_old.len(), 1);

    // Update to NewCat
    use crate::types::ProjectUpdateParams;
    let update_params = ProjectUpdateParams {
        project_id,
        caller: owner.clone(),
        name: None,
        slug: None,
        description: None,
        category: Some(String::from_str(&env, "NewCat")),
        website: None,
        logo_cid: None,
        metadata_cid: None,
        tags: None,
        social_links: None,
        launch_timestamp: None,
        bounty_url: None,
    };
    client.mock_all_auths().update_project(&update_params);

    // Verify it's removed from OldCat and added to NewCat
    let result_old_after =
        client.list_projects_by_category(&String::from_str(&env, "OldCat"), &0, &10);
    assert_eq!(result_old_after.len(), 0);

    let result_new = client.list_projects_by_category(&String::from_str(&env, "NewCat"), &0, &10);
    assert_eq!(result_new.len(), 1);
    assert_eq!(result_new.get(0).unwrap().id, project_id);
}

#[test]
fn test_list_projects_by_category_pagination() {
    let env = Env::default();
    let (client, _) = setup(&env);
    let owner = Address::generate(&env);

    for i in 0..5 {
        let name_str = match i {
            0 => "ProjMulti0",
            1 => "ProjMulti1",
            2 => "ProjMulti2",
            3 => "ProjMulti3",
            _ => "ProjMulti4",
        };
        let params = ProjectRegistrationParams {
            owner: owner.clone(),
            name: String::from_str(&env, name_str),
            slug: String::from_str(&env, &name_str.to_lowercase()),
            description: String::from_str(&env, "Description"),
            category: String::from_str(&env, "MultiCat"),
            website: None,
            logo_cid: None,
            metadata_cid: None,
            tags: None,
            social_links: None,
            launch_timestamp: None,
            bounty_url: None,
        };
        client.mock_all_auths().register_project(&params);
    }

    let page1 = client.list_projects_by_category(&String::from_str(&env, "MultiCat"), &0, &2);
    assert_eq!(page1.len(), 2);

    let page2 = client.list_projects_by_category(&String::from_str(&env, "MultiCat"), &2, &5);
    assert_eq!(page2.len(), 3);
}
