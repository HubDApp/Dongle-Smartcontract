use crate::events::publish_review_event;
use crate::types::{Review, ReviewAction};
use crate::storage_keys::StorageKey;
use soroban_sdk::{Address, Env, String, contract, contractimpl};

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
        let key = StorageKey::Review(project_id, reviewer.clone());
        
        let review = Review {
            project_id,
            reviewer: reviewer.clone(),
            rating,
            comment_cid: comment_cid.clone(),
            timestamp: env.ledger().timestamp(),
        };

        // If it's a new review, add reviewer to the project list
        if !env.storage().persistent().has(&key) {
            let list_key = StorageKey::ProjectReviewers(project_id);
            let mut list = env.storage().persistent().get::<_, soroban_sdk::Vec<Address>>(&list_key)
                .unwrap_or_else(|| soroban_sdk::vec![&env]);
            list.push_back(reviewer.clone());
            env.storage().persistent().set(&list_key, &list);
        }

        env.storage().persistent().set(&key, &review);

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
        let key = StorageKey::Review(project_id, reviewer.clone());
        
        if let Some(mut review) = env.storage().persistent().get::<_, Review>(&key) {
            review.rating = rating;
            review.comment_cid = comment_cid.clone();
            review.timestamp = env.ledger().timestamp();
            
            env.storage().persistent().set(&key, &review);

            publish_review_event(
                &env,
                project_id,
                reviewer,
                ReviewAction::Updated,
                comment_cid,
            );
        }
    }

    pub fn delete_review(env: Env, project_id: u64, reviewer: Address) {
        let key = StorageKey::Review(project_id, reviewer.clone());
        if env.storage().persistent().has(&key) {
            env.storage().persistent().remove(&key);
            
            // Note: We don't remove from the ProjectReviewers list for simplicity and gas efficiency,
            // get_reviews_by_project will skip missing reviews.
            
            publish_review_event(&env, project_id, reviewer, ReviewAction::Deleted, None);
        }
    }

    pub fn get_review(env: Env, project_id: u64, reviewer: Address) -> Option<Review> {
        let key = StorageKey::Review(project_id, reviewer);
        env.storage().persistent().get(&key)
    }

    pub fn get_reviews_by_project(env: Env, project_id: u64) -> soroban_sdk::Vec<Review> {
        let list_key = StorageKey::ProjectReviewers(project_id);
        let reviewers = env.storage().persistent().get::<_, soroban_sdk::Vec<Address>>(&list_key)
            .unwrap_or_else(|| soroban_sdk::vec![&env]);
        
        let mut reviews = soroban_sdk::vec![&env];
        for reviewer in reviewers.iter() {
            if let Some(review) = Self::get_review(env.clone(), project_id, reviewer) {
                reviews.push_back(review);
            }
        }
        reviews
    }
}
