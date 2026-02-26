 //! Review submission with validation, duplicate handling, hard-delete,
//! user review index, events, and proper aggregate updates.

use crate::errors::ContractError;
use crate::events::publish_review_event;
use crate::rating_calculator::RatingCalculator;
use crate::storage_keys::StorageKey;
use crate::types::{ProjectStats, Review, ReviewAction};
use soroban_sdk::{Address, Env, String, Vec};

pub struct ReviewRegistry;

#[allow(dead_code)]
impl ReviewRegistry {
    pub fn add_review(
        env: &Env,
        project_id: u64,
        reviewer: Address,
        rating: u32,
        comment_cid: Option<String>,
    ) -> Result<(), ContractError> {
        reviewer.require_auth();

        if !(1..=5).contains(&rating) {
            return Err(ContractError::InvalidRating);
        }

        let review_key = StorageKey::Review(project_id, reviewer.clone());
        if env.storage().persistent().has(&review_key) {
            return Err(ContractError::DuplicateReview);
        }

        let review = Review {
            project_id,
            reviewer: reviewer.clone(),
            rating,
            timestamp: env.ledger().timestamp(),
            comment_cid: comment_cid.clone(),
        };
        env.storage().persistent().set(&review_key, &review);

        // Update user review index (first submission only — guaranteed by duplicate check above)
        let mut user_reviews: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::UserReviews(reviewer.clone()))
            .unwrap_or_else(|| Vec::new(env));
        user_reviews.push_back(project_id);
        env.storage()
            .persistent()
            .set(&StorageKey::UserReviews(reviewer.clone()), &user_reviews);

        // Update project reviewer index
        let mut project_reviews: Vec<Address> = env
            .storage()
            .persistent()
            .get(&StorageKey::ProjectReviews(project_id))
            .unwrap_or_else(|| Vec::new(env));
        project_reviews.push_back(reviewer.clone());
        env.storage()
            .persistent()
            .set(&StorageKey::ProjectReviews(project_id), &project_reviews);

        // Update aggregate stats
        let stats_key = StorageKey::ProjectStats(project_id);
        let stats: ProjectStats = env
            .storage()
            .persistent()
            .get(&stats_key)
            .unwrap_or(ProjectStats {
                rating_sum: 0,
                review_count: 0,
                average_rating: 0,
            });
        let (new_sum, new_count, new_avg) =
            RatingCalculator::add_rating(stats.rating_sum, stats.review_count, rating);
        env.storage().persistent().set(
            &stats_key,
            &ProjectStats {
                rating_sum: new_sum,
                review_count: new_count,
                average_rating: new_avg,
            },
        );

        publish_review_event(
            env,
            project_id,
            reviewer,
            ReviewAction::Submitted,
            comment_cid,
        );
        Ok(())
    }

    pub fn update_review(
        env: &Env,
        project_id: u64,
        reviewer: Address,
        rating: u32,
        comment_cid: Option<String>,
    ) -> Result<(), ContractError> {
        reviewer.require_auth();

        if !(1..=5).contains(&rating) {
            return Err(ContractError::InvalidRating);
        }

        let review_key = StorageKey::Review(project_id, reviewer.clone());
        let mut review: Review = env
            .storage()
            .persistent()
            .get(&review_key)
            .ok_or(ContractError::ReviewNotFound)?;

        let old_rating = review.rating;
        review.rating = rating;
        review.comment_cid = comment_cid.clone();
        review.timestamp = env.ledger().timestamp();
        env.storage().persistent().set(&review_key, &review);

        // Update aggregate stats
        let stats_key = StorageKey::ProjectStats(project_id);
        let mut stats: ProjectStats = env
            .storage()
            .persistent()
            .get(&stats_key)
            .ok_or(ContractError::InvalidProjectData)?;
        let (new_sum, _new_count, new_avg) =
            RatingCalculator::update_rating(stats.rating_sum, stats.review_count, old_rating, rating);
        stats.rating_sum = new_sum;
        stats.average_rating = new_avg;
        env.storage().persistent().set(&stats_key, &stats);

        publish_review_event(
            env,
            project_id,
            reviewer,
            ReviewAction::Updated,
            comment_cid,
        );
        Ok(())
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

        // Hard delete
        env.storage().persistent().remove(&review_key);

        // Update aggregate stats
        let stats_key = StorageKey::ProjectStats(project_id);
        let mut stats: ProjectStats = env
            .storage()
            .persistent()
            .get(&stats_key)
            .unwrap_or(ProjectStats {
                rating_sum: 0,
                review_count: 0,
                average_rating: 0,
            });
        if stats.review_count > 0 {
            let (new_sum, new_count, new_avg) =
                RatingCalculator::remove_rating(stats.rating_sum, stats.review_count, review.rating);
            stats.rating_sum = new_sum;
            stats.review_count = new_count;
            stats.average_rating = new_avg;
            env.storage().persistent().set(&stats_key, &stats);
        }

        // Remove from user review index
        let user_reviews: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::UserReviews(reviewer.clone()))
            .unwrap_or_else(|| Vec::new(env));
        let mut new_user_reviews = Vec::new(env);
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

        // Remove from project reviewer index
        let project_reviews: Vec<Address> = env
            .storage()
            .persistent()
            .get(&StorageKey::ProjectReviews(project_id))
            .unwrap_or_else(|| Vec::new(env));
        let mut new_project_reviews = Vec::new(env);
        for i in 0..project_reviews.len() {
            if let Some(addr) = project_reviews.get(i) {
                if addr != reviewer {
                    new_project_reviews.push_back(addr);
                }
            }
        }
        env.storage()
            .persistent()
            .set(&StorageKey::ProjectReviews(project_id), &new_project_reviews);

        publish_review_event(env, project_id, reviewer, ReviewAction::Deleted, None);
        Ok(())
    }

    pub fn get_review(env: &Env, project_id: u64, reviewer: Address) -> Option<Review> {
        env.storage()
            .persistent()
            .get(&StorageKey::Review(project_id, reviewer))
    }

    pub fn list_reviews(env: &Env, project_id: u64, start_id: u32, limit: u32) -> Vec<Review> {
        let reviewers: Vec<Address> = env
            .storage()
            .persistent()
            .get(&StorageKey::ProjectReviews(project_id))
            .unwrap_or_else(|| Vec::new(env));

        let mut reviews = Vec::new(env);
        let len = reviewers.len();
        let end = core::cmp::min(start_id.saturating_add(limit), len);

        for i in start_id..end {
            if let Some(reviewer) = reviewers.get(i) {
                if let Some(review) = Self::get_review(env, project_id, reviewer) {
                    reviews.push_back(review);
                }
            }
        }
        reviews
    }

    pub fn get_reviews_by_user(env: &Env, user: Address, offset: u32, limit: u32) -> Vec<Review> {
        let project_ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::UserReviews(user.clone()))
            .unwrap_or_else(|| Vec::new(env));

        let mut reviews = Vec::new(env);
        let len = project_ids.len();
        let end = core::cmp::min(offset.saturating_add(limit), len);

        for i in offset..end {
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
        let params = crate::types::ProjectRegistrationParams {
            owner: owner.clone(),
            name: name.clone(),
            description: desc.clone(),
            category: cat.clone(),
            website: None,
            logo_cid: None,
            metadata_cid: None,
        };
        let project_id = client.register_project(&params).unwrap();
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