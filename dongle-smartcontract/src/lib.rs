use soroban_sdk::{contract, contractimpl, Env, Address, String};

mod review_registry;
use crate::review_registry::{ReviewRegistry, Review};

#[contract]
pub struct HubDAppContract;

#[contractimpl]
impl HubDAppContract {

    pub fn add_review(
        env:         Env,
        project_id:  u64,
        reviewer:    Address,
        rating:      u32,
        comment_cid: Option<String>,
    ) {
        ReviewRegistry::add_review(&env, project_id, reviewer, rating, comment_cid);
    }

    pub fn update_review(
        env:         Env,
        project_id:  u64,
        reviewer:    Address,
        new_rating:  u32,
        comment_cid: Option<String>,
    ) {
        ReviewRegistry::update_review(&env, project_id, reviewer, new_rating, comment_cid);
    }

    pub fn get_review(env: Env, project_id: u64, reviewer: Address) -> Option<Review> {
        ReviewRegistry::get_review(&env, project_id, reviewer)
    }

    pub fn average_rating(env: Env, project_id: u64) -> u64 {
        ReviewRegistry::average_rating(&env, project_id)
    }
}