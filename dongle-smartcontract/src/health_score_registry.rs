//! Project health score computation and history tracking (#982).
//!
//! ## Score formula (0–100)
//!
//! The composite score is composed of four weighted components:
//!
//! | Component          | Signal                                  | Default weight |
//! |--------------------|-----------------------------------------|---------------|
//! | Rating             | Bayesian avg ÷ 5 × weight               | 35             |
//! | Activity           | Recency of last update (linear decay)   | 25             |
//! | Verification       | Status bonus                            | 25             |
//! | Engagement         | Endorsements (capped) × weight          | 15             |
//!
//! Weights are admin-configurable via [`HealthScoreConfig`]; they must always
//! sum to 100. Historical snapshots are stored up to
//! [`MAX_HEALTH_HISTORY_ENTRIES`] per project.
//!
//! ## Decay
//!
//! The activity component linearly decays from `activity_weight` (full score
//! when just updated) to `0` once the project has been inactive for longer
//! than [`ACTIVITY_WINDOW_SECS`] (30 days). This causes the total score to
//! decay automatically over time for inactive projects.

use crate::endorsement_registry::EndorsementRegistry;
use crate::errors::ContractError;
use crate::events::publish_project_health_score_updated_event;
use crate::project_registry::ProjectRegistry;
use crate::review_registry::ReviewRegistry;
use crate::storage_keys::HealthScoreKey as HSK;
use crate::storage_manager::StorageManager;
use crate::types::{
    HealthScoreBreakdown, HealthScoreConfig, HealthScoreSnapshot, ProjectHealthScore,
    VerificationStatus,
};
use soroban_sdk::{Address, Env, Vec};

// ── Constants ─────────────────────────────────────────────────────────────────

/// Maximum number of historical snapshots retained per project.
pub const MAX_HEALTH_HISTORY_ENTRIES: u32 = 365;

/// Default weights (must sum to 100).
pub const DEFAULT_RATING_WEIGHT: u32 = 35;
pub const DEFAULT_ACTIVITY_WEIGHT: u32 = 25;
pub const DEFAULT_VERIFICATION_WEIGHT: u32 = 25;
pub const DEFAULT_ENGAGEMENT_WEIGHT: u32 = 15;

/// Maximum seconds of inactivity before full activity deduction (30 days).
const ACTIVITY_WINDOW_SECS: u64 = 30 * 24 * 3600;

/// Endorsement count at which the engagement component is saturated (full weight).
/// Projects with ≥ `ENGAGEMENT_SATURATION` endorsements receive the full engagement weight.
const ENGAGEMENT_SATURATION: u32 = 50;

// ── Registry ──────────────────────────────────────────────────────────────────

pub struct HealthScoreRegistry;

impl HealthScoreRegistry {
    // ── Config helpers ────────────────────────────────────────────────────────

    /// Return the current health score configuration (defaults applied when absent).
    pub fn get_config(env: &Env) -> HealthScoreConfig {
        env.storage()
            .persistent()
            .get(&HSK::HealthScoreConfig)
            .unwrap_or(HealthScoreConfig {
                rating_weight: DEFAULT_RATING_WEIGHT,
                activity_weight: DEFAULT_ACTIVITY_WEIGHT,
                verification_weight: DEFAULT_VERIFICATION_WEIGHT,
                engagement_weight: DEFAULT_ENGAGEMENT_WEIGHT,
                update_frequency_secs: 3600, // 1 hour default
            })
    }

    /// Admin: update the health score component weights and update frequency.
    ///
    /// `rating_weight + activity_weight + verification_weight + engagement_weight`
    /// must equal exactly 100.
    pub fn set_config(
        env: &Env,
        admin: &Address,
        config: HealthScoreConfig,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        crate::admin_manager::AdminManager::require_admin(env, admin)?;

        let weight_sum = config
            .rating_weight
            .saturating_add(config.activity_weight)
            .saturating_add(config.verification_weight)
            .saturating_add(config.engagement_weight);

        if weight_sum != 100 {
            return Err(ContractError::InvalidInput);
        }

        env.storage()
            .persistent()
            .set(&HSK::HealthScoreConfig, &config);
        StorageManager::extend_if_exists_hsk(env, &HSK::HealthScoreConfig);
        Ok(())
    }

    // ── Score computation ─────────────────────────────────────────────────────

    /// Compute and persist a health score snapshot for `project_id`.
    ///
    /// Emits a [`ProjectHealthScoreUpdatedEvent`].
    pub fn compute_and_store(
        env: &Env,
        project_id: u64,
    ) -> Result<ProjectHealthScore, ContractError> {
        let project = ProjectRegistry::get_project(env, project_id)
            .ok_or(ContractError::ProjectNotFound)?;
        let config = Self::get_config(env);

        // ── 1. Rating component ──────────────────────────────────────────────
        //
        // `average_rating` from ProjectStats is already scaled ×100
        // (e.g. 4.50 stars → 450). We normalise to [0, rating_weight].
        let stats = ReviewRegistry::get_project_stats(env, project_id);
        let rating_score = if stats.review_count == 0 {
            0u32
        } else {
            // average_rating ∈ [0, 500], weight ∈ [0, 100]
            (stats.average_rating.min(500) as u64
                * config.rating_weight as u64
                / 500) as u32
        };

        // ── 2. Activity component (time-decay) ───────────────────────────────
        //
        // Full weight when updated at the current ledger timestamp.
        // Linear decay to 0 after ACTIVITY_WINDOW_SECS (30 days) of inactivity.
        let now = env.ledger().timestamp();
        let age_secs = now.saturating_sub(project.updated_at);
        let activity_score = if age_secs >= ACTIVITY_WINDOW_SECS {
            0u32
        } else {
            let remaining = ACTIVITY_WINDOW_SECS - age_secs;
            (remaining * config.activity_weight as u64 / ACTIVITY_WINDOW_SECS) as u32
        };

        // ── 3. Verification component ────────────────────────────────────────
        let verification_score = match project.verification_status {
            VerificationStatus::Verified => config.verification_weight,
            VerificationStatus::Probationary => config.verification_weight * 80 / 100,
            VerificationStatus::Pending => config.verification_weight * 30 / 100,
            VerificationStatus::Rejected => config.verification_weight * 10 / 100,
            VerificationStatus::Suspended => config.verification_weight * 5 / 100,
            VerificationStatus::Unverified => 0,
        };

        // ── 4. Engagement component ──────────────────────────────────────────
        //
        // Community engagement is measured by endorsement count.
        // Score saturates at ENGAGEMENT_SATURATION endorsements.
        let endorsement_count =
            EndorsementRegistry::get_endorsement_count(env, project_id);
        let engagement_score = if config.engagement_weight == 0 {
            0u32
        } else {
            let capped = endorsement_count.min(ENGAGEMENT_SATURATION);
            (capped as u64 * config.engagement_weight as u64
                / ENGAGEMENT_SATURATION as u64) as u32
        };

        // ── Composite score ──────────────────────────────────────────────────
        let total_score = rating_score
            .saturating_add(activity_score)
            .saturating_add(verification_score)
            .saturating_add(engagement_score)
            .min(100);

        let breakdown = HealthScoreBreakdown {
            rating_score,
            activity_score,
            verification_score,
            engagement_score,
            review_count: stats.review_count,
            average_rating: stats.average_rating,
            last_updated_at: project.updated_at,
            verification_status: project.verification_status,
            endorsement_count,
        };

        let health = ProjectHealthScore {
            project_id,
            score: total_score,
            computed_at: now,
            breakdown: breakdown.clone(),
        };

        // ── Persist current score ────────────────────────────────────────────
        env.storage()
            .persistent()
            .set(&HSK::ProjectHealthScore(project_id), &health);
        StorageManager::extend_if_exists_hsk(env, &HSK::ProjectHealthScore(project_id));

        // ── Append to history (evict oldest when at cap) ─────────────────────
        let mut history: Vec<HealthScoreSnapshot> = env
            .storage()
            .persistent()
            .get(&HSK::ProjectHealthHistory(project_id))
            .unwrap_or_else(|| Vec::new(env));

        if history.len() >= MAX_HEALTH_HISTORY_ENTRIES {
            let mut trimmed = Vec::new(env);
            for i in 1..history.len() {
                if let Some(snap) = history.get(i) {
                    trimmed.push_back(snap);
                }
            }
            history = trimmed;
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

        // ── Emit typed event ─────────────────────────────────────────────────
        publish_project_health_score_updated_event(
            env,
            project_id,
            total_score,
            rating_score,
            activity_score,
            verification_score,
            engagement_score,
        );

        Ok(health)
    }

    /// Return the latest stored health score for a project, computing it on
    /// first access if no snapshot exists yet.
    pub fn get_health_score(
        env: &Env,
        project_id: u64,
    ) -> Result<ProjectHealthScore, ContractError> {
        // Verify the project exists first.
        ProjectRegistry::get_project(env, project_id)
            .ok_or(ContractError::ProjectNotFound)?;

        match env
            .storage()
            .persistent()
            .get::<_, ProjectHealthScore>(&HSK::ProjectHealthScore(project_id))
        {
            Some(h) => Ok(h),
            None => Self::compute_and_store(env, project_id),
        }
    }

    /// Return paginated health score history for a project (oldest-first).
    ///
    /// Returns up to `limit` snapshots starting at `offset`.
    pub fn get_health_history(
        env: &Env,
        project_id: u64,
        offset: u32,
        limit: u32,
    ) -> Result<Vec<HealthScoreSnapshot>, ContractError> {
        ProjectRegistry::get_project(env, project_id)
            .ok_or(ContractError::ProjectNotFound)?;

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
