 #![no_std]

mod errors;
mod types;

use crate::errors::ContractError;
use crate::types::{ProjectAggregate, Review};
use soroban_sdk::{contracttype, symbol_short, Address, Env, String, Vec};

// ── Storage keys ──────────────────────────────────────────────────────────────

#[contracttype]
pub enum DataKey {
    Review(u64, Address),
    Aggregate(u64),
}

// ── Events ────────────────────────────────────────────────────────────────────

fn emit_review_added(env: &Env, project_id: u64, reviewer: &Address, rating: u32) {
    env.events().publish(
        (symbol_short!("rev_add"), project_id),
        (reviewer.clone(), rating),
    );
}

fn emit_review_updated(
    env: &Env,
    project_id: u64,
    reviewer: &Address,
    old_rating: u32,
    new_rating: u32,
) {
    env.events().publish(
        (symbol_short!("rev_upd"), project_id),
        (reviewer.clone(), old_rating, new_rating),
    );
}

// ── Registry ──────────────────────────────────────────────────────────────────

pub struct ReviewRegistry;

impl ReviewRegistry {
    fn validate_rating(rating: u32) -> Result<(), ContractError> {
        if rating < 1 || rating > 5 {
            return Err(ContractError::InvalidRating);
        }
        Ok(())
    }

    fn get_aggregate(env: &Env, project_id: u64) -> ProjectAggregate {
        env.storage()
            .persistent()
            .get(&DataKey::Aggregate(project_id))
            .unwrap_or_default()
    }

    fn save_aggregate(env: &Env, project_id: u64, agg: &ProjectAggregate) {
        env.storage()
            .persistent()
            .set(&DataKey::Aggregate(project_id), agg);
    }

    pub fn average_rating(env: &Env, project_id: u64) -> u64 {
        let agg = Self::get_aggregate(env, project_id);
        if agg.review_count == 0 {
            0
        } else {
            agg.total_rating / agg.review_count
        }
    }

    pub fn add_review(
        env: &Env,
        project_id: u64,
        reviewer: Address,
        rating: u32,
        comment_cid: Option<String>,
    ) -> Result<(), ContractError> {
        reviewer.require_auth();
        Self::validate_rating(rating)?;

        let key = DataKey::Review(project_id, reviewer.clone());
        if env.storage().persistent().has(&key) {
            return Err(ContractError::AlreadyExists);
        }

        let now = env.ledger().timestamp();
        let review = Review {
            reviewer: reviewer.clone(),
            project_id,
            rating,
            comment_cid,
            created_at: now,
            updated_at: now,
        };
        env.storage().persistent().set(&key, &review);

        let mut agg = Self::get_aggregate(env, project_id);
        agg.total_rating += rating as u64;
        agg.review_count += 1;
        Self::save_aggregate(env, project_id, &agg);

        emit_review_added(env, project_id, &reviewer, rating);
        Ok(())
    }

    pub fn update_review(
        env: &Env,
        project_id: u64,
        reviewer: Address,
        new_rating: u32,
        comment_cid: Option<String>,
    ) -> Result<(), ContractError> {
        reviewer.require_auth();
        Self::validate_rating(new_rating)?;

        let key = DataKey::Review(project_id, reviewer.clone());
        let mut review: Review = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(ContractError::NotFound)?;

        let old_rating = review.rating;
        let mut agg = Self::get_aggregate(env, project_id);
        agg.total_rating = agg
            .total_rating
            .saturating_sub(old_rating as u64)
            .saturating_add(new_rating as u64);
        Self::save_aggregate(env, project_id, &agg);

        review.rating = new_rating;
        review.comment_cid = comment_cid;
        review.updated_at = env.ledger().timestamp();
        env.storage().persistent().set(&key, &review);

        emit_review_updated(env, project_id, &reviewer, old_rating, new_rating);
        Ok(())
    }

    pub fn get_review(
        env: &Env,
        project_id: u64,
        reviewer: Address,
    ) -> Result<Review, ContractError> {
        env.storage()
            .persistent()
            .get(&DataKey::Review(project_id, reviewer))
            .ok_or(ContractError::NotFound)
    }

    pub fn get_project_reviews(
        _env: &Env,
        _project_id: u64,
        _start_reviewer: Option<Address>,
        _limit: u32,
    ) -> Result<Vec<Review>, ContractError> {
        todo!("Project review listing logic not implemented")
    }

    pub fn get_review_stats(
        env: &Env,
        project_id: u64,
    ) -> Result<(u32, u32), ContractError> {
        let agg = Self::get_aggregate(env, project_id);
        Ok((agg.review_count as u32, agg.total_rating as u32))
    }

    pub fn review_exists(env: &Env, project_id: u64, reviewer: Address) -> bool {
        env.storage()
            .persistent()
            .has(&DataKey::Review(project_id, reviewer))
    }

    pub fn delete_review(
        _env: &Env,
        _project_id: u64,
        _reviewer: Address,
        _admin: Address,
    ) -> Result<(), ContractError> {
        todo!("Review deletion logic not implemented")
    }
}
