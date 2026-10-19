//! Per-address rate limiting (closes #945).
//!
//! Prevents DOS attacks and abuse of contract endpoints by tracking calls per
//! address with configurable limits per operation type.
//!
//! - [`RateLimiter::check_and_consume`] records one call and returns
//!   `ContractError::RateLimitExceeded` when the limit is hit.
//! - Limits reset on a fixed schedule: each operation has a window in seconds
//!   (hourly, daily, …); the first call after the window opens a new window.
//! - [`RateLimiter::configure_limit`] (admin) sets `max_calls` + `window_secs`
//!   per operation; [`RateLimiter::default_limit`] covers known operations.
//! - [`RateLimiter::reset_rate_limit`] (admin) clears one caller's window.
//! - [`RateLimiter::set_trusted`] (admin) marks trusted addresses that bypass
//!   limits (admin override).

use crate::admin_manager::AdminManager;
use crate::errors::ContractError;
use soroban_sdk::{Address, Env, Symbol, contracttype, symbol_short};

/// Storage key binding a caller to one operation type.
#[contracttype]
#[derive(Clone)]
pub struct RateLimitKey {
    pub user: Address,
    pub op: Symbol,
}

/// Call count inside the current window.
#[contracttype]
#[derive(Clone)]
pub struct RateLimitEntry {
    pub count: u32,
    pub window_start: u64,
}

/// Stored per-operation limit configuration.
#[contracttype]
#[derive(Clone)]
pub struct RateLimitConfig {
    pub max_calls: u32,
    pub window_secs: u64,
}

/// Namespaced storage key for per-operation limit configs
/// (avoids collisions with view-cache `Symbol` keys).
#[contracttype]
#[derive(Clone)]
pub struct LimitKey(pub Symbol);

/// Per-address rate limiter with per-operation limits and reset schedules.
pub struct RateLimiter;

impl RateLimiter {
    /// Default `(max_calls, window_secs)` per operation type.
    pub fn default_limit(env: &Env, op: Symbol) -> (u32, u64) {
        if op == symbol_short!("submit_rev") {
            (10, 3600) // 10 reviews/hour
        } else if op == symbol_short!("add_project") {
            (20, 86400) // 20 projects/day
        } else if op == symbol_short!("follow") {
            (100, 3600) // 100 follows/hour
        } else {
            let _ = env;
            (100, 3600)
        }
    }

    fn config_key(op: &Symbol) -> LimitKey {
        LimitKey(op.clone())
    }

    /// Admin: configure `max_calls` per `window_secs` for an operation type.
    pub fn configure_limit(
        env: &Env,
        admin: Address,
        op: Symbol,
        max_calls: u32,
        window_secs: u64,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        AdminManager::require_admin(env, &admin)?;
        env.storage().persistent().set(
            &Self::config_key(&op),
            &RateLimitConfig {
                max_calls,
                window_secs,
            },
        );
        Ok(())
    }

    fn limit_for(env: &Env, op: &Symbol) -> (u32, u64) {
        env.storage()
            .persistent()
            .get::<LimitKey, RateLimitConfig>(&Self::config_key(op))
            .map(|c| (c.max_calls, c.window_secs))
            .unwrap_or_else(|| Self::default_limit(env, op.clone()))
    }

    /// Admin: mark/unmark a trusted address that bypasses all limits.
    pub fn set_trusted(
        env: &Env,
        admin: Address,
        user: Address,
        trusted: bool,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        AdminManager::require_admin(env, &admin)?;
        env.storage().persistent().set(&user, &trusted);
        Ok(())
    }

    /// Whether `user` bypasses rate limits.
    pub fn is_trusted(env: &Env, user: &Address) -> bool {
        env.storage().persistent().get(user).unwrap_or(false)
    }

    /// Record one call; Err(RateLimitExceeded) when the limit is hit.
    ///
    /// Windows reset on schedule: if `now - window_start >= window_secs`, a
    /// fresh window starts. Trusted addresses always succeed.
    pub fn check_and_consume(env: &Env, user: Address, op: Symbol) -> Result<(), ContractError> {
        if Self::is_trusted(env, &user) {
            return Ok(());
        }
        let (max_calls, window_secs) = Self::limit_for(env, &op);
        let key = RateLimitKey {
            user: user.clone(),
            op: op.clone(),
        };
        let now = env.ledger().timestamp();
        let mut entry: RateLimitEntry = env.storage().persistent().get(&key).unwrap_or(RateLimitEntry {
            count: 0,
            window_start: now,
        });
        if now.saturating_sub(entry.window_start) >= window_secs {
            entry = RateLimitEntry {
                count: 0,
                window_start: now,
            };
        }
        if entry.count >= max_calls {
            return Err(ContractError::RateLimitExceeded);
        }
        entry.count = entry.count.saturating_add(1);
        env.storage().persistent().set(&key, &entry);
        Ok(())
    }

    /// Remaining calls for `user` + `op` in the current window.
    pub fn get_remaining(env: &Env, user: Address, op: Symbol) -> u32 {
        if Self::is_trusted(env, &user) {
            return u32::MAX;
        }
        let (max_calls, window_secs) = Self::limit_for(env, &op);
        let key = RateLimitKey { user, op };
        let now = env.ledger().timestamp();
        match env.storage().persistent().get::<RateLimitKey, RateLimitEntry>(&key) {
            Some(e) if now.saturating_sub(e.window_start) < window_secs => {
                max_calls.saturating_sub(e.count)
            }
            _ => max_calls,
        }
    }

    /// Admin: clear one caller's window for one operation (manual reset).
    pub fn reset_rate_limit(
        env: &Env,
        admin: Address,
        user: Address,
        op: Symbol,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        AdminManager::require_admin(env, &admin)?;
        env.storage()
            .persistent()
            .remove(&RateLimitKey { user, op });
        Ok(())
    }
}
