//! Trending projects registry (#912).
//!
//! Calculates and exposes trending projects dynamically based on reviews,
//! follows, and registration recency/volume.
//! Supports 24h, 7d, 30d, and all-time windows, returning trending projects
//! with trend velocity, with lazy evaluation and cache recalculation.

use crate::constants::{
    MAX_PAGE_LIMIT, TRENDING_WINDOW_24H, TRENDING_WINDOW_30D, TRENDING_WINDOW_7D,
    TRENDING_WINDOW_ALL_TIME,
};
use crate::events::publish_trending_recalculated_event;
use crate::project_controls::ProjectControls;
use crate::project_registry::ProjectRegistry;
use crate::review_registry::ReviewRegistry;
use crate::storage_keys::{StorageKey, TrendingKey};
use crate::subscription_registry::SubscriptionRegistry;
use crate::types::{Project, TrendingProject};
use soroban_sdk::{Env, Vec};

/// Cache TTL for precomputed trending calculations (1 hour in seconds).
pub const TRENDING_CACHE_TTL_SECS: u64 = 3600;

pub struct TrendingRegistry;

impl TrendingRegistry {
    /// Get trending projects for a specified time window (24h, 7d, 30d, or 0 for all-time).
    /// Uses cached results if fresh, or lazily evaluates dynamically.
    pub fn get_trending_projects(
        env: &Env,
        time_window: u64,
        limit: u32,
    ) -> Vec<TrendingProject> {
        let effective_limit = if limit == 0 || limit > MAX_PAGE_LIMIT {
            MAX_PAGE_LIMIT
        } else {
            limit
        };

        let now = env.ledger().timestamp();
        let cached_ts: u64 = env
            .storage()
            .persistent()
            .get(&TrendingKey::TrendingCacheTimestamp(time_window))
            .unwrap_or(0);

        let list: Vec<TrendingProject> = if now.saturating_sub(cached_ts) < TRENDING_CACHE_TTL_SECS
        {
            if let Some(cached) = env
                .storage()
                .persistent()
                .get::<_, Vec<TrendingProject>>(&TrendingKey::TrendingCache(time_window))
            {
                cached
            } else {
                Self::recalculate_trending(env, time_window)
            }
        } else {
            Self::recalculate_trending(env, time_window)
        };

        let total = list.len();
        let end = core::cmp::min(effective_limit, total);
        let mut page = Vec::new(env);
        for i in 0..end {
            if let Some(item) = list.get(i) {
                page.push_back(item);
            }
        }
        page
    }

    /// Recalculates trending projects for the given time window, updates storage
    /// cache, and returns the full ranked list. Suitable for background cron jobs.
    pub fn recalculate_trending(env: &Env, time_window: u64) -> Vec<TrendingProject> {
        let count: u64 = env
            .storage()
            .persistent()
            .get(&StorageKey::ProjectCount)
            .unwrap_or(0);

        let mut candidates: Vec<TrendingProject> = Vec::new(env);

        for id in 1..=count {
            let Some(project) = ProjectRegistry::get_project(env, id) else {
                continue;
            };
            if project.archived {
                continue;
            }
            if ProjectControls::is_quarantined(env, id) {
                continue;
            }

            let (score, velocity) = Self::compute_score_and_velocity(env, &project, time_window);

            candidates.push_back(TrendingProject {
                project,
                trending_score: score,
                trend_velocity: velocity,
            });
        }

        // Rank candidates by trending_score descending (and lower project.id on tie)
        let total = candidates.len();
        let mut sorted = Vec::new(env);
        let mut taken = Vec::new(env);
        for _ in 0..total {
            taken.push_back(false);
        }

        for _ in 0..total {
            let mut best_idx: Option<u32> = None;
            for i in 0..total {
                if taken.get(i).unwrap_or(false) {
                    continue;
                }
                let Some(curr_best) = best_idx else {
                    best_idx = Some(i);
                    continue;
                };

                let item_i = candidates.get(i).unwrap();
                let item_best = candidates.get(curr_best).unwrap();

                if item_i.trending_score > item_best.trending_score
                    || (item_i.trending_score == item_best.trending_score
                        && item_i.project.id < item_best.project.id)
                {
                    best_idx = Some(i);
                }
            }

            let Some(chosen) = best_idx else {
                break;
            };
            taken.set(chosen, true);
            if let Some(item) = candidates.get(chosen) {
                sorted.push_back(item);
            }
        }

        let now = env.ledger().timestamp();
        env.storage()
            .persistent()
            .set(&TrendingKey::TrendingCache(time_window), &sorted);
        env.storage()
            .persistent()
            .set(&TrendingKey::TrendingCacheTimestamp(time_window), &now);

        publish_trending_recalculated_event(env, time_window, sorted.len());

        sorted
    }

    /// Compute dynamic trending score and engagement velocity for a project.
    pub fn compute_score_and_velocity(
        env: &Env,
        project: &Project,
        time_window: u64,
    ) -> (u64, u32) {
        let now = env.ledger().timestamp();
        let age = now.saturating_sub(project.created_at);

        // 1. Recency / registration volume score
        let recency_score: u64 = if time_window > 0 {
            if age <= time_window {
                1000u64.saturating_mul(time_window - age) / time_window
            } else {
                0
            }
        } else {
            // All-time window: decays over a 30-day baseline
            if age < TRENDING_WINDOW_30D {
                1000u64.saturating_mul(TRENDING_WINDOW_30D - age) / TRENDING_WINDOW_30D
            } else {
                0
            }
        };

        // 2. Follows (subscriptions) score
        let follower_count = SubscriptionRegistry::get_follower_count(env, project.id) as u64;
        let follows_score = follower_count.saturating_mul(50);

        // 3. Reviews score: count and average rating
        let stats = ReviewRegistry::get_project_stats(env, project.id);
        let review_count = stats.total_reviews as u64;
        let avg_rating = stats.average_rating as u64;
        let reviews_score = review_count.saturating_mul(100) + (avg_rating / 10);

        // Total trending score
        let activity_score = follows_score.saturating_add(reviews_score);
        let trending_score = recency_score.saturating_mul(2).saturating_add(activity_score);

        // 4. Trend velocity: engagement activity per standard day (86,400s)
        let trend_velocity: u32 = if time_window > 0 {
            let daily_rate = activity_score.saturating_mul(TRENDING_WINDOW_24H) / time_window;
            daily_rate as u32
        } else {
            let days = core::cmp::max(1, age / TRENDING_WINDOW_24H);
            (activity_score / days) as u32
        };

        (trending_score, trend_velocity)
    }
}
