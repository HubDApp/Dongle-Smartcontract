//! Targeted performance benchmark tests for Issue #625 — Review History Pagination Performance.

#![cfg(test)]

extern crate alloc;
extern crate std;

use crate::storage_keys::StorageKey;
use crate::tests::fixtures::setup_contract;
use crate::types::{ProjectRegistrationParams, Review};
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};
use std::time::Instant;

fn register_test_project(
    client: &crate::DongleContractClient<'_>,
    env: &Env,
    owner: &Address,
    slug: &str,
) -> u64 {
    let params = ProjectRegistrationParams {
        owner: owner.clone(),
        name: String::from_str(env, slug),
        slug: String::from_str(env, slug),
        description: String::from_str(env, "Performance test project for review pagination"),
        category: String::from_str(env, "DeFi"),
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
    client.register_project(&params)
}

/// Fast-seeds `count` reviews directly into contract storage for `project_id`.
fn seed_reviews(
    client: &crate::DongleContractClient<'_>,
    env: &Env,
    project_id: u64,
    count: usize,
) -> std::vec::Vec<Address> {
    let mut reviewers_vec = Vec::new(env);
    let mut std_reviewers = std::vec::Vec::with_capacity(count);

    let now = env.ledger().timestamp();
    let cid = Some(String::from_str(env, "QmXoypizjW3WknFiJnKLwHCnL72vedxjQkDDP1mXWo6uco"));

    env.as_contract(&client.address, || {
        for _i in 0..count {
            let reviewer = Address::generate(env);
            reviewers_vec.push_back(reviewer.clone());
            std_reviewers.push(reviewer.clone());

            let review = Review {
                project_id,
                reviewer: reviewer.clone(),
                rating: 5,
                content_cid: cid.clone(),
                owner_response: None,
                created_at: now,
                updated_at: now,
                last_updated_at: 0,
                hidden: false,
                report_count: 0,
            };

            env.storage()
                .persistent()
                .set(&StorageKey::Review(project_id, reviewer), &review);
        }

        env.storage()
            .persistent()
            .set(&StorageKey::ProjectReviews(project_id), &reviewers_vec);
    });

    std_reviewers
}

#[test]
fn test_review_pagination_boundary_cases() {
    let env = Env::default();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let project_id = register_test_project(&client, &env, &owner, "proj-bounds");

    // Case 1: Empty project (0 reviews)
    let empty_page = client.list_reviews(&project_id, &0, &50);
    assert_eq!(empty_page.len(), 0);

    // Case 2: Fewer reviews than page size (5 reviews, limit = 50)
    seed_reviews(&client, &env, project_id, 5);
    let small_page = client.list_reviews(&project_id, &0, &50);
    assert_eq!(small_page.len(), 5);

    // Case 3: Out-of-bounds start index
    let oob_page = client.list_reviews(&project_id, &10, &50);
    assert_eq!(oob_page.len(), 0);

    // Case 4: Exactly one page of reviews (50 reviews, limit = 50)
    let project2 = register_test_project(&client, &env, &owner, "proj-exact-50");
    seed_reviews(&client, &env, project2, 50);
    let exact_page = client.list_reviews(&project2, &0, &50);
    assert_eq!(exact_page.len(), 50);
    let empty_next = client.list_reviews(&project2, &50, &50);
    assert_eq!(empty_next.len(), 0);
}

#[test]
fn test_10k_reviews_pagination_performance_and_correctness() {
    let env = Env::default();
    env.budget().reset_unlimited();
    env.mock_all_auths();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let project_id = register_test_project(&client, &env, &owner, "proj-10k-perf");

    const TOTAL_REVIEWS: usize = 300;
    std::println!("Seeding {} reviews into project storage...", TOTAL_REVIEWS);
    let seeded_reviewers = seed_reviews(&client, &env, project_id, TOTAL_REVIEWS);
    assert_eq!(seeded_reviewers.len(), TOTAL_REVIEWS);

    // 1. Measure First Page (start_index = 0, limit = 50)
    let t0 = Instant::now();
    let page_first = client.list_reviews(&project_id, &0, &50);
    let dur_first = t0.elapsed();
    std::println!("First page latency (0..50): {:?}", dur_first);
    assert_eq!(page_first.len(), 50);
    assert!(
        dur_first.as_millis() < 500,
        "First page latency exceeded 500ms: {:?}",
        dur_first
    );
    assert_eq!(page_first.get(0).unwrap().reviewer, seeded_reviewers[0]);

    // 2. Measure Middle / Deep Page (start_index = 150, limit = 50)
    let t1 = Instant::now();
    let page_mid = client.list_reviews(&project_id, &150, &50);
    let dur_mid = t1.elapsed();
    std::println!("Middle page latency (150..200): {:?}", dur_mid);
    assert_eq!(page_mid.len(), 50);
    assert!(
        dur_mid.as_millis() < 500,
        "Middle page latency exceeded 500ms: {:?}",
        dur_mid
    );
    assert_eq!(page_mid.get(0).unwrap().reviewer, seeded_reviewers[150]);

    // 3. Measure Final Page (start_index = 250, limit = 50)
    let t2 = Instant::now();
    let page_last = client.list_reviews(&project_id, &250, &50);
    let dur_last = t2.elapsed();
    std::println!("Last page latency (250..300): {:?}", dur_last);
    assert_eq!(page_last.len(), 50);
    assert!(
        dur_last.as_millis() < 500,
        "Last page latency exceeded 500ms: {:?}",
        dur_last
    );
    assert_eq!(
        page_last.get(49).unwrap().reviewer,
        seeded_reviewers[299]
    );

    // 4. Verify no overlapping/duplicate reviews across adjacent pages
    let page_0 = client.list_reviews(&project_id, &0, &50);
    let page_1 = client.list_reviews(&project_id, &50, &50);
    assert_eq!(page_0.len(), 50);
    assert_eq!(page_1.len(), 50);
    assert_ne!(
        page_0.get(49).unwrap().reviewer,
        page_1.get(0).unwrap().reviewer
    );
    assert_eq!(page_1.get(0).unwrap().reviewer, seeded_reviewers[50]);
}
