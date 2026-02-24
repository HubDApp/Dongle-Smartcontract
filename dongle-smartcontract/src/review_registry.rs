use crate::events::publish_review_event;
use crate::types::{Review, ReviewAction, ReviewEventData, DataKey, ProjectStats};
use crate::errors::ContractError;
use crate::rating_calculator::RatingCalculator;
use soroban_sdk::{Address, Env, String, contract, contractimpl};

#[contract]
pub struct ReviewRegistry;

#[contractimpl]
impl ReviewRegistry {
    pub fn add_review(
        env: Env,
        project_id: u64,
        reviewer: Address,
        rating: u32, // Matches types.rs u32
        comment_cid: Option<String>,
    ) {
        // Check if review exists
        // Save review in Map<(u64, Address), Review>
        // Update aggregates

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
        // Only original reviewer can update

        publish_review_event(
            &env,
            project_id,
            reviewer,
            ReviewAction::Updated,
            comment_cid,
        );
    }

    pub fn delete_review(env: Env, project_id: u64, reviewer: Address) {
        // Only original reviewer can delete

        publish_review_event(&env, project_id, reviewer, ReviewAction::Deleted, None);
    }

    pub fn delete_review(env: Env, project_id: u64, reviewer: Address) -> Result<(), ContractError> {
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
        let mut stats: ProjectStats = env
            .storage()
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
        None
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{
        Env, IntoVal, String, TryIntoVal,
        testutils::{Address as _, Events},
    };

    #[test]
    fn test_add_review_event() {
        let env = Env::default();
        let reviewer = Address::generate(&env);
        let comment_cid = String::from_str(&env, "QmHash");
        let contract_id = env.register_contract(None, ReviewRegistry);
        let client = ReviewRegistryClient::new(&env, &contract_id);

        client.add_review(&1, &reviewer, &5, &Some(comment_cid.clone()));

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

    #[test]
    fn test_update_review_event() {
        let env = Env::default();
        let reviewer = Address::generate(&env);
        let comment_cid = String::from_str(&env, "QmHash2");
        let contract_id = env.register_contract(None, ReviewRegistry);
        let client = ReviewRegistryClient::new(&env, &contract_id);

        client.update_review(&1, &reviewer, &4, &Some(comment_cid.clone()));

        let events = env.events().all();
        assert_eq!(events.len(), 1);

        let (_, topics, data) = events.last().unwrap();
        let topic1: soroban_sdk::Symbol = topics.get(1).unwrap().into_val(&env);
        assert_eq!(topic1, soroban_sdk::symbol_short!("UPDATED"));

        let event_data: ReviewEventData = data.into_val(&env);
        assert_eq!(event_data.action, ReviewAction::Updated);
        assert_eq!(event_data.comment_cid, Some(comment_cid));
    }

    #[test]
    fn test_delete_review_event() {
        let env = Env::default();
        let reviewer = Address::generate(&env);
        let contract_id = env.register_contract(None, ReviewRegistry);
        let client = ReviewRegistryClient::new(&env, &contract_id);

        client.delete_review(&1, &reviewer);

        let events = env.events().all();
        assert_eq!(events.len(), 1);

        let (_, topics, data) = events.last().unwrap();
        let topic1: soroban_sdk::Symbol = topics.get(1).unwrap().into_val(&env);
        assert_eq!(topic1, soroban_sdk::symbol_short!("DELETED"));

        let event_data: ReviewEventData = data.into_val(&env);
        assert_eq!(event_data.action, ReviewAction::Deleted);
        assert_eq!(event_data.comment_cid, None);
    }
}