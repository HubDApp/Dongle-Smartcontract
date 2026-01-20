use crate::types::Review;
use soroban_sdk::{Env, Address, Map};

pub struct ReviewRegistry;

impl ReviewRegistry {
    pub fn add_review(env: &Env, project_id: u64, reviewer: Address, rating: u8, comment_cid: Option<String>) {
        // Check if review exists
        // Save review in Map<(u64, Address), Review>
        // Update aggregates
    }

    pub fn update_review(env: &Env, project_id: u64, reviewer: Address, rating: u8, comment_cid: Option<String>) {
        // Only original reviewer can update
    }

    pub fn get_review(env: &Env, project_id: u64, reviewer: Address) -> Option<Review> {
        None
    }
}
