//! Periodic, admin-funded rewards for reviewers with high quality scores.

use crate::admin_manager::AdminManager;
use crate::errors::ContractError;
use crate::events::{publish_reward_claimed_event, publish_reward_period_finalized_event};
use crate::storage_keys::{ExtensionKey2, StorageKey};
use crate::types::{RewardPeriod, ReviewerReward, RewardPoolConfig};
use soroban_sdk::{Address, Env, Vec};

pub struct RewardRegistry;

impl RewardRegistry {
    pub fn configure(env: &Env, admin: Address, token: Address, period_duration: u64, max_reviewers: u32) -> Result<(), ContractError> {
        admin.require_auth();
        if !AdminManager::is_admin(env, &admin) { return Err(ContractError::AdminOnly); }
        if period_duration == 0 || max_reviewers == 0 { return Err(ContractError::InvalidInput); }
        env.storage().persistent().set(&ExtensionKey2::RewardPoolConfig, &RewardPoolConfig { token, period_duration, max_reviewers });
        Ok(())
    }

    pub fn get_config(env: &Env) -> Option<RewardPoolConfig> {
        env.storage().persistent().get(&ExtensionKey2::RewardPoolConfig)
    }

    pub fn fund(env: &Env, admin: Address, amount: u128) -> Result<(), ContractError> {
        admin.require_auth();
        if !AdminManager::is_admin(env, &admin) { return Err(ContractError::AdminOnly); }
        if amount == 0 || amount > i128::MAX as u128 { return Err(ContractError::InvalidInput); }
        let config = Self::get_config(env).ok_or(ContractError::RewardPoolNotConfigured)?;
        soroban_sdk::token::Client::new(env, &config.token).transfer(&admin, &env.current_contract_address(), &(amount as i128));
        let funded: u128 = env.storage().persistent().get(&ExtensionKey2::RewardPoolFunded).unwrap_or(0);
        env.storage().persistent().set(&ExtensionKey2::RewardPoolFunded, &funded.checked_add(amount).ok_or(ContractError::ArithmeticOverflow)?);
        Ok(())
    }

    pub fn set_quality_score(env: &Env, admin: Address, project_id: u64, reviewer: Address, score: u32) -> Result<(), ContractError> {
        admin.require_auth();
        if !AdminManager::is_admin(env, &admin) { return Err(ContractError::AdminOnly); }
        if score > 1_000 { return Err(ContractError::InvalidInput); }
        if !env.storage().persistent().has(&StorageKey::Review(project_id, reviewer.clone())) { return Err(ContractError::ReviewNotFound); }
        env.storage().persistent().set(&ExtensionKey2::ReviewQualityScore(project_id, reviewer), &score);
        Ok(())
    }

    pub fn finalize(env: &Env, admin: Address, start_at: u64, end_at: u64, candidates: Vec<Address>) -> Result<u64, ContractError> {
        admin.require_auth();
        if !AdminManager::is_admin(env, &admin) { return Err(ContractError::AdminOnly); }
        let config = Self::get_config(env).ok_or(ContractError::RewardPoolNotConfigured)?;
        if start_at >= end_at || end_at > env.ledger().timestamp() || end_at - start_at != config.period_duration { return Err(ContractError::InvalidInput); }
        if candidates.len() == 0 { return Err(ContractError::InvalidInput); }
        let period_id: u64 = env.storage().persistent().get(&ExtensionKey2::NextRewardPeriodId).unwrap_or(0);
        let mut total_score = 0u128;
        let mut scores: Vec<(Address, u128)> = Vec::new(env);
        for i in 0..candidates.len() {
            let reviewer = candidates.get(i).unwrap();
            let projects: Vec<u64> = env.storage().persistent().get(&StorageKey::UserReviews(reviewer.clone())).unwrap_or_else(|| Vec::new(env));
            let mut score = 0u128;
            for j in 0..projects.len() {
                let project_id = projects.get(j).unwrap();
                if let Some(review) = env.storage().persistent().get::<_, crate::types::Review>(&StorageKey::Review(project_id, reviewer.clone())) {
                    if !review.hidden && review.created_at >= start_at && review.created_at < end_at {
                        let quality: u32 = env.storage().persistent().get(&ExtensionKey2::ReviewQualityScore(project_id, reviewer.clone())).unwrap_or(0);
                        score = score.checked_add(quality as u128).ok_or(ContractError::ArithmeticOverflow)?;
                    }
                }
            }
            if score > 0 { scores.push_back((reviewer, score)); }
        }
        // Keep the highest scores only. Equal scores retain candidate order,
        // making ties deterministic and independently reproducible by indexers.
        for i in 1..scores.len() {
            let current = scores.get(i).unwrap();
            let mut j = i;
            while j > 0 && scores.get(j - 1).unwrap().1 < current.1 {
                let previous = scores.get(j - 1).unwrap();
                scores.set(j, previous);
                j -= 1;
            }
            scores.set(j, current);
        }
        while scores.len() as u32 > config.max_reviewers { scores.pop_back(); }
        for i in 0..scores.len() { total_score = total_score.checked_add(scores.get(i).unwrap().1).ok_or(ContractError::ArithmeticOverflow)?; }
        if total_score == 0 { return Err(ContractError::NoEligibleReviewers); }
        let funded: u128 = env.storage().persistent().get(&ExtensionKey2::RewardPoolFunded).unwrap_or(0);
        if funded == 0 { return Err(ContractError::RewardPoolInsufficient); }
        let mut rewards = Vec::new(env);
        for i in 0..scores.len() {
            let (reviewer, score) = scores.get(i).unwrap();
            let amount = funded.checked_mul(score).ok_or(ContractError::ArithmeticOverflow)? / total_score;
            if amount > 0 { rewards.push_back(ReviewerReward { period_id, reviewer, quality_score: score, amount, claimed_at: None }); }
        }
        let period = RewardPeriod { id: period_id, start_at, end_at, pool_amount: funded, total_quality_score: total_score, rewards: rewards.clone() };
        env.storage().persistent().set(&ExtensionKey2::RewardPeriod(period_id), &period);
        env.storage().persistent().set(&ExtensionKey2::RewardPoolFunded, &0u128);
        env.storage().persistent().set(&ExtensionKey2::NextRewardPeriodId, &(period_id + 1));
        publish_reward_period_finalized_event(env, period_id, funded, total_score, rewards.len() as u32);
        Ok(period_id)
    }

    pub fn get_period(env: &Env, period_id: u64) -> Option<RewardPeriod> {
        env.storage().persistent().get(&ExtensionKey2::RewardPeriod(period_id))
    }

    pub fn claim(env: &Env, reviewer: Address, period_id: u64) -> Result<u128, ContractError> {
        reviewer.require_auth();
        let mut period = Self::get_period(env, period_id).ok_or(ContractError::RewardPeriodNotFound)?;
        for i in 0..period.rewards.len() {
            let mut reward = period.rewards.get(i).unwrap();
            if reward.reviewer == reviewer {
                if reward.claimed_at.is_some() { return Err(ContractError::RewardAlreadyClaimed); }
                let amount = reward.amount;
                reward.claimed_at = Some(env.ledger().timestamp());
                period.rewards.set(i, reward);
                env.storage().persistent().set(&ExtensionKey2::RewardPeriod(period_id), &period);
                let config = Self::get_config(env).ok_or(ContractError::RewardPoolNotConfigured)?;
                soroban_sdk::token::Client::new(env, &config.token).transfer(&env.current_contract_address(), &reviewer, &(amount as i128));
                publish_reward_claimed_event(env, period_id, reviewer, amount);
                return Ok(amount);
            }
        }
        Err(ContractError::NoRewardAvailable)
    }
}