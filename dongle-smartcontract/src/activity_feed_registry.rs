//! Project activity feed / timeline (#759).
//!
//! Every on-chain action affecting a project (review submission, project
//! updates, verification state changes, ownership transfers, metadata updates)
//! is appended here so frontends and indexers can show a unified timeline.
//!
//! ## Storage layout
//!
//! `ActivityFeedKey::ProjectActivityFeed(project_id)` → `Vec<ActivityEntry>`
//!
//! The feed is capped at `MAX_FEED_ENTRIES_PER_PROJECT` (500) with the oldest
//! entry evicted when the cap is reached to bound ledger entry size.
//!
//! Pagination returns up to 100 items per page (`MAX_PAGE_LIMIT`).
//!
//! ## Filtering
//!
//! Callers pass an `Option<ActivityKind>` to `get_activity_feed` to filter by
//! activity type. `None` returns all kinds.

use crate::errors::ContractError;
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::ActivityFeedKey as AFK;
use crate::storage_manager::StorageManager;
use crate::types::{ActivityEntry, ActivityKind};
use soroban_sdk::{symbol_short, Address, Env, String, Vec};

/// Maximum number of activity entries retained per project.
pub const MAX_FEED_ENTRIES_PER_PROJECT: u32 = 500;

pub struct ActivityFeedRegistry;

impl ActivityFeedRegistry {
    // ── Internal ──────────────────────────────────────────────────────────

    fn load_feed(env: &Env, project_id: u64) -> Vec<ActivityEntry> {
        env.storage()
            .persistent()
            .get(&AFK::ProjectActivityFeed(project_id))
            .unwrap_or_else(|| Vec::new(env))
    }

    fn save_feed(env: &Env, project_id: u64, feed: &Vec<ActivityEntry>) {
        env.storage()
            .persistent()
            .set(&AFK::ProjectActivityFeed(project_id), feed);
        StorageManager::extend_if_exists_afk(env, &AFK::ProjectActivityFeed(project_id));
    }

    // ── Write (called from other registries/lib.rs) ───────────────────────

    /// Append an activity entry for a project. Evicts oldest entry if the feed
    /// is at capacity.
    pub fn record(
        env: &Env,
        project_id: u64,
        actor: Address,
        kind: ActivityKind,
        detail: Option<String>,
    ) {
        let mut feed = Self::load_feed(env, project_id);

        // Evict oldest entry when at capacity.
        if feed.len() >= MAX_FEED_ENTRIES_PER_PROJECT {
            let mut trimmed = Vec::new(env);
            for i in 1..feed.len() {
                if let Some(e) = feed.get(i) {
                    trimmed.push_back(e);
                }
            }
            feed = trimmed;
        }

        let now = env.ledger().timestamp();
        feed.push_back(ActivityEntry {
            project_id,
            actor,
            kind: kind.clone(),
            detail,
            timestamp: now,
        });

        Self::save_feed(env, project_id, &feed);

        env.events().publish(
            (symbol_short!("ACT_FEED"), project_id),
            (kind, now),
        );
    }

    // ── Read ──────────────────────────────────────────────────────────────

    /// Return a paginated page of the activity feed for a project.
    ///
    /// - `filter`: optional activity kind to restrict results to.
    /// - `offset`: zero-based index into the (optionally filtered) feed.
    /// - `limit`: maximum entries returned (capped at `MAX_PAGE_LIMIT` = 100).
    ///
    /// Results are ordered newest-first (descending timestamp).
    pub fn get_activity_feed(
        env: &Env,
        project_id: u64,
        filter: Option<ActivityKind>,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<ActivityEntry>, ContractError> {
        // Ensure project exists.
        ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        let feed = Self::load_feed(env, project_id);
        let clamped_limit = limit.min(crate::constants::MAX_PAGE_LIMIT);

        // Collect matching entries newest-first.
        let mut matched: Vec<ActivityEntry> = Vec::new(env);
        let len = feed.len();
        // Iterate in reverse (newest first).
        for idx in (0..len).rev() {
            if let Some(entry) = feed.get(idx) {
                let include = match &filter {
                    None => true,
                    Some(k) => entry.kind == *k,
                };
                if include {
                    matched.push_back(entry);
                }
            }
        }

        // Apply pagination.
        let mut result = Vec::new(env);
        let mut count = 0u32;
        let mlen = matched.len();
        let start = offset.min(mlen);
        for i in start..mlen {
            if count >= clamped_limit {
                break;
            }
            if let Some(e) = matched.get(i) {
                result.push_back(e);
                count += 1;
            }
        }

        Ok(result)
    }

    /// Return the total number of activity entries for a project (unfiltered).
    pub fn get_activity_count(env: &Env, project_id: u64) -> Result<u32, ContractError> {
        ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;
        Ok(Self::load_feed(env, project_id).len())
    }
}
