//! Review submission with validation, duplicate handling, and events.

use crate::errors::ContractError;
use crate::events::publish_review_event;
use crate::rating_calculator::RatingCalculator;
use crate::types::{ProjectStats, Review, ReviewAction};
use crate::storage_keys::StorageKey;
use soroban_sdk::{Address, Env, String};

pub struct ReviewRegistry;

impl ReviewRegistry {
    pub fn add_review(
        env: &Env,
        project_id: u64,
        reviewer: Address,
        rating: u32,
        comment_cid: Option<String>,
    ) {
        reviewer.require_auth();

        let review_key = StorageKey::Review(project_id, reviewer.clone());
        let review = Review {
            project_id,
            reviewer: reviewer.clone(),
            rating,
            timestamp: env.ledger().timestamp(),
            comment_cid: comment_cid.clone(),
        };

        if !env.storage().persistent().has(&review_key) {
            let mut user_reviews: soroban_sdk::Vec<u64> = env
                .storage()
                .persistent()
                .get(&StorageKey::UserReviews(reviewer.clone()))
                .unwrap_or(soroban_sdk::Vec::new(env));
            user_reviews.push_back(project_id);
            env.storage()
                .persistent()
                .set(&StorageKey::UserReviews(reviewer.clone()), &user_reviews);
        }

        env.storage().persistent().set(&review_key, &review);

        publish_review_event(
            env,
            project_id,
            reviewer,
            ReviewAction::Submitted,
            comment_cid,
        );
    }

    pub fn update_review(
        env: &Env,
        project_id: u64,
        reviewer: Address,
        rating: u32,
        comment_cid: Option<String>,
    ) {
        reviewer.require_auth();

        let review_key = StorageKey::Review(project_id, reviewer.clone());
        let mut review: Review = env
            .storage()
            .persistent()
            .get(&review_key)
            .expect("Review not found");

        review.rating = rating;
        review.comment_cid = comment_cid.clone();
        review.timestamp = env.ledger().timestamp();

        env.storage().persistent().set(&review_key, &review);

        publish_review_event(
            env,
            project_id,
            reviewer,
            ReviewAction::Updated,
            comment_cid,
        );
    }

    pub fn delete_review(
        env: &Env,
        project_id: u64,
        reviewer: Address,
    ) -> Result<(), ContractError> {
        reviewer.require_auth();

        let review_key = StorageKey::Review(project_id, reviewer.clone());
        let review: Review = env
            .storage()
            .persistent()
            .get(&review_key)
            .ok_or(ContractError::ReviewNotFound)?;

        let stats_key = StorageKey::ProjectStats(project_id);
        let mut stats: ProjectStats =
            env.storage()
                .persistent()
                .get(&stats_key)
                .unwrap_or(ProjectStats {
                    rating_sum: 0,
                    review_count: 0,
                    average_rating: 0,
                });

        if stats.review_count > 0 {
            let (new_sum, new_count, new_avg) = RatingCalculator::remove_rating(
                stats.rating_sum,
                stats.review_count,
                review.rating,
            );

            stats.rating_sum = new_sum;
            stats.review_count = new_count;
            stats.average_rating = new_avg;

            env.storage().persistent().set(&stats_key, &stats);
        }

        env.storage().persistent().remove(&review_key);

        let mut user_reviews: soroban_sdk::Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::UserReviews(reviewer.clone()))
            .unwrap_or(soroban_sdk::Vec::new(env));

        let mut new_user_reviews = soroban_sdk::Vec::new(env);
        for i in 0..user_reviews.len() {
            if let Some(id) = user_reviews.get(i) {
                if id != project_id {
                    new_user_reviews.push_back(id);
                }
            }
        }
        env.storage()
            .persistent()
            .set(&StorageKey::UserReviews(reviewer.clone()), &new_user_reviews);

        publish_review_event(env, project_id, reviewer, ReviewAction::Deleted, None);

        Ok(())
    }

    pub fn get_review(env: &Env, project_id: u64, reviewer: Address) -> Option<Review> {
        env.storage()
            .persistent()
            .get(&StorageKey::Review(project_id, reviewer))
    }

    pub fn get_reviews_by_user(
        env: &Env,
        user: Address,
        offset: u32,
        limit: u32,
    ) -> soroban_sdk::Vec<Review> {
        let project_ids: soroban_sdk::Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::UserReviews(user.clone()))
            .unwrap_or(soroban_sdk::Vec::new(env));

        let mut reviews = soroban_sdk::Vec::new(env);
        let start = offset;
        let len = project_ids.len();
        let end = core::cmp::min(offset.saturating_add(limit), len);

        for i in start..end {
            if let Some(project_id) = project_ids.get(i) {
                if let Some(review) = Self::get_review(env, project_id, user.clone()) {
                    reviews.push_back(review);
                }
            }
        }

        reviews
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::types::ReviewEventData;
    use crate::{DongleContract, DongleContractClient};
    use soroban_sdk::String as SorobanString;
    use soroban_sdk::{
        testutils::{Address as _, Events},
        Env, IntoVal, String,
    };

    #[test]
    fn test_add_review_event() {
        let env = Env::default();
        env.mock_all_auths();
        let reviewer = Address::generate(&env);
        let owner = Address::generate(&env);
        let comment_cid = String::from_str(&env, "QmHash");
        let contract_id = env.register_contract(None, DongleContract);
        let client = DongleContractClient::new(&env, &contract_id);

        client.initialize(&owner);
        let name = SorobanString::from_str(&env, "Test Project");
        let desc =
            SorobanString::from_str(&env, "A description that is long enough for validation.");
        let cat = SorobanString::from_str(&env, "DeFi");
        let project_id = client.register_project(&owner, &name, &desc, &cat, &None, &None, &None);
        client.add_review(&project_id, &reviewer, &5, &Some(comment_cid.clone()));

        let events = env.events().all();
        assert!(events.len() >= 1);

        let (_, topics, data) = events.last().unwrap();
        assert_eq!(topics.len(), 4);

        let topic0: soroban_sdk::Symbol = topics.get(0).unwrap().into_val(&env);
        let topic1: soroban_sdk::Symbol = topics.get(1).unwrap().into_val(&env);
        let topic2: u64 = topics.get(2).unwrap().into_val(&env);
        let topic3: Address = topics.get(3).unwrap().into_val(&env);

        assert_eq!(topic0, soroban_sdk::symbol_short!("REVIEW"));
        assert_eq!(topic1, soroban_sdk::symbol_short!("SUBMITTED"));
        assert_eq!(topic2, project_id);
        assert_eq!(topic3, reviewer);

        let event_data: ReviewEventData = data.into_val(&env);
        assert_eq!(event_data.project_id, project_id);
        assert_eq!(event_data.reviewer, reviewer);
        assert_eq!(event_data.action, ReviewAction::Submitted);
        assert_eq!(event_data.comment_cid, Some(comment_cid));
    }
}
