//! Property-based tests for contract invariants (issue #981).
//!
//! Random sequences of valid operations are applied to a fresh contract and
//! the following properties are checked after **every** operation:
//!
//! 1. **Project count** — `get_project_count` equals the number of successful
//!    registrations and every id in `1..=count` resolves to a project.
//! 2. **Stats consistency** — for each project, `review_count` equals the number
//!    of live reviews and `rating_sum` equals the sum of their ratings ×100.
//! 3. **Average bound** — `average_rating` equals `rating_sum / review_count`
//!    (0 when there are no reviews) and never exceeds 500.
//! 4. **Soft-delete window** — a project has `deleted_at` set iff it is listed
//!    by `get_deleted_projects`, and then `recoverable_until > deleted_at`.

use crate::tests::fixtures::{create_test_project, setup_contract};
use proptest::prelude::*;
use soroban_sdk::{testutils::Address as _, Address, Env};
use std::collections::BTreeMap;
use std::vec::Vec as StdVec;

#[derive(Clone, Debug)]
enum Op {
    Register,
    Review {
        project: usize,
        reviewer: usize,
        rating: u32,
    },
    DeleteReview {
        project: usize,
        reviewer: usize,
    },
    SoftDelete {
        project: usize,
    },
    Recover {
        project: usize,
    },
}

fn op_strategy() -> impl Strategy<Value = Op> {
    prop_oneof![
        Just(Op::Register),
        (0usize..4, 0usize..4, 1u32..=5).prop_map(|(project, reviewer, rating)| Op::Review {
            project,
            reviewer,
            rating
        }),
        (0usize..4, 0usize..4)
            .prop_map(|(project, reviewer)| Op::DeleteReview { project, reviewer }),
        (0usize..4).prop_map(|project| Op::SoftDelete { project }),
        (0usize..4).prop_map(|project| Op::Recover { project }),
    ]
}

/// Off-chain model: project id -> (reviewer index -> rating), deleted flag.
#[derive(Default)]
struct Model {
    projects: StdVec<u64>,
    reviews: BTreeMap<u64, BTreeMap<usize, u32>>,
    deleted: BTreeMap<u64, bool>,
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(24))]

    #[test]
    fn invariants_hold_after_every_operation(ops in prop::collection::vec(op_strategy(), 1..20)) {
        let env = Env::default();
        let (client, admin) = setup_contract(&env);
        let owner = Address::generate(&env);
        let reviewers: StdVec<Address> = (0..4).map(|_| Address::generate(&env)).collect();
        let mut model = Model::default();

        for op in ops {
            match op {
                Op::Register => {
                    let name = std::format!("Prop Project {}", model.projects.len());
                    let id = create_test_project(&client, &owner, &name);
                    model.projects.push(id);
                    model.reviews.insert(id, BTreeMap::new());
                    model.deleted.insert(id, false);
                }
                Op::Review { project, reviewer, rating } => {
                    if let Some(&id) = model.projects.get(project) {
                        let res = client.mock_all_auths().try_add_review(
                            &id, &reviewers[reviewer], &rating, &None,
                        );
                        if res.is_ok() {
                            model.reviews.get_mut(&id).unwrap().insert(reviewer, rating);
                        }
                    }
                }
                Op::DeleteReview { project, reviewer } => {
                    if let Some(&id) = model.projects.get(project) {
                        let res = client
                            .mock_all_auths()
                            .try_delete_review(&id, &reviewers[reviewer]);
                        if res.is_ok() {
                            model.reviews.get_mut(&id).unwrap().remove(&reviewer);
                        }
                    }
                }
                Op::SoftDelete { project } => {
                    if let Some(&id) = model.projects.get(project) {
                        let res = client.mock_all_auths().try_delete_project(&id, &owner);
                        prop_assert_eq!(res.is_ok(), !model.deleted[&id]);
                        model.deleted.insert(id, true);
                    }
                }
                Op::Recover { project } => {
                    if let Some(&id) = model.projects.get(project) {
                        let res = client
                            .mock_all_auths()
                            .try_recover_deleted_project(&id, &owner);
                        prop_assert_eq!(res.is_ok(), model.deleted[&id]);
                        model.deleted.insert(id, false);
                    }
                }
            }

            // Invariant 1: project count.
            prop_assert_eq!(client.get_project_count(), model.projects.len() as u64);
            let deleted_ids: StdVec<u64> = client
                .get_deleted_projects(&admin)
                .iter()
                .map(|p| p.id)
                .collect();

            for &id in &model.projects {
                let project = client.get_project(&id);
                prop_assert!(project.is_some());
                let project = project.unwrap();

                // Invariants 2 & 3: stats consistency and average bound.
                let live = &model.reviews[&id];
                let stats = client.get_project_stats(&id);
                let sum: u64 = live.values().map(|&r| r as u64 * 100).sum();
                prop_assert_eq!(stats.review_count, live.len() as u32);
                prop_assert_eq!(stats.rating_sum, sum);
                let expected_avg = if live.is_empty() { 0 } else { (sum / live.len() as u64) as u32 };
                prop_assert_eq!(stats.average_rating, expected_avg);
                prop_assert!(stats.average_rating <= 500);

                // Invariant 4: soft-delete bookkeeping.
                let is_deleted = model.deleted[&id];
                prop_assert_eq!(project.deleted_at.is_some(), is_deleted);
                prop_assert_eq!(deleted_ids.contains(&id), is_deleted);
                if let Some(at) = project.deleted_at {
                    prop_assert!(project.recoverable_until > at);
                }
            }
        }
    }
}
