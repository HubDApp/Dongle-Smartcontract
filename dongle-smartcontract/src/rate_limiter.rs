//! Rate limiting module to prevent spam and abuse of write actions.
//!
//! This module implements cooldown-based rate limiting for user actions.
//! Each user has separate cooldowns for different action types.

use crate::constants::{REVIEW_ACTION_COOLDOWN, VERIFICATION_REQUEST_COOLDOWN};
use crate::errors::ContractError;
use crate::storage_keys::StorageKey;
use soroban_sdk::{Address, Env};

/// Rate limiter for user actions
pub struct RateLimiter;

impl RateLimiter {
    /// Check and enforce rate limit for review actions (add/update/delete)
    pub fn check_review_action_cooldown(env: &Env, user: &Address) -> Result<(), ContractError> {
        let last_action_key = StorageKey::UserLastReviewAction(user.clone());
        let now = env.ledger().timestamp();

        if let Some(last_timestamp) = env.storage().persistent().get(&last_action_key) {
            if now < last_timestamp + REVIEW_ACTION_COOLDOWN {
                return Err(ContractError::RateLimitExceeded);
            }
        }

        // Update the last action timestamp
        env.storage().persistent().set(&last_action_key, &now);
        Ok(())
    }

    /// Check and enforce rate limit for verification requests
    pub fn check_verification_request_cooldown(env: &Env, user: &Address) -> Result<(), ContractError> {
        let last_request_key = StorageKey::UserLastVerificationRequest(user.clone());
        let now = env.ledger().timestamp();

        if let Some(last_timestamp) = env.storage().persistent().get(&last_request_key) {
            if now < last_timestamp + VERIFICATION_REQUEST_COOLDOWN {
                return Err(ContractError::RateLimitExceeded);
            }
        }

        // Update the last request timestamp
        env.storage().persistent().set(&last_request_key, &now);
        Ok(())
    }

    /// Get the remaining cooldown time for review actions (in seconds)
    /// Returns 0 if no cooldown is active
    pub fn get_review_action_cooldown_remaining(env: &Env, user: &Address) -> u64 {
        let last_action_key = StorageKey::UserLastReviewAction(user.clone());
        let now = env.ledger().timestamp();

        if let Some(last_timestamp) = env.storage().persistent().get(&last_action_key) {
            let cooldown_end = last_timestamp + REVIEW_ACTION_COOLDOWN;
            if now < cooldown_end {
                return cooldown_end - now;
            }
        }
        0
    }

    /// Get the remaining cooldown time for verification requests (in seconds)
    /// Returns 0 if no cooldown is active
    pub fn get_verification_request_cooldown_remaining(env: &Env, user: &Address) -> u64 {
        let last_request_key = StorageKey::UserLastVerificationRequest(user.clone());
        let now = env.ledger().timestamp();

        if let Some(last_timestamp) = env.storage().persistent().get(&last_request_key) {
            let cooldown_end = last_timestamp + VERIFICATION_REQUEST_COOLDOWN;
            if now < cooldown_end {
                return cooldown_end - now;
            }
        }
        0
    }
}