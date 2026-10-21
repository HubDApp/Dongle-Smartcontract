//! Cached computed views (closes #937).
//!
//! Expensive read aggregations — project stats, trending projects, and
//! recommendations — are cached with a configurable TTL (in ledgers) so
//! repeated reads do not recompute them.
//!
//! - [`ComputedViewCache::set_cached_view`] stores a view under a `Symbol`
//!   cache key with a TTL.
//! - [`ComputedViewCache::get_cached_view`] returns the value on a cache hit,
//!   or `None` on a miss/expiry (expired entries are evicted), while recording
//!   hit/miss counters for [`ComputedViewCache::cache_hit_rate`] monitoring.
//! - [`ComputedViewCache::invalidate`] performs manual cache invalidation.
//! - [`ComputedViewCache::refresh_cache`] is the admin-only refresh entry
//!   point: it recomputes (caller-supplied value) and stores the view.
//!
//! Cache keys are namespaced by the caller, e.g. `project_stats`,
//! `trending`, `recommendations`.

use crate::admin_manager::AdminManager;
use crate::errors::ContractError;
use soroban_sdk::{Address, Bytes, Env, Symbol, contracttype};

/// A cached view value with its expiry ledger.
#[contracttype]
#[derive(Clone)]
pub struct CachedView {
    pub value: Bytes,
    pub expires_at_ledger: u32,
}

fn hits_key(env: &Env) -> Symbol {
    Symbol::new(env, "view_hits")
}

fn misses_key(env: &Env) -> Symbol {
    Symbol::new(env, "view_miss")
}

fn bump(env: &Env, key: Symbol) {
    let n: u32 = env.storage().persistent().get(&key).unwrap_or(0);
    env.storage().persistent().set(&key, &n.saturating_add(1));
}

/// Cache for expensive computed views with TTL, invalidation, and monitoring.
pub struct ComputedViewCache;

impl ComputedViewCache {
    /// Store `value` under `cache_key` for `ttl_ledgers` ledgers.
    pub fn set_cached_view(env: &Env, cache_key: Symbol, value: Bytes, ttl_ledgers: u32) {
        let entry = CachedView {
            value,
            expires_at_ledger: env.ledger().sequence().saturating_add(ttl_ledgers),
        };
        env.storage().persistent().set(&cache_key, &entry);
    }

    /// Return the cached value on hit; `None` on miss or expiry.
    ///
    /// Expired entries are evicted. Hits and misses feed [`Self::cache_hit_rate`].
    pub fn get_cached_view(env: &Env, cache_key: Symbol) -> Option<Bytes> {
        let entry: Option<CachedView> = env.storage().persistent().get(&cache_key);
        match entry {
            Some(e) if env.ledger().sequence() < e.expires_at_ledger => {
                bump(env, hits_key(env));
                Some(e.value)
            }
            Some(_) => {
                env.storage().persistent().remove(&cache_key);
                bump(env, misses_key(env));
                None
            }
            None => {
                bump(env, misses_key(env));
                None
            }
        }
    }

    /// Manually invalidate a cached view.
    pub fn invalidate(env: &Env, cache_key: Symbol) {
        env.storage().persistent().remove(&cache_key);
    }

    /// Admin-only refresh: recompute off-chain, then store under `cache_key`.
    ///
    /// Returns `ContractError::AdminOnly` for non-admins.
    pub fn refresh_cache(
        env: &Env,
        admin: Address,
        cache_key: Symbol,
        value: Bytes,
        ttl_ledgers: u32,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        AdminManager::require_admin(env, &admin)?;
        Self::set_cached_view(env, cache_key, value, ttl_ledgers);
        Ok(())
    }

    /// (hits, misses) counters for monitoring.
    pub fn cache_stats(env: &Env) -> (u32, u32) {
        let hits: u32 = env.storage().persistent().get(&hits_key(env)).unwrap_or(0);
        let misses: u32 = env.storage().persistent().get(&misses_key(env)).unwrap_or(0);
        (hits, misses)
    }

    /// Cache hit rate as a percentage (0-100). Returns 100 when empty.
    pub fn cache_hit_rate(env: &Env) -> u32 {
        let (hits, misses) = Self::cache_stats(env);
        let total = hits.saturating_add(misses);
        if total == 0 {
            100
        } else {
            hits.saturating_mul(100) / total
        }
    }
}
