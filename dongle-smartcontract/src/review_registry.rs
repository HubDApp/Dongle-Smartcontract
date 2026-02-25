use crate::errors::ContractError;
use crate::events::publish_review_event;
use crate::rating_calculator::RatingCalculator;
use crate::types::{DataKey, ProjectStats, Review, ReviewAction, ReviewEventData};
use soroban_sdk::{contract, contractimpl, Address, Env, String};

#[contract]
pub struct ReviewRegistry;

#[contractimpl]
impl ReviewRegistry {
    pub fn add_review(
        env: Env,
        project_id: u64,
        reviewer: Address,
        rating: u32,
        comment_cid: Option<String>,
    ) {
        reviewer.require_auth();

        let review_key = DataKey::Review(project_id, reviewer.clone());
        let review = Review {
            project_id,
            reviewer: reviewer.clone(),
            rating,
            timestamp: env.ledger().timestamp(),
            comment_cid: comment_cid.clone(),
            is_deleted: false,
        };

        // If it's a new review, add it to the user's list
        if !env.storage().persistent().has(&review_key) {
            let mut user_reviews: soroban_sdk::Vec<u64> = env
                .storage()
                .persistent()
                .get(&DataKey::UserReviews(reviewer.clone()))
                .unwrap_or(soroban_sdk::Vec::new(&env));
            user_reviews.push_back(project_id);
            env.storage()
                .persistent()
                .set(&DataKey::UserReviews(reviewer.clone()), &user_reviews);
        }

        env.storage().persistent().set(&review_key, &review);

        publish_review_event(
            &env,
            project_id,
            reviewer,
            ReviewAction::Submitted,
            comment_cid,
        );
    }

    pub fn update_review(
        env: Env,
        project_id: u64,
        reviewer: Address,
        rating: u32,
        comment_cid: Option<String>,
    ) {
        reviewer.require_auth();

        let review_key = DataKey::Review(project_id, reviewer.clone());
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
            &env,
            project_id,
            reviewer,
            ReviewAction::Updated,
            comment_cid,
        );
    }

    pub fn delete_review(
        env: Env,
        project_id: u64,
        reviewer: Address,
    ) -> Result<(), ContractError> {
        // 1. Authorize the caller - only the reviewer can delete their own review
        reviewer.require_auth();

        // 2. Fetch the review
        let review_key = DataKey::Review(project_id, reviewer.clone());
        let mut review: Review = env
            .storage()
            .persistent()
            .get(&review_key)
            .ok_or(ContractError::ReviewNotFound)?;

        // 3. Validate it hasn't already been deleted
        if review.is_deleted {
            return Err(ContractError::ReviewAlreadyDeleted);
        }

        // 4. Update the aggregate ratings
        let stats_key = DataKey::ProjectStats(project_id);
        let mut stats: ProjectStats =
            env.storage()
                .persistent()
                .get(&stats_key)
                .unwrap_or(ProjectStats {
                    rating_sum: 0,
                    review_count: 0,
                    average_rating: 0,
                });

        // Use your RatingCalculator to safely remove the rating
        if stats.review_count > 0 {
            let (new_sum, new_count, new_avg) = RatingCalculator::remove_rating(
                stats.rating_sum,
                stats.review_count,
                review.rating,
            );

            stats.rating_sum = new_sum;
            stats.review_count = new_count;
            stats.average_rating = new_avg;

            // Save the updated stats
            env.storage().persistent().set(&stats_key, &stats);
        }

        // 5. Perform the soft delete
        review.is_deleted = true;
        env.storage().persistent().set(&review_key, &review);

        // 6. Emit the deleted event
        publish_review_event(&env, project_id, reviewer, ReviewAction::Deleted, None);

        Ok(())
    }

    pub fn get_review(env: Env, project_id: u64, reviewer: Address) -> Option<Review> {
        env.storage()
            .persistent()
            .get(&DataKey::Review(project_id, reviewer))
    }

    pub fn get_reviews_by_user(
        env: Env,
        user: Address,
        offset: u32,
        limit: u32,
    ) -> soroban_sdk::Vec<Review> {
        let project_ids: soroban_sdk::Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::UserReviews(user.clone()))
            .unwrap_or(soroban_sdk::Vec::new(&env));

        let mut reviews = soroban_sdk::Vec::new(&env);
        let start = offset;
        let end = core::cmp::min(offset + limit, project_ids.len());

        for i in start..end {
            let project_id = project_ids.get(i).unwrap();
            if let Some(review) = Self::get_review(env.clone(), project_id, user.clone()) {
                reviews.push_back(review);
            }
        }

        reviews
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::String as SorobanString;
    use soroban_sdk::{
        testutils::{Address as _, Events},
        Env, IntoVal, String, TryIntoVal,
    }; // Alias for clarity

    #[test]
    fn test_add_review_event() {
        let env = Env::default();
        let reviewer = Address::generate(&env);
        let comment_cid = String::from_str(&env, "QmHash");
        let contract_id = env.register_contract(None, ReviewRegistry);
        let client = ReviewRegistryClient::new(&env, &contract_id);

        client
            .mock_all_auths()
            .add_review(&1, &reviewer, &5, &Some(comment_cid.clone()));

        let events = env.events().all();
        assert_eq!(events.len(), 1);

        let (_, topics, data) = events.last().unwrap();

        assert_eq!(topics.len(), 4);

        let topic0: soroban_sdk::Symbol = topics.get(0).unwrap().into_val(&env);
        let topic1: soroban_sdk::Symbol = topics.get(1).unwrap().into_val(&env);
        let topic2: u64 = topics.get(2).unwrap().into_val(&env);
        let topic3: Address = topics.get(3).unwrap().into_val(&env);

        assert_eq!(topic0, soroban_sdk::symbol_short!("REVIEW"));
        assert_eq!(topic1, soroban_sdk::symbol_short!("SUBMITTED"));
        assert_eq!(topic2, 1u64);
        assert_eq!(topic3, reviewer);

        let event_data: ReviewEventData = data.into_val(&env);
        assert_eq!(event_data.project_id, 1);
        assert_eq!(event_data.reviewer, reviewer);
        assert_eq!(event_data.action, ReviewAction::Submitted);
        assert_eq!(event_data.comment_cid, Some(comment_cid));
    }

    /*
        #[test]
        fn test_update_review_event() {
            let env = Env::default();
            let reviewer = Address::generate(&env);
            let comment_cid = String::from_str(&env, "QmHash2");
            let contract_id = env.register_contract(None, ReviewRegistry);
            let client = ReviewRegistryClient::new(&env, &contract_id);

            client.mock_all_auths().update_review(&1, &reviewer, &4, &Some(comment_cid.clone()));

            let events = env.events().all();
            assert_eq!(events.len(), 1);

            let (_, topics, data) = events.last().unwrap();
            let topic1: soroban_sdk::Symbol = topics.get(1).unwrap().into_val(&env);
            assert_eq!(topic1, soroban_sdk::symbol_short!("UPDATED"));

            let event_data: ReviewEventData = data.into_val(&env);
            assert_eq!(event_data.action, ReviewAction::Updated);
            assert_eq!(event_data.comment_cid, Some(comment_cid));
        }
    */

    /*
        #[test]
        fn test_delete_review_event() {
            let env = Env::default();
            let reviewer = Address::generate(&env);
            let contract_id = env.register_contract(None, ReviewRegistry);
            let client = ReviewRegistryClient::new(&env, &contract_id);

            client.mock_all_auths().delete_review(&1, &reviewer);

            let events = env.events().all();
            assert_eq!(events.len(), 1);

            let (_, topics, data) = events.last().unwrap();
            let topic1: soroban_sdk::Symbol = topics.get(1).unwrap().into_val(&env);
            assert_eq!(topic1, soroban_sdk::symbol_short!("DELETED"));

            let event_data: ReviewEventData = data.into_val(&env);
            assert_eq!(event_data.action, ReviewAction::Deleted);
            assert_eq!(event_data.comment_cid, None);
        }
    */
}
