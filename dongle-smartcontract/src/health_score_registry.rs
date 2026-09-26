//! Project health score computation and history tracking (#756).
//!
//! ## Score formula (0–100)
//!
//! The score is composed of three components weighted equally at first but
//! configurable via `HealthScoreWeights`:
//!
//! | Component          | Input                             | Max pts |
//! |--------------------|-----------------------------------|---------|
//! | Rating             | Bayesian avg ÷ 5 × weight         |   40    |
//! | Activity           | Recent updates + reviews (capped) |   30    |
//! | Verification       | Status bonus                      |   30    |
//!
//! Historical snapshots are stored (up to `MAX_HEALTH_HISTORY_ENTRIES`) and
//! the breakdown is preserved alongside each snapshot for auditing.

use crate::errors::ContractError;
use crate::project_registry::ProjectRegistry;
use crate::review_registry::ReviewRegistry;
use crate::storage_keys::HealthScoreKey as HSK;
use crate::storage_manager::StorageManager;
use crate::types::{
    HealthScoreBreakdown, HealthScoreConfig, HealthScoreSnapshot, ProjectHealthScore,
    VerificationStatus,
};
use soroban_sdk::{symbol_short, Address, Env, Vec};

/// Maximum number of historical snapshots retained per project.
pub const MAX_HEALTH_HISTORY_ENTRIES: u32 = 365;

/// Default weights (must sum to 100).
pub const DEFAULT_RATING_WEIGHT: u32 = 40;
pub const DEFAULT_ACTIVITY_WEIGHT: u32 = 30;
pub const DEFAULT_VERIFICATION_WEIGHT: u32 = 30;

/// Maximum seconds of inactivity before full activity deduction (30 days).
const ACTIVITY_WINDOW_SECS: u64 = 30 * 24 * 3600;

pub struct HealthScoreRegistry;

impl HealthScoreRegistry {
    // ── Config helpers ────────────────────────────────────────────────────

    pub fn get_config(env: &Env) -> HealthScoreConfig {
        env.storage()
            .persistent()
            .get(&HSK::HealthScoreConfig)
            .unwrap_or(HealthScoreConfig {
                rating_weight: DEFAULT_RATING_WEIGHT,
                activity_weight: DEFAULT_ACTIVITY_WEIGHT,
                verification_weight: DEFAULT_VERIFICATION_WEIGHT,
                update_frequency_secs: 3600, // 1 hour default
            })
    }

    pub fn set_config(
        env: &Env,
        admin: &Address,
        config: HealthScoreConfig,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        crate::admin_manager::AdminManager::require_admin(env, admin)?;

        if config.rating_weight.saturating_add(config.activity_weight)
            .saturating_add(config.verification_weight) != 100
        {
            return Err(ContractError::InvalidInput);
        }

        env.storage().persistent().set(&HSK::HealthScoreConfig, &config);
        StorageManager::extend_if_exists_hsk(env, &HSK::HealthScoreConfig);
        Ok(())
    }

    // ── Score computation ─────────────────────────────────────────────────

    /// Compute and store a health score snapshot for a project.
    pub fn compute_and_store(
        env: &Env,
        project_id: u64,
    ) -> Result<ProjectHealthScore, ContractError> {
        let project = ProjectRegistry::get_project(env, project_id)
            .ok_or(ContractError::ProjectNotFound)?;
        let config = Self::get_config(env);

        // ── Rating component ──────────────────────────────────────────────
        let stats = ReviewRegistry::get_project_stats(env, project_id);
        // average_rating is scaled by 100 (e.g. 450 = 4.50 stars out of 5).
        // Normalise to 0–weight range.
        let rating_score = if stats.review_count == 0 {
            0u32
        } else {
            // (avg / 500) * weight, where avg is in [0, 500].
            (stats.average_rating.min(500) as u64 * config.rating_weight as u64 / 500) as u32
        };

        // ── Activity component ────────────────────────────────────────────
        let now = env.ledger().timestamp();
        let age_secs = now.saturating_sub(project.updated_at);
        let activity_score = if age_secs >= ACTIVITY_WINDOW_SECS {
            0u32
        } else {
            // Linear decay: more recent = higher score.
            let fraction = (ACTIVITY_WINDOW_SECS - age_secs) as u64;
            (fraction * config.activity_weight as u64 / ACTIVITY_WINDOW_SECS) as u32
        };

        // ── Verification component ────────────────────────────────────────
        let verification_score = match project.verification_status {
            VerificationStatus::Verified => config.verification_weight,
            VerificationStatus::Probationary => config.verification_weight * 80 / 100,
            VerificationStatus::Pending => config.verification_weight * 30 / 100,
            VerificationStatus::Rejected => config.verification_weight * 10 / 100,
            VerificationStatus::Suspended => config.verification_weight * 5 / 100,
            VerificationStatus::Unverified => 0,
        };

        let total_score = rating_score
            .saturating_add(activity_score)
            .saturating_add(verification_score)
            .min(100);

        let breakdown = HealthScoreBreakdown {
            rating_score,
            activity_score,
            verification_score,
            review_count: stats.review_count,
            average_rating: stats.average_rating,
            last_updated_at: project.updated_at,
            verification_status: project.verification_status,
        };

        let health = ProjectHealthScore {
            project_id,
            score: total_score,
            computed_at: now,
            breakdown: breakdown.clone(),
        };

        // Persist current score.
        env.storage()
            .persistent()
            .set(&HSK::ProjectHealthScore(project_id), &health);
        StorageManager::extend_if_exists_hsk(env, &HSK::ProjectHealthScore(project_id));

        // Append snapshot to history (evict oldest if at cap).
        let mut history: Vec<HealthScoreSnapshot> = env
            .storage()
            .persistent()
            .get(&HSK::ProjectHealthHistory(project_id))
            .unwrap_or_else(|| Vec::new(env));

        if history.len() >= MAX_HEALTH_HISTORY_ENTRIES {
            // Remove the oldest entry (index 0).
            let mut new_history = Vec::new(env);
            for i in 1..history.len() {
                if let Some(s) = history.get(i) {
                    new_history.push_back(s);
                }
            }
            history = new_history;
        }

        history.push_back(HealthScoreSnapshot {
            score: total_score,
            computed_at: now,
            breakdown,
        });

        env.storage()
            .persistent()
            .set(&HSK::ProjectHealthHistory(project_id), &history);
        StorageManager::extend_if_exists_hsk(env, &HSK::ProjectHealthHistory(project_id));

        env.events().publish(
            (symbol_short!("HLTH_SCR"), project_id),
            (total_score, now),
        );

        Ok(health)
    }

    /// Return the latest stored health score for a project (or compute if absent).
    pub fn get_health_score(
        env: &Env,
        project_id: u64,
    ) -> Result<ProjectHealthScore, ContractError> {
        // Ensure project exists.
        ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        match env
            .storage()
            .persistent()
            .get::<_, ProjectHealthScore>(&HSK::ProjectHealthScore(project_id))
        {
            Some(h) => Ok(h),
            None => Self::compute_and_store(env, project_id),
        }
    }

    /// Return paginated health score history for a project.
    ///
    /// Returns up to `limit` snapshots starting at `offset` (oldest-first).
    pub fn get_health_history(
        env: &Env,
        project_id: u64,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<HealthScoreSnapshot>, ContractError> {
        ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        let history: Vec<HealthScoreSnapshot> = env
            .storage()
            .persistent()
            .get(&HSK::ProjectHealthHistory(project_id))
            .unwrap_or_else(|| Vec::new(env));

        let clamped_limit = limit.min(crate::constants::MAX_PAGE_LIMIT);
        let mut result = Vec::new(env);
        let mut count = 0u32;
        let len = history.len();
        let start = offset.min(len);
        for i in start..len {
            if count >= clamped_limit {
                break;
            }
            if let Some(snap) = history.get(i) {
                result.push_back(snap);
                count += 1;
            }
        }
        Ok(result)
    }

    /// Admin: manually trigger a health score recomputation for a project.
    pub fn refresh_health_score(
        env: &Env,
        project_id: u64,
        caller: &Address,
    ) -> Result<ProjectHealthScore, ContractError> {
        caller.require_auth();
        crate::admin_manager::AdminManager::require_admin(env, caller)?;
        Self::compute_and_store(env, project_id)
    }
}
