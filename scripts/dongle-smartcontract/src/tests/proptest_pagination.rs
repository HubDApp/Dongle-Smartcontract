//! Property-based tests for pagination endpoints using proptest.
//!
//! For any valid number of registered items N, paginating through all pages
//! must:
//!   1. Return no duplicate records across pages.
//!   2. Collectively reconstruct all N registered records.

use crate::tests::fixtures::{create_test_project, setup_contract};
use proptest::prelude::*;
use soroban_sdk::{testutils::Address as _, Address, Env};

// Static name/slug pairs used to register up to 15 projects in a single test
// run without requiring heap allocation for formatting.
static PROJECT_NAMES: &[&str] = &[
    "PropA", "PropB", "PropC", "PropD", "PropE", "PropF", "PropG", "PropH", "PropI", "PropJ",
    "PropK", "PropL", "PropM", "PropN", "PropO",
];

// ── Project pagination ────────────────────────────────────────────────────────

proptest! {
    /// Paginating through all pages yields every registered project exactly once.
    #[test]
    fn prop_project_pagination_covers_all_records(
        n in 1u32..=10u32,
        page_size in 1u32..=5u32,
    ) {
        let env = Env::default();
        let (client, _) = setup_contract(&env);
        let owner = Address::generate(&env);

        for i in 0..n {
            create_test_project(&client, &owner, PROJECT_NAMES[i as usize]);
        }

        // Walk all pages and accumulate IDs.
        let mut collected: soroban_sdk::Vec<u64> = soroban_sdk::Vec::new(&env);
        let mut start: u64 = 1;
        loop {
            let page = client.list_projects(&start, &page_size);
            let page_len = page.len();
            for p in page.iter() {
                collected.push_back(p.id);
            }
            if page_len < page_size {
                break;
            }
            start += page_size as u64;
        }

        // All N projects must appear.
        prop_assert_eq!(
            collected.len(),
            n,
            "all {} projects must be reachable by walking pages of size {}",
            n,
            page_size
        );
    }

    /// No project ID appears in more than one page.
    #[test]
    fn prop_project_pagination_no_duplicates(
        n in 2u32..=10u32,
        page_size in 1u32..=4u32,
    ) {
        let env = Env::default();
        let (client, _) = setup_contract(&env);
        let owner = Address::generate(&env);

        for i in 0..n {
            create_test_project(&client, &owner, PROJECT_NAMES[i as usize]);
        }

        let mut collected: soroban_sdk::Vec<u64> = soroban_sdk::Vec::new(&env);
        let mut start: u64 = 1;
        loop {
            let page = client.list_projects(&start, &page_size);
            let page_len = page.len();
            for p in page.iter() {
                collected.push_back(p.id);
            }
            if page_len < page_size {
                break;
            }
            start += page_size as u64;
        }

        // Every expected sequential ID must appear exactly once.
        for expected_id in 1u64..=(n as u64) {
            let mut count = 0u32;
            for id in collected.iter() {
                if id == expected_id {
                    count += 1;
                }
            }
            prop_assert_eq!(
                count,
                1u32,
                "project ID {} must appear exactly once across pages (found {} times)",
                expected_id,
                count
            );
        }
    }
}

// ── Review pagination ─────────────────────────────────────────────────────────

proptest! {
    /// Paginating through all review pages yields every review exactly once.
    #[test]
    fn prop_review_pagination_covers_all_records(
        n in 1u32..=8u32,
        page_size in 1u32..=4u32,
    ) {
        let env = Env::default();
        let (client, _) = setup_contract(&env);
        let owner = Address::generate(&env);
        let project_id = create_test_project(&client, &owner, "ReviewPropProject");

        let mut reviewers: soroban_sdk::Vec<Address> = soroban_sdk::Vec::new(&env);
        for _ in 0..n {
            let reviewer = Address::generate(&env);
            client
                .mock_all_auths()
                .add_review(&project_id, &reviewer, &4, &None);
            reviewers.push_back(reviewer);
        }

        // Walk all pages and count collected reviews.
        let mut total = 0u32;
        let mut start = 0u32;
        loop {
            let page = client.list_reviews(&project_id, &start, &page_size);
            let page_len = page.len();
            total += page_len;
            if page_len < page_size {
                break;
            }
            start += page_size;
        }

        prop_assert_eq!(
            total,
            n,
            "all {} reviews must be reachable by walking pages of size {}",
            n,
            page_size
        );
    }

    /// No reviewer address appears in more than one review page.
    #[test]
    fn prop_review_pagination_no_duplicates(
        n in 2u32..=8u32,
        page_size in 1u32..=4u32,
    ) {
        let env = Env::default();
        let (client, _) = setup_contract(&env);
        let owner = Address::generate(&env);
        let project_id = create_test_project(&client, &owner, "ReviewNoDupProject");

        let mut reviewers: soroban_sdk::Vec<Address> = soroban_sdk::Vec::new(&env);
        for _ in 0..n {
            let reviewer = Address::generate(&env);
            client
                .mock_all_auths()
                .add_review(&project_id, &reviewer, &3, &None);
            reviewers.push_back(reviewer.clone());
        }

        // Collect all reviewer addresses across pages.
        let mut collected: soroban_sdk::Vec<Address> = soroban_sdk::Vec::new(&env);
        let mut start = 0u32;
        loop {
            let page = client.list_reviews(&project_id, &start, &page_size);
            let page_len = page.len();
            for r in page.iter() {
                collected.push_back(r.reviewer.clone());
            }
            if page_len < page_size {
                break;
            }
            start += page_size;
        }

        prop_assert_eq!(
            collected.len(),
            n,
            "must collect exactly {} reviewer entries across all pages",
            n
        );

        // Each reviewer from the registered set must appear exactly once.
        for reviewer in reviewers.iter() {
            let mut count = 0u32;
            for collected_reviewer in collected.iter() {
                if collected_reviewer == reviewer {
                    count += 1;
                }
            }
            prop_assert_eq!(
                count,
                1u32,
                "each reviewer must appear exactly once across all pages"
            );
        }
    }
}
