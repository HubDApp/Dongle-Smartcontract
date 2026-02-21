use crate::types::Review;
use soroban_sdk::{Env, Address, String};

pub struct ReviewRegistry;

impl ReviewRegistry {
    pub fn add_review(env: &Env, project_id: u64, reviewer: Address, rating: u32, comment_cid: Option<String>) {
        // Check if review exists
        // Save review in Map<(u64, Address), Review>
        // Update aggregates
    }

    pub fn update_review(_env: &Env, _project_id: u64, _reviewer: Address, _rating: u32, _comment_cid: Option<String>) {
        // Only original reviewer can update
    }

    pub fn get_review(_env: &Env, _project_id: u64, _reviewer: Address) -> Option<Review> {
        None
    }
}
