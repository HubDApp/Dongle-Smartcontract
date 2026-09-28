use crate::constants::*;
use crate::errors::ContractError;
use crate::events::{
    publish_recommendation_analytics_snapshot_event, publish_recommendation_clicked_event,
    publish_recommendation_created_event, publish_recommendation_engagement_event,
    publish_recommendation_feedback_event, publish_recommendation_impression_event,
};
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::RecommendationKey as RK;
use crate::storage_manager::StorageManager;
use crate::types::{
    Recommendation, RecommendationAlgorithm, RecommendationAnalytics, RecommendationEngagementKind,
    RecommendationFeedback,
};
use soroban_sdk::{Address, Env, String, Vec};

const MAX_PAGE_LIMIT: u32 = crate::constants::MAX_PAGE_LIMIT;

pub struct RecommendationRegistry;

impl RecommendationRegistry {
    fn algorithm_to_u32(algo: RecommendationAlgorithm) -> u32 {
        match algo {
            RecommendationAlgorithm::Popular => 0,
            RecommendationAlgorithm::TopRated => 1,
            RecommendationAlgorithm::Similar => 2,
            RecommendationAlgorithm::Trending => 3,
            RecommendationAlgorithm::Featured => 4,
            RecommendationAlgorithm::Personalised => 5,
            RecommendationAlgorithm::Custom => 6,
        }
    }

    fn increment_counter(env: &Env, key: &RK) -> u64 {
        let current: u64 = env.storage().persistent().get(key).unwrap_or(0);
        let next = current.saturating_add(1);
        env.storage().persistent().set(key, &next);
        next
    }

    fn read_counter(env: &Env, key: &RK) -> u64 {
        env.storage().persistent().get(key).unwrap_or(0)
    }

    fn next_id(env: &Env) -> u64 {
        let current: u64 = env
            .storage()
            .persistent()
            .get(&RK::NextRecommendationId)
            .unwrap_or(0);
        let next = current.saturating_add(1);
        env.storage()
            .persistent()
            .set(&RK::NextRecommendationId, &next);
        StorageManager::extend_recommendation_global_ttl(env);
        next
    }

    fn validate_label_len(env: &Env, label: &Option<String>) -> Result<(), ContractError> {
        if let Some(lbl) = label {
            if lbl.len() as usize > MAX_RECOMMENDATION_LABEL_LEN {
                return Err(ContractError::RecommendationLabelTooLong);
            }
            let _ = env; // silence unused when label_len check above passes
        }
        Ok(())
    }

    fn validate_context(
        algorithm: RecommendationAlgorithm,
        reference_project_id: Option<u64>,
    ) -> Result<(), ContractError> {
        match algorithm {
            RecommendationAlgorithm::Similar => {
                if reference_project_id.is_none() {
                    return Err(ContractError::RecommendationInvalidContext);
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn check_audience(rec: &Recommendation, user: &Address) -> Result<(), ContractError> {
        if let Some(audience) = &rec.audience {
            if audience != user {
                return Err(ContractError::RecommendationAudienceMismatch);
            }
        }
        Ok(())
    }

    // ── Public CRUD ────────────────────────────────────────────────────────

    pub fn create_recommendation(
        env: &Env,
        creator: Address,
        target_project_id: u64,
        algorithm: RecommendationAlgorithm,
        reference_project_id: Option<u64>,
        audience: Option<Address>,
        score: Option<u64>,
        label: Option<String>,
    ) -> Result<u64, ContractError> {
        creator.require_auth();

        if ProjectRegistry::get_project(env, target_project_id).is_none() {
            return Err(ContractError::ProjectNotFound);
        }
        if let Some(ref_id) = reference_project_id {
            if ProjectRegistry::get_project(env, ref_id).is_none() {
                return Err(ContractError::ProjectNotFound);
            }
        }
        Self::validate_context(algorithm, reference_project_id)?;
        Self::validate_label_len(env, &label)?;

        let per_proj_len: u32 = env
            .storage()
            .persistent()
            .get::<_, Vec<u64>>(&RK::RecommendationsForProject(target_project_id))
            .map(|v| v.len())
            .unwrap_or(0);
        if per_proj_len >= MAX_RECOMMENDATIONS_PER_PROJECT {
            return Err(ContractError::InvalidInput);
        }

        let global_list: Vec<u64> = env
            .storage()
            .persistent()
            .get(&RK::RecommendationList)
            .unwrap_or_else(|| Vec::new(env));
        if global_list.len() >= MAX_RECOMMENDATIONS_GLOBAL {
            return Err(ContractError::InvalidInput);
        }

        let id = Self::next_id(env);
        let rec = Recommendation {
            id,
            target_project_id,
            algorithm,
            reference_project_id,
            audience: audience.clone(),
            score,
            label: label.clone(),
            created_at: env.ledger().timestamp(),
        };
        env.storage()
            .persistent()
            .set(&RK::Recommendation(id), &rec);

        let mut global = global_list;
        global.push_back(id);
        env.storage()
            .persistent()
            .set(&RK::RecommendationList, &global);

        let mut per_proj: Vec<u64> = env
            .storage()
            .persistent()
            .get(&RK::RecommendationsForProject(target_project_id))
            .unwrap_or_else(|| Vec::new(env));
        per_proj.push_back(id);
        env.storage()
            .persistent()
            .set(&RK::RecommendationsForProject(target_project_id), &per_proj);

        let algo_idx = Self::algorithm_to_u32(algorithm);
        let mut by_algo: Vec<u64> = env
            .storage()
            .persistent()
            .get(&RK::RecommendationsByAlgorithm(algo_idx))
            .unwrap_or_else(|| Vec::new(env));
        by_algo.push_back(id);
        env.storage()
            .persistent()
            .set(&RK::RecommendationsByAlgorithm(algo_idx), &by_algo);

        StorageManager::extend_recommendation_ttl(env, id);
        StorageManager::extend_recommendation_global_ttl(env);
        StorageManager::extend_recommendations_for_project_ttl(env, target_project_id);
        StorageManager::extend_project_ttl(env, target_project_id);
        if let Some(ref_id) = reference_project_id {
            StorageManager::extend_project_ttl(env, ref_id);
        }

        publish_recommendation_created_event(env, id, target_project_id, algorithm, creator);
        Ok(id)
    }

    pub fn get_recommendation(env: &Env, recommendation_id: u64) -> Option<Recommendation> {
        let rec = env
            .storage()
            .persistent()
            .get(&RK::Recommendation(recommendation_id));
        if rec.is_some() {
            StorageManager::extend_recommendation_ttl(env, recommendation_id);
        }
        rec
    }

    pub fn get_recommendation_count(env: &Env) -> u32 {
        env.storage()
            .persistent()
            .get::<_, Vec<u64>>(&RK::RecommendationList)
            .map(|v| v.len())
            .unwrap_or(0)
    }

    pub fn get_recommendation_count_for_project(env: &Env, target_project_id: u64) -> u32 {
        env.storage()
            .persistent()
            .get::<_, Vec<u64>>(&RK::RecommendationsForProject(target_project_id))
            .map(|v| v.len())
            .unwrap_or(0)
    }

    pub fn list_recommendations(env: &Env, start_index: u32, limit: u32) -> Vec<Recommendation> {
        let effective_limit = if limit == 0 || limit > MAX_PAGE_LIMIT {
            MAX_PAGE_LIMIT
        } else {
            limit
        };

        let ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&RK::RecommendationList)
            .unwrap_or_else(|| Vec::new(env));

        let len = ids.len();
        let mut page: Vec<Recommendation> = Vec::new(env);
        if start_index >= len {
            return page;
        }
        let end = core::cmp::min(start_index.saturating_add(effective_limit), len);
        for i in start_index..end {
            if let Some(id) = ids.get(i) {
                if let Some(rec) = Self::get_recommendation(env, id) {
                    page.push_back(rec);
                }
            }
        }
        page
    }

    pub fn list_recommendations_for_project(
        env: &Env,
        target_project_id: u64,
        start_index: u32,
        limit: u32,
    ) -> Vec<Recommendation> {
        let effective_limit = if limit == 0 || limit > MAX_PAGE_LIMIT {
            MAX_PAGE_LIMIT
        } else {
            limit
        };

        let ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&RK::RecommendationsForProject(target_project_id))
            .unwrap_or_else(|| Vec::new(env));

        let len = ids.len();
        let mut page: Vec<Recommendation> = Vec::new(env);
        if start_index >= len {
            return page;
        }
        let end = core::cmp::min(start_index.saturating_add(effective_limit), len);
        for i in start_index..end {
            if let Some(id) = ids.get(i) {
                if let Some(rec) = Self::get_recommendation(env, id) {
                    page.push_back(rec);
                }
            }
        }
        StorageManager::extend_recommendations_for_project_ttl(env, target_project_id);
        page
    }

    // ── Impression + Click tracking ────────────────────────────────────────

    pub fn record_impression(
        env: &Env,
        recommendation_id: u64,
        viewer: Address,
    ) -> Result<(), ContractError> {
        viewer.require_auth();

        let rec = Self::get_recommendation(env, recommendation_id)
            .ok_or(ContractError::RecommendationNotFound)?;
        Self::check_audience(&rec, &viewer)?;

        let seen_key = RK::ImpressionSeen(recommendation_id, viewer.clone());
        let already_seen: bool = env.storage().persistent().get(&seen_key).unwrap_or(false);
        if !already_seen {
            env.storage().persistent().set(&seen_key, &true);
            Self::increment_counter(env, &RK::ImpressionCount(recommendation_id));
        }

        publish_recommendation_impression_event(
            env,
            recommendation_id,
            rec.target_project_id,
            viewer,
        );
        Ok(())
    }

    pub fn record_click(
        env: &Env,
        recommendation_id: u64,
        viewer: Address,
    ) -> Result<(), ContractError> {
        viewer.require_auth();

        let rec = Self::get_recommendation(env, recommendation_id)
            .ok_or(ContractError::RecommendationNotFound)?;
        Self::check_audience(&rec, &viewer)?;

        let seen_key = RK::ImpressionSeen(recommendation_id, viewer.clone());
        let has_impression: bool = env.storage().persistent().get(&seen_key).unwrap_or(false);
        if !has_impression {
            return Err(ContractError::RecommendationNoImpression);
        }

        Self::increment_counter(env, &RK::ClickCount(recommendation_id));
        publish_recommendation_clicked_event(
            env,
            recommendation_id,
            rec.target_project_id,
            viewer.clone(),
        );
        publish_recommendation_engagement_event(
            env,
            recommendation_id,
            rec.target_project_id,
            viewer,
            RecommendationEngagementKind::Click,
        );
        Ok(())
    }

    pub fn record_engagement(
        env: &Env,
        recommendation_id: u64,
        user: Address,
        kind: RecommendationEngagementKind,
    ) -> Result<(), ContractError> {
        user.require_auth();

        let rec = Self::get_recommendation(env, recommendation_id)
            .ok_or(ContractError::RecommendationNotFound)?;
        Self::check_audience(&rec, &user)?;

        if matches!(kind, RecommendationEngagementKind::Impression) {
            return Self::record_impression(env, recommendation_id, user);
        }
        if matches!(kind, RecommendationEngagementKind::Click) {
            return Self::record_click(env, recommendation_id, user);
        }

        let counter_key = match kind {
            RecommendationEngagementKind::Follow => RK::FollowCount(recommendation_id),
            RecommendationEngagementKind::Bookmark => RK::BookmarkCount(recommendation_id),
            RecommendationEngagementKind::Endorse => RK::EndorseCount(recommendation_id),
            RecommendationEngagementKind::Review => RK::ReviewCount(recommendation_id),
            _ => return Err(ContractError::InvalidInput),
        };
        Self::increment_counter(env, &counter_key);

        publish_recommendation_engagement_event(
            env,
            recommendation_id,
            rec.target_project_id,
            user,
            kind,
        );
        Ok(())
    }

    // ── Thumbs up / down feedback ──────────────────────────────────────────

    pub fn give_feedback(
        env: &Env,
        recommendation_id: u64,
        user: Address,
        helpful: bool,
    ) -> Result<(), ContractError> {
        user.require_auth();

        let rec = Self::get_recommendation(env, recommendation_id)
            .ok_or(ContractError::RecommendationNotFound)?;
        Self::check_audience(&rec, &user)?;

        let fb_key = RK::Feedback(recommendation_id, user.clone());
        let existing: Option<RecommendationFeedback> = env.storage().persistent().get(&fb_key);
        if existing.is_some() {
            return Err(ContractError::RecommendationFeedbackAlreadyGiven);
        }

        let feedback = RecommendationFeedback {
            recommendation_id,
            user: user.clone(),
            helpful,
            created_at: env.ledger().timestamp(),
        };
        env.storage().persistent().set(&fb_key, &feedback);

        if helpful {
            Self::increment_counter(env, &RK::HelpfulCount(recommendation_id));
        } else {
            Self::increment_counter(env, &RK::NotHelpfulCount(recommendation_id));
        }

        publish_recommendation_feedback_event(
            env,
            recommendation_id,
            rec.target_project_id,
            user,
            helpful,
        );
        Ok(())
    }

    pub fn get_user_feedback(
        env: &Env,
        recommendation_id: u64,
        user: Address,
    ) -> Option<RecommendationFeedback> {
        env.storage()
            .persistent()
            .get(&RK::Feedback(recommendation_id, user))
    }

    // ── Analytics + Effectiveness Score ────────────────────────────────────

    pub fn compute_effectiveness_score_bps(
        impressions: u64,
        clicks: u64,
        helpful: u64,
        not_helpful: u64,
        follows: u64,
        bookmarks: u64,
        endorses: u64,
        reviews: u64,
    ) -> u32 {
        // CTR component (35% of total)
        let ctr_signal_ppm = if impressions >= MIN_IMPRESSIONS_FOR_CTR_SIGNAL {
            clicks
                .saturating_mul(RATIO_SCALE_PPM as u64)
                .checked_div(impressions)
                .unwrap_or(0)
        } else {
            0
        } as u128;

        // Helpful ratio component (35% of total)
        let total_feedback = helpful.saturating_add(not_helpful);
        let helpful_signal_ppm = if total_feedback >= MIN_FEEDBACK_FOR_HELPFUL_SIGNAL {
            helpful
                .saturating_mul(RATIO_SCALE_PPM as u64)
                .checked_div(total_feedback)
                .unwrap_or(0)
        } else {
            0
        } as u128;

        // Engagement per impression (30% of total). Normalise to PPM by
        // comparing (follows + bookmarks + endorses + reviews) to impressions,
        // capped at 1 full impression worth of signals.
        let engagement_total = follows
            .saturating_add(bookmarks)
            .saturating_add(endorses)
            .saturating_add(reviews);
        let engagement_signal_ppm = if impressions >= MIN_IMPRESSIONS_FOR_CTR_SIGNAL {
            let capped = core::cmp::min(engagement_total, impressions);
            capped
                .saturating_mul(RATIO_SCALE_PPM as u64)
                .checked_div(impressions)
                .unwrap_or(0)
        } else {
            0
        } as u128;

        let scale = RATIO_SCALE_PPM as u128;
        let w_ctr = EFFECTIVENESS_WEIGHT_CTR_BPS as u128;
        let w_help = EFFECTIVENESS_WEIGHT_HELPFUL_BPS as u128;
        let w_eng = EFFECTIVENESS_WEIGHT_ENGAGEMENT_BPS as u128;
        let score_scale = SCORE_SCALE_BPS as u128;

        // weighted average: sum(signal_i * weight_i / ppm) * score_scale / ppm
        // = sum(signal_i * weight_i) * score_scale / (ppm * ppm)
        let weighted_num =
            ctr_signal_ppm * w_ctr + helpful_signal_ppm * w_help + engagement_signal_ppm * w_eng;
        let denom = scale * scale;
        let score = weighted_num * score_scale / denom;
        core::cmp::min(score, score_scale) as u32
    }

    pub fn get_analytics(env: &Env, recommendation_id: u64) -> Option<RecommendationAnalytics> {
        let rec = Self::get_recommendation(env, recommendation_id)?;
        let impressions = Self::read_counter(env, &RK::ImpressionCount(recommendation_id));
        let clicks = Self::read_counter(env, &RK::ClickCount(recommendation_id));
        let helpful = Self::read_counter(env, &RK::HelpfulCount(recommendation_id));
        let not_helpful = Self::read_counter(env, &RK::NotHelpfulCount(recommendation_id));
        let follows = Self::read_counter(env, &RK::FollowCount(recommendation_id));
        let bookmarks = Self::read_counter(env, &RK::BookmarkCount(recommendation_id));
        let endorses = Self::read_counter(env, &RK::EndorseCount(recommendation_id));
        let reviews = Self::read_counter(env, &RK::ReviewCount(recommendation_id));

        let click_through_rate_ppm = if impressions == 0 {
            0
        } else {
            (clicks
                .saturating_mul(RATIO_SCALE_PPM as u64)
                .checked_div(impressions)
                .unwrap_or(0)) as u32
        };

        let total_fb = helpful.saturating_add(not_helpful);
        let helpful_ratio_ppm = if total_fb == 0 {
            0
        } else {
            (helpful
                .saturating_mul(RATIO_SCALE_PPM as u64)
                .checked_div(total_fb)
                .unwrap_or(0)) as u32
        };

        let effectiveness_score_bps = Self::compute_effectiveness_score_bps(
            impressions,
            clicks,
            helpful,
            not_helpful,
            follows,
            bookmarks,
            endorses,
            reviews,
        );

        publish_recommendation_analytics_snapshot_event(
            env,
            recommendation_id,
            rec.target_project_id,
            impressions,
            clicks,
            click_through_rate_ppm,
            helpful,
            not_helpful,
            helpful_ratio_ppm,
            effectiveness_score_bps,
        );

        Some(RecommendationAnalytics {
            recommendation_id,
            impressions,
            clicks,
            click_through_rate_ppm,
            helpful_count: helpful,
            not_helpful_count: not_helpful,
            helpful_ratio_ppm,
            follow_engagements: follows,
            bookmark_engagements: bookmarks,
            endorse_engagements: endorses,
            review_engagements: reviews,
            effectiveness_score_bps,
        })
    }

    /// Compute (id, score_bps) pairs over a slice of recommendation ids and
    /// return them in descending score order (highest effectiveness first).
    /// Ties are broken by recommendation id ascending (newest id last).
    pub fn list_sorted_by_effectiveness(
        env: &Env,
        start_index: u32,
        limit: u32,
    ) -> Vec<Recommendation> {
        let effective_limit = if limit == 0 || limit > MAX_PAGE_LIMIT {
            MAX_PAGE_LIMIT
        } else {
            limit
        };

        let ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&RK::RecommendationList)
            .unwrap_or_else(|| Vec::new(env));

        let ids_len = ids.len();
        let n = ids_len;
        let mut pairs: alloc::vec::Vec<(u64, u32)> = alloc::vec::Vec::with_capacity(n as usize);
        for i in 0..n {
            if let Some(id) = ids.get(i) {
                let impressions = Self::read_counter(env, &RK::ImpressionCount(id));
                let clicks = Self::read_counter(env, &RK::ClickCount(id));
                let helpful = Self::read_counter(env, &RK::HelpfulCount(id));
                let not_helpful = Self::read_counter(env, &RK::NotHelpfulCount(id));
                let follows = Self::read_counter(env, &RK::FollowCount(id));
                let bookmarks = Self::read_counter(env, &RK::BookmarkCount(id));
                let endorses = Self::read_counter(env, &RK::EndorseCount(id));
                let reviews = Self::read_counter(env, &RK::ReviewCount(id));
                let score = Self::compute_effectiveness_score_bps(
                    impressions,
                    clicks,
                    helpful,
                    not_helpful,
                    follows,
                    bookmarks,
                    endorses,
                    reviews,
                );
                pairs.push((id, score));
            }
        }
        pairs.sort_by(|a, b| {
            use core::cmp::Ordering;
            match b.1.cmp(&a.1) {
                Ordering::Equal => a.0.cmp(&b.0),
                o => o,
            }
        });

        let mut page: Vec<Recommendation> = Vec::new(env);
        let total = pairs.len() as u32;
        if start_index >= total {
            return page;
        }
        let end = core::cmp::min(start_index.saturating_add(effective_limit), total);
        for p in pairs
            .iter()
            .skip(start_index as usize)
            .take((end - start_index) as usize)
        {
            if let Some(rec) = Self::get_recommendation(env, p.0) {
                page.push_back(rec);
            }
        }
        page
    }
}
