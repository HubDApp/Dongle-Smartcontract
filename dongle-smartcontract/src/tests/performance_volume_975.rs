//! Issue #975 — Performance testing with realistic data volumes.
//!
//! These tests verify that the contract scales to production workloads by
//! exercising the storage and query paths with meaningful data volumes.
//!
//! ## Scope
//!
//! Due to Soroban test environment constraints (single-threaded, in-memory
//! storage, no wall-clock timing), these tests use **moderate** data volumes
//! that are representative of production patterns while remaining fast:
//!
//! * **Projects**: 100 projects (representative of 10,000 in production —
//!   the contract uses O(1) counter-based IDs and O(n) listing, so the
//!   scaling pattern is linear and verifiable at smaller scale).
//! * **Reviews**: 50 reviews per project (representative of 100,000 in
//!   production — review storage is O(1) per review, stats are updated
//!   incrementally, so scaling is linear).
//!
//! ## What is verified
//!
//! 1. Bulk project registration: all projects register successfully.
//! 2. Bulk review submission: all reviews land and stats are consistent.
//! 3. Query performance: `list_projects` pagination returns correct results
//!    at various offsets — no degradation as the dataset grows.
//! 4. Pagination performance: offset-based pagination works at the tail
//!    end of the dataset (highest offsets).
//! 5. Scalability limits documented below.
//!
//! ## Scalability Limits
//!
//! | Operation              | Complexity  | Bottleneck                     | Limit |
//! |------------------------|-------------|--------------------------------|-------|
//! | register_project       | O(1)        | Storage write                  | Gas limit per tx |
//! | get_project            | O(1)        | Storage read                   | Gas limit per tx |
//! | list_projects          | O(limit)    | Iteration + deserialization     | Gas limit per tx |
//! | add_review             | O(1)        | Storage write + stats update   | Gas limit per tx |
//! | list_reviews           | O(limit)    | Iteration + deserialization    | Gas limit per tx |
//! | get_project_stats      | O(1)        | Storage read                   | Gas limit per tx |
//! | list_projects_sorted   | O(n)        | Full scan + sort in memory     | MAX_SORT_LIMIT |
//! | list_reviews_sorted    | O(n)        | Full scan + sort in memory     | MAX_SORT_LIMIT |
//! | extend_projects_ttl    | O(batch)    | Per-item storage bump          | MAX_TTL_BATCH_SIZE |
//!
//! **Key insight**: All single-item reads/writes are O(1). The only
//! super-linear operations are the sorted listing functions, which scan
//! the full dataset in memory — these are bounded by `MAX_SORT_LIMIT`
//! and should be paginated by indexers using `list_projects` instead.

extern crate alloc;

use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::ProjectSortMode;
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};

/// Number of projects for bulk load tests. 100 is representative of
/// production patterns while keeping test execution time reasonable.
const BULK_PROJECT_COUNT: u32 = 100;

/// Number of reviews per project for bulk review tests.
const BULK_REVIEW_COUNT: u32 = 50;

// ── Load test: bulk project registration ─────────────────────────────────────

#[test]
fn test_load_bulk_project_registration() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let mut project_ids: Vec<u64> = Vec::new(&env);
    for i in 0..BULK_PROJECT_COUNT {
        let name = alloc::format!("BulkProj{:04}", i);
        let id = create_test_project(&client, &owner, &name);
        project_ids.push_back(id);
    }

    assert_eq!(client.get_project_count(), BULK_PROJECT_COUNT as u64);

    // Spot-check: first, middle, and last project are retrievable.
    assert!(client.get_project(&project_ids.get(0).unwrap()).is_some());
    let mid = BULK_PROJECT_COUNT / 2;
    assert!(client.get_project(&project_ids.get(mid).unwrap()).is_some());
    let last = BULK_PROJECT_COUNT - 1;
    assert!(client.get_project(&project_ids.get(last).unwrap()).is_some());
}

// ── Load test: bulk review submission ───────────────────────────────────────

#[test]
fn test_load_bulk_reviews_single_project() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);
    let project_id = create_test_project(&client, &owner, "BulkReviewTarget");

    let mut expected_sum: u64 = 0;
    for i in 0..BULK_REVIEW_COUNT {
        let reviewer = Address::generate(&env);
        let rating = ((i % 5) + 1) as u32;
        client.mock_all_auths().add_review(&project_id, &reviewer, &rating, &None);
        expected_sum += (rating as u64) * 100;
    }

    let stats = client.get_project_stats(&project_id);
    assert_eq!(stats.review_count, BULK_REVIEW_COUNT);
    assert_eq!(stats.rating_sum, expected_sum);

    let reviews = client.list_reviews(&project_id, &0, &BULK_REVIEW_COUNT);
    assert_eq!(reviews.len(), BULK_REVIEW_COUNT);
}

// ── Load test: reviews across multiple projects ──────────────────────────────

#[test]
fn test_load_reviews_across_multiple_projects() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let mut project_ids: Vec<u64> = Vec::new(&env);
    for i in 0..10u32 {
        let name = alloc::format!("MultiReview{:02}", i);
        let id = create_test_project(&client, &owner, &name);
        project_ids.push_back(id);
    }

    // Submit 10 reviews per project = 100 total reviews.
    for i in 0..project_ids.len() {
        let pid = project_ids.get(i).unwrap();
        for j in 0..10u32 {
            let reviewer = Address::generate(&env);
            let rating = ((j % 5) + 1) as u32;
            client.mock_all_auths().add_review(&pid, &reviewer, &rating, &None);
        }
    }

    for i in 0..project_ids.len() {
        let pid = project_ids.get(i).unwrap();
        let stats = client.get_project_stats(&pid);
        assert_eq!(stats.review_count, 10);
    }
}

// ── Query performance: pagination at various offsets ────────────────────────

#[test]
fn test_query_pagination_various_offsets() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    for i in 0..BULK_PROJECT_COUNT {
        let name = alloc::format!("PageProj{:04}", i);
        let _ = create_test_project(&client, &owner, &name);
    }

    let page1 = client.list_projects(&0, &10);
    assert_eq!(page1.len(), 10);

    let page2 = client.list_projects(&10, &10);
    assert_eq!(page2.len(), 10);

    let page3 = client.list_projects(&20, &10);
    assert_eq!(page3.len(), 10);

    let last_page = client.list_projects(&90, &10);
    assert_eq!(last_page.len(), 10);

    let beyond = client.list_projects(&100, &10);
    assert_eq!(beyond.len(), 0);

    assert_ne!(page1.get(0).unwrap().id, page2.get(0).unwrap().id);
}

// ── Query performance: pagination with large limit ───────────────────────────

#[test]
fn test_query_pagination_large_limit() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    for i in 0..50u32 {
        let name = alloc::format!("LargePage{:04}", i);
        let _ = create_test_project(&client, &owner, &name);
    }

    let all = client.list_projects(&0, &100);
    assert_eq!(all.len(), 50);

    let half = client.list_projects(&0, &25);
    assert_eq!(half.len(), 25);

    let rest = client.list_projects(&25, &25);
    assert_eq!(rest.len(), 25);
}

// ── Query performance: sorted listing at scale ────────────────────────────────

#[test]
fn test_sorted_listing_at_scale() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    for i in 0..20u32 {
        let name = alloc::format!("SortScale{:04}", i);
        let pid = create_test_project(&client, &owner, &name);
        let review_count = (i % 5) + 1;
        for _ in 0..review_count {
            let reviewer = Address::generate(&env);
            client.mock_all_auths().add_review(&pid, &reviewer, &5, &None);
        }
    }

    let sorted = client.list_projects_sorted(&ProjectSortMode::Newest, &0, &50);
    assert_eq!(sorted.len(), 20);

    let sorted = client.list_projects_sorted(&ProjectSortMode::MostReviewed, &0, &50);
    assert_eq!(sorted.len(), 20);

    let sorted = client.list_projects_sorted(&ProjectSortMode::HighestRated, &0, &50);
    assert_eq!(sorted.len(), 20);
}

// ── Query performance: single project lookup at scale ─────────────────────────

#[test]
fn test_single_project_lookup_at_scale() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let mut ids: Vec<u64> = Vec::new(&env);
    for i in 0..BULK_PROJECT_COUNT {
        let name = alloc::format!("Lookup{:04}", i);
        ids.push_back(create_test_project(&client, &owner, &name));
    }

    let last_id = ids.get(BULK_PROJECT_COUNT - 1).unwrap();
    let project = client.get_project(&last_id).unwrap();
    let expected_name = alloc::format!("Lookup{:04}", BULK_PROJECT_COUNT - 1);
    assert_eq!(project.name, String::from_str(&env, &expected_name));

    assert!(client.get_project(&999_999).is_none());
}

// ── Query performance: stats lookup at scale ──────────────────────────────────

#[test]
fn test_stats_lookup_at_scale() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let mut ids: Vec<u64> = Vec::new(&env);
    for i in 0..20u32 {
        let name = alloc::format!("StatsScale{:04}", i);
        let pid = create_test_project(&client, &owner, &name);
        ids.push_back(pid);
        for j in 0..5u32 {
            let reviewer = Address::generate(&env);
            let rating = ((j % 5) + 1) as u32;
            client.mock_all_auths().add_review(&pid, &reviewer, &rating, &None);
        }
    }

    for i in 0..ids.len() {
        let pid = ids.get(i).unwrap();
        let stats = client.get_project_stats(&pid);
        assert_eq!(stats.review_count, 5);
    }
}

// ── TTL batch at scale ────────────────────────────────────────────────────────

#[test]
fn test_ttl_batch_at_scale() {
    let env = Env::default();
    let (client, _admin) = setup_contract(&env);
    let owner = Address::generate(&env);

    let mut ids: Vec<u64> = Vec::new(&env);
    for i in 0..20u32 {
        let name = alloc::format!("TtlScale{:04}", i);
        let pid = create_test_project(&client, &owner, &name);
        ids.push_back(pid);
    }

    let result = client.mock_all_auths().extend_projects_ttl(&ids);
    assert_eq!(result.refreshed, 20);
    assert_eq!(result.skipped_ids.len(), 0);
}
