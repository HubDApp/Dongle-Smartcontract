#![no_std]


mod errors;
mod fee_manager;
mod project_registry;
mod review_registry;
mod types;
mod utils;
mod verification_registry;


use crate::errors::ContractError;
use crate::review_registry::ReviewRegistry;
use crate::types::Review;
use soroban_sdk::{contract, contractimpl, Address, Env, String};


#[contract]
pub struct DongleContract;
#[contractimpl]
impl DongleContract {
    
    pub fn add_review(
        env: Env,
        project_id: u64,
        reviewer: Address,
        rating: u8,
        comment_cid: Option<String>,
    ) -> Result<(), ContractError> {
        ReviewRegistry::add_review(&env, project_id, reviewer, rating, comment_cid)
    }

    
    pub fn get_review(env: Env, project_id: u64, reviewer: Address) -> Option<Review> {
        ReviewRegistry::get_review(&env, project_id, reviewer)
    }

    
    pub fn update_review(
        env: Env,
        project_id: u64,
        reviewer: Address,
        rating: u8,
        comment_cid: Option<String>,
    ) -> Result<(), ContractError> {
        ReviewRegistry::update_review(&env, project_id, reviewer, rating, comment_cid)
    }
}