//! On-chain project recommendations, personalization, A/B assignment, and
//! conversion analytics (issue #749).

use crate::auth::require_admin_auth;
use crate::constants::*;
use crate::dependency_registry::DependencyRegistry;
use crate::errors::ContractError;
use crate::events::{
    publish_project_view_recorded_event, publish_recommendation_ab_config_event,
    publish_recommendation_analytics_snapshot_event, publish_recommendation_clicked_event,
    publish_recommendation_conversion_event, publish_recommendation_created_event,
    publish_recommendation_engagement_event, publish_recommendation_feedback_event,
    publish_recommendation_impression_event,
};
use crate::project_registry::ProjectRegistry;
use crate::review_registry::ReviewRegistry;
use crate::storage_keys::{RecommendationKey as RK, StorageKey};
use crate::storage_manager::StorageManager;
use crate::types::{
    Project, ProjectView, Recommendation, RecommendationABAnalytics, RecommendationABConfig,
    RecommendationAlgorithm, RecommendationAnalytics, RecommendationEngagementKind,
    RecommendationFeedback, RecommendationVariant,
};
use soroban_sdk::xdr::ToXdr;
use soroban_sdk::{Address, Env, String, Vec};

const DEFAULT_RECOMMENDATION_LIMIT: u32 = 10;
const RECENT_TRENDING_WINDOW_SECS: u64 = 30 * 24 * 60 * 60;

pub struct RecommendationRegistry;

type VariantTotals = (u64, u64, u64);

impl RecommendationRegistry {
    fn algorithm_to_u32(algorithm: RecommendationAlgorithm) -> u32 {
        match algorithm {
            RecommendationAlgorithm::Popular => 0,
            RecommendationAlgorithm::TopRated => 1,
            RecommendationAlgorithm::Similar => 2,
            RecommendationAlgorithm::Trending => 3,
            RecommendationAlgorithm::Featured => 4,
            RecommendationAlgorithm::Personalised => 5,
            RecommendationAlgorithm::Custom => 6,
        }
    }

    fn algorithm_label(env: &Env, algorithm: RecommendationAlgorithm) -> String {
        let label = match algorithm {
            RecommendationAlgorithm::Popular => "popular",
            RecommendationAlgorithm::TopRated => "top-rated",
            RecommendationAlgorithm::Similar => "similar",
            RecommendationAlgorithm::Trending => "trending",
            RecommendationAlgorithm::Featured => "featured",
            RecommendationAlgorithm::Personalised => "personalised",
            RecommendationAlgorithm::Custom => "custom",
        };
        String::from_str(env, label)
    }

    fn default_ab_config(env: &Env) -> RecommendationABConfig {
        RecommendationABConfig {
            enabled: false,
            split_bps: DEFAULT_RECOMMENDATION_AB_SPLIT_BPS,
            variant_a: RecommendationAlgorithm::Popular,
            variant_b: RecommendationAlgorithm::Similar,
            description: String::from_str(env, "Default disabled experiment"),
        }
    }

    pub fn get_ab_config(env: &Env) -> RecommendationABConfig {
        let config = env
            .storage()
            .persistent()
            .get(&RK::ABConfig)
            .unwrap_or_else(|| Self::default_ab_config(env));
        if env.storage().persistent().has(&RK::ABConfig) {
            StorageManager::extend_recommendation_ab_ttl(env);
        }
        config
    }

    /// Configure a recommendation algorithm experiment. Variant B receives
    /// `split_bps` basis points of deterministic viewer assignments.
    pub fn set_ab_config(
        env: &Env,
        admin: Address,
        config: RecommendationABConfig,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;
        if config.split_bps > 10_000
            || config.description.is_empty()
            || config.description.len() as usize > MAX_AB_TEST_DESC_LEN
            || config.variant_a == RecommendationAlgorithm::Custom
            || config.variant_b == RecommendationAlgorithm::Custom
            || config.variant_a == config.variant_b
        {
            return Err(ContractError::InvalidInput);
        }

        // A configuration change starts a fresh experiment window.
        env.storage()
            .persistent()
            .set(&RK::VariantAAnalytics, &(0u64, 0u64, 0u64));
        env.storage()
            .persistent()
            .set(&RK::VariantBAnalytics, &(0u64, 0u64, 0u64));
        env.storage().persistent().set(&RK::ABConfig, &config);
        StorageManager::extend_recommendation_ab_ttl(env);
        publish_recommendation_ab_config_event(env, &config, admin);
        Ok(())
    }

    fn assignment_bucket(env: &Env, viewer: &Address) -> u32 {
        let bytes = viewer.clone().to_xdr(env);
        let mut value = 0u64;
        let take = core::cmp::min(bytes.len(), 8);
        for index in 0..take {
            value = (value << 8) | u64::from(bytes.get(index).unwrap_or(0));
        }
        (value % 10_000) as u32
    }

    /// Return the viewer's stable experiment arm. Assignment does not require
    /// authentication because it only reads public experiment configuration.
    pub fn get_variant(env: &Env, viewer: Address) -> RecommendationVariant {
        let config = Self::get_ab_config(env);
        if config.enabled && Self::assignment_bucket(env, &viewer) < config.split_bps {
            RecommendationVariant::VariantB
        } else {
            RecommendationVariant::VariantA
        }
    }

    fn configured_algorithm(
        env: &Env,
        viewer: &Address,
    ) -> (RecommendationVariant, RecommendationAlgorithm) {
        let config = Self::get_ab_config(env);
        let variant = Self::get_variant(env, viewer.clone());
        let algorithm = match variant {
            RecommendationVariant::VariantA => config.variant_a,
            RecommendationVariant::VariantB => config.variant_b,
        };
        (variant, algorithm)
    }

    fn extend_user_key_ttl(env: &Env, key: &RK) {
        if env.storage().persistent().has(key) {
            env.storage()
                .persistent()
                .extend_ttl(key, LEDGER_THRESHOLD_USER, LEDGER_BUMP_USER);
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

    fn validate_label_len(label: &Option<String>) -> Result<(), ContractError> {
        if let Some(value) = label {
            if value.len() as usize > MAX_RECOMMENDATION_LABEL_LEN {
                return Err(ContractError::RecommendationLabelTooLong);
            }
        }
        Ok(())
    }

    fn validate_context(
        algorithm: RecommendationAlgorithm,
        reference_project_id: Option<u64>,
    ) -> Result<(), ContractError> {
        if algorithm == RecommendationAlgorithm::Similar && reference_project_id.is_none() {
            return Err(ContractError::RecommendationInvalidContext);
        }
        Ok(())
    }

    fn check_audience(
        recommendation: &Recommendation,
        user: &Address,
    ) -> Result<(), ContractError> {
        if let Some(audience) = &recommendation.audience {
            if audience != user {
                return Err(ContractError::RecommendationAudienceMismatch);
            }
        }
        Ok(())
    }

    fn create_record(
        env: &Env,
        creator: Address,
        target_project_id: u64,
        algorithm: RecommendationAlgorithm,
        reference_project_id: Option<u64>,
        audience: Option<Address>,
        score: Option<u64>,
        label: Option<String>,
    ) -> Result<u64, ContractError> {
        if ProjectRegistry::get_project(env, target_project_id).is_none() {
            return Err(ContractError::ProjectNotFound);
        }
        if let Some(reference_id) = reference_project_id {
            if ProjectRegistry::get_project(env, reference_id).is_none() {
                return Err(ContractError::ProjectNotFound);
            }
        }
        Self::validate_context(algorithm, reference_project_id)?;
        Self::validate_label_len(&label)?;

        let per_project_len: u32 = env
            .storage()
            .persistent()
            .get::<_, Vec<u64>>(&RK::RecommendationsForProject(target_project_id))
            .map(|values| values.len())
            .unwrap_or(0);
        if per_project_len >= MAX_RECOMMENDATIONS_PER_PROJECT {
            return Err(ContractError::InvalidInput);
        }

        let mut global_list: Vec<u64> = env
            .storage()
            .persistent()
            .get(&RK::RecommendationList)
            .unwrap_or_else(|| Vec::new(env));
        if global_list.len() >= MAX_RECOMMENDATIONS_GLOBAL {
            return Err(ContractError::InvalidInput);
        }

        let assignment_viewer = audience.clone().unwrap_or_else(|| creator.clone());
        let (variant, _) = Self::configured_algorithm(env, &assignment_viewer);
        let id = Self::next_id(env);
        let recommendation = Recommendation {
            id,
            target_project_id,
            algorithm,
            reference_project_id,
            audience: audience.clone(),
            score,
            label,
            created_at: env.ledger().timestamp(),
            variant,
        };
        env.storage()
            .persistent()
            .set(&RK::Recommendation(id), &recommendation);

        global_list.push_back(id);
        env.storage()
            .persistent()
            .set(&RK::RecommendationList, &global_list);

        let mut per_project: Vec<u64> = env
            .storage()
            .persistent()
            .get(&RK::RecommendationsForProject(target_project_id))
            .unwrap_or_else(|| Vec::new(env));
        per_project.push_back(id);
        env.storage().persistent().set(
            &RK::RecommendationsForProject(target_project_id),
            &per_project,
        );

        let mut by_algorithm: Vec<u64> = env
            .storage()
            .persistent()
            .get(&RK::RecommendationsByAlgorithm(Self::algorithm_to_u32(
                algorithm,
            )))
            .unwrap_or_else(|| Vec::new(env));
        by_algorithm.push_back(id);
        env.storage().persistent().set(
            &RK::RecommendationsByAlgorithm(Self::algorithm_to_u32(algorithm)),
            &by_algorithm,
        );

        StorageManager::extend_recommendation_ttl(env, id);
        StorageManager::extend_recommendation_global_ttl(env);
        StorageManager::extend_recommendations_for_project_ttl(env, target_project_id);
        StorageManager::extend_project_ttl(env, target_project_id);
        if let Some(reference_id) = reference_project_id {
            StorageManager::extend_project_ttl(env, reference_id);
        }
        publish_recommendation_created_event(env, id, target_project_id, algorithm, creator);
        Ok(id)
    }

    /// Create a manually curated recommendation. The creator authenticates and
    /// optional personalization scope is enforced on all funnel events.
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
        Self::create_record(
            env,
            creator,
            target_project_id,
            algorithm,
            reference_project_id,
            audience,
            score,
            label,
        )
    }

    pub fn get_recommendation(env: &Env, recommendation_id: u64) -> Option<Recommendation> {
        let recommendation = env
            .storage()
            .persistent()
            .get(&RK::Recommendation(recommendation_id));
        if recommendation.is_some() {
            StorageManager::extend_recommendation_ttl(env, recommendation_id);
        }
        recommendation
    }

    pub fn get_recommendation_count(env: &Env) -> u32 {
        env.storage()
            .persistent()
            .get::<_, Vec<u64>>(&RK::RecommendationList)
            .map(|values| values.len())
            .unwrap_or(0)
    }

    pub fn get_recommendation_count_for_project(env: &Env, target_project_id: u64) -> u32 {
        env.storage()
            .persistent()
            .get::<_, Vec<u64>>(&RK::RecommendationsForProject(target_project_id))
            .map(|values| values.len())
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
        if start_index >= ids.len() {
            return Vec::new(env);
        }
        let end = core::cmp::min(start_index.saturating_add(effective_limit), ids.len());
        let mut page = Vec::new(env);
        for index in start_index..end {
            if let Some(id) = ids.get(index) {
                if let Some(recommendation) = Self::get_recommendation(env, id) {
                    page.push_back(recommendation);
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
        if start_index >= ids.len() {
            return Vec::new(env);
        }
        let end = core::cmp::min(start_index.saturating_add(effective_limit), ids.len());
        let mut page = Vec::new(env);
        for index in start_index..end {
            if let Some(id) = ids.get(index) {
                if let Some(recommendation) = Self::get_recommendation(env, id) {
                    page.push_back(recommendation);
                }
            }
        }
        StorageManager::extend_recommendations_for_project_ttl(env, target_project_id);
        page
    }

    // ── Viewing history and personalized generation ──────────────────────────

    /// Record a unique project view in a bounded per-viewer history. Repeated
    /// views refresh the timestamp rather than consuming additional slots.
    pub fn record_project_view(
        env: &Env,
        viewer: Address,
        project_id: u64,
    ) -> Result<(), ContractError> {
        viewer.require_auth();
        if ProjectRegistry::get_project(env, project_id).is_none() {
            return Err(ContractError::ProjectNotFound);
        }
        let mut history: Vec<ProjectView> = env
            .storage()
            .persistent()
            .get(&RK::ViewHistory(viewer.clone()))
            .unwrap_or_else(|| Vec::new(env));
        let view = ProjectView {
            project_id,
            viewed_at: env.ledger().timestamp(),
        };

        let mut found = false;
        for index in 0..history.len() {
            if let Some(existing) = history.get(index) {
                if existing.project_id == project_id {
                    history.set(index, view.clone());
                    found = true;
                    break;
                }
            }
        }
        if !found {
            if history.len() >= MAX_RECOMMENDATION_VIEW_HISTORY {
                history.remove(0);
            }
            history.push_back(view);
        }

        env.storage()
            .persistent()
            .set(&RK::ViewHistory(viewer.clone()), &history);
        StorageManager::extend_recommendation_view_history_ttl(env, &viewer);
        publish_project_view_recorded_event(env, viewer, project_id);
        Ok(())
    }

    pub fn get_view_history(env: &Env, viewer: &Address) -> Vec<ProjectView> {
        let history: Vec<ProjectView> = env
            .storage()
            .persistent()
            .get(&RK::ViewHistory(viewer.clone()))
            .unwrap_or_else(|| Vec::new(env));
        if !history.is_empty() {
            StorageManager::extend_recommendation_view_history_ttl(env, viewer);
        }
        history
    }

    fn push_candidate(_env: &Env, candidates: &mut Vec<u64>, project_id: u64) {
        if project_id != 0
            && !candidates.contains(&project_id)
            && candidates.len() < MAX_RECOMMENDATION_CANDIDATES
        {
            candidates.push_back(project_id);
        }
    }

    fn has_dependency(env: &Env, project_id: u64, dependency_id: u64) -> bool {
        for dependency in DependencyRegistry::get_dependencies(env, project_id).iter() {
            if dependency.reference.project_id == Some(dependency_id) {
                return true;
            }
        }
        false
    }

    fn shared_tag_count(left: &Project, right: &Project) -> u32 {
        if let Some(left_tags) = &left.tags {
            if let Some(right_tags) = &right.tags {
                let mut count = 0u32;
                for tag in left_tags.iter() {
                    if right_tags.contains(&tag) {
                        count = count.saturating_add(1);
                    }
                }
                count
            } else {
                0
            }
        } else {
            0
        }
    }

    fn collect_candidates(env: &Env, source: &Project, viewer: &Address) -> Vec<u64> {
        let mut candidates = Vec::new(env);

        // Direct dependencies are the strongest explicit relationship, so add
        // them before broader category/tag indexes consume the candidate cap.
        for dependency in DependencyRegistry::get_dependencies(env, source.id).iter() {
            if let Some(project_id) = dependency.reference.project_id {
                Self::push_candidate(env, &mut candidates, project_id);
            }
        }
        let category_ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::CategoryProjects(source.category.clone()))
            .unwrap_or_else(|| Vec::new(env));
        for project_id in category_ids.iter() {
            Self::push_candidate(env, &mut candidates, project_id);
        }
        if let Some(tags) = &source.tags {
            for tag in tags.iter() {
                for project_id in ProjectRegistry::get_tag_index(env, &tag).iter() {
                    Self::push_candidate(env, &mut candidates, project_id);
                }
            }
        }

        // Reverse dependencies are not indexed; scan a bounded ID prefix.
        let count = env
            .storage()
            .persistent()
            .get(&StorageKey::ProjectCount)
            .unwrap_or(0);
        for project_id in 1..=core::cmp::min(count, MAX_RECOMMENDATION_SCAN) {
            if Self::has_dependency(env, project_id, source.id) {
                Self::push_candidate(env, &mut candidates, project_id);
            }
        }

        for view in Self::get_view_history(env, viewer).iter() {
            if view.project_id == source.id {
                continue;
            }
            if let Some(historical) = ProjectRegistry::get_project(env, view.project_id) {
                let historical_category: Vec<u64> = env
                    .storage()
                    .persistent()
                    .get(&StorageKey::CategoryProjects(historical.category.clone()))
                    .unwrap_or_else(|| Vec::new(env));
                for project_id in historical_category.iter() {
                    Self::push_candidate(env, &mut candidates, project_id);
                }
                if let Some(tags) = &historical.tags {
                    for tag in tags.iter() {
                        for project_id in ProjectRegistry::get_tag_index(env, &tag).iter() {
                            Self::push_candidate(env, &mut candidates, project_id);
                        }
                    }
                }
            }
        }
        candidates
    }

    fn base_score(
        env: &Env,
        algorithm: RecommendationAlgorithm,
        source: &Project,
        candidate: &Project,
        dependency_related: bool,
    ) -> u64 {
        let tags = Self::shared_tag_count(source, candidate);
        let category = if source.category == candidate.category {
            1u64
        } else {
            0
        };
        let dependency = if dependency_related { 1u64 } else { 0 };
        let rating = ReviewRegistry::get_project_stats(env, candidate.id).average_rating as u64;
        let mut score = match algorithm {
            RecommendationAlgorithm::Popular => {
                category * 1_000 + tags as u64 * 150 + dependency * 2_000 + rating * 2
            }
            RecommendationAlgorithm::TopRated => {
                category * 500 + tags as u64 * 100 + dependency * 1_500 + rating * 7
            }
            RecommendationAlgorithm::Similar => {
                category * 2_500 + tags as u64 * 400 + dependency * 3_500 + rating
            }
            RecommendationAlgorithm::Trending => {
                let age = env
                    .ledger()
                    .timestamp()
                    .saturating_sub(candidate.created_at);
                let recency = if age <= RECENT_TRENDING_WINDOW_SECS {
                    1_000
                } else {
                    0
                };
                category * 1_000 + tags as u64 * 150 + dependency * 2_500 + rating * 2 + recency
            }
            RecommendationAlgorithm::Featured => {
                let featured: Vec<u64> = env
                    .storage()
                    .persistent()
                    .get(&StorageKey::FeaturedProjects)
                    .unwrap_or_else(|| Vec::new(env));
                let featured_bonus = if featured.contains(&candidate.id) {
                    2_000
                } else {
                    0
                };
                category * 500
                    + tags as u64 * 100
                    + dependency * 1_500
                    + rating * 4
                    + featured_bonus
            }
            RecommendationAlgorithm::Personalised => {
                category * 1_000 + tags as u64 * 150 + dependency * 2_000 + rating * 2
            }
            RecommendationAlgorithm::Custom => {
                category * 1_500 + tags as u64 * 250 + dependency * 2_500 + rating * 2
            }
        };
        if score > 0 {
            score = score.saturating_add(1);
        }
        score
    }

    fn personalization_score(
        env: &Env,
        source: &Project,
        candidate: &Project,
        viewer: &Address,
    ) -> i64 {
        let history = Self::get_view_history(env, viewer);
        let mut category_boost = 0i64;
        let mut tag_boost = 0i64;
        let mut dependency_boost = 0i64;
        let mut seen_penalty = 0i64;

        for view in history.iter() {
            if view.project_id == candidate.id {
                seen_penalty = 1_000;
            }
            if view.project_id == source.id {
                continue;
            }
            if let Some(historical) = ProjectRegistry::get_project(env, view.project_id) {
                if historical.category == candidate.category {
                    category_boost = core::cmp::min(category_boost + 300, 1_200);
                }
                let shared_tags = Self::shared_tag_count(&historical, candidate) as i64;
                tag_boost = core::cmp::min(tag_boost + shared_tags * 150, 900);
                if Self::has_dependency(env, historical.id, candidate.id) {
                    dependency_boost = core::cmp::min(dependency_boost + 400, 800);
                }
            }
        }
        category_boost + tag_boost + dependency_boost - seen_penalty
    }

    /// Generate bounded, personalized recommendations from a source project.
    /// Candidate scoring uses category, tags, direct/reverse dependencies, and
    /// Bayesian rating, then boosts candidates related to the viewer's history.
    pub fn recommend_projects(
        env: &Env,
        viewer: Address,
        source_project_id: u64,
        limit: u32,
    ) -> Result<Vec<Recommendation>, ContractError> {
        viewer.require_auth();
        let source = ProjectRegistry::get_project(env, source_project_id)
            .ok_or(ContractError::ProjectNotFound)?;
        let effective_limit = if limit == 0 {
            DEFAULT_RECOMMENDATION_LIMIT
        } else {
            core::cmp::min(limit, MAX_RECOMMENDATION_CANDIDATES)
        };
        let (_, algorithm) = Self::configured_algorithm(env, &viewer);
        let candidates = Self::collect_candidates(env, &source, &viewer);
        let mut scored: alloc::vec::Vec<(u64, u64)> =
            alloc::vec::Vec::with_capacity(candidates.len() as usize);

        for candidate_id in candidates.iter() {
            if candidate_id == source.id {
                continue;
            }
            if let Some(candidate) = ProjectRegistry::get_project(env, candidate_id) {
                if candidate.archived {
                    continue;
                }
                let dependency_related = Self::has_dependency(env, source.id, candidate.id)
                    || Self::has_dependency(env, candidate.id, source.id);
                let base =
                    Self::base_score(env, algorithm, &source, &candidate, dependency_related);
                let personalization =
                    Self::personalization_score(env, &source, &candidate, &viewer);
                let score = if personalization >= 0 {
                    base.saturating_add(personalization as u64)
                } else {
                    base.saturating_sub(personalization.unsigned_abs())
                };
                if score > 0 {
                    scored.push((candidate_id, score));
                }
            }
        }

        scored.sort_by(|left, right| {
            use core::cmp::Ordering;
            match right.1.cmp(&left.1) {
                Ordering::Equal => left.0.cmp(&right.0),
                ordering => ordering,
            }
        });

        let mut recommendations = Vec::new(env);
        for (candidate_id, score) in scored.iter().take(effective_limit as usize) {
            let dedupe_key = RK::PersonalizedRecommendation(
                viewer.clone(),
                source_project_id,
                *candidate_id,
                Self::algorithm_to_u32(algorithm),
            );
            if let Some(recommendation_id) = env.storage().persistent().get::<_, u64>(&dedupe_key) {
                if let Some(recommendation) = Self::get_recommendation(env, recommendation_id) {
                    recommendations.push_back(recommendation);
                }
                continue;
            }

            match Self::create_record(
                env,
                viewer.clone(),
                *candidate_id,
                algorithm,
                Some(source_project_id),
                Some(viewer.clone()),
                Some(*score),
                Some(Self::algorithm_label(env, algorithm)),
            ) {
                Ok(recommendation_id) => {
                    env.storage()
                        .persistent()
                        .set(&dedupe_key, &recommendation_id);
                    Self::extend_user_key_ttl(env, &dedupe_key);
                    if let Some(recommendation) = Self::get_recommendation(env, recommendation_id) {
                        recommendations.push_back(recommendation);
                    }
                }
                Err(ContractError::InvalidInput) => break,
                Err(error) => return Err(error),
            }
        }
        Ok(recommendations)
    }

    // ── Funnel tracking ──────────────────────────────────────────────────────

    fn variant_totals_key(variant: RecommendationVariant) -> RK {
        match variant {
            RecommendationVariant::VariantA => RK::VariantAAnalytics,
            RecommendationVariant::VariantB => RK::VariantBAnalytics,
        }
    }

    fn add_variant_funnel(
        env: &Env,
        variant: RecommendationVariant,
        impressions: u64,
        clicks: u64,
        conversions: u64,
    ) {
        let key = Self::variant_totals_key(variant);
        let current: VariantTotals = env.storage().persistent().get(&key).unwrap_or((0, 0, 0));
        let next = (
            current.0.saturating_add(impressions),
            current.1.saturating_add(clicks),
            current.2.saturating_add(conversions),
        );
        env.storage().persistent().set(&key, &next);
        StorageManager::extend_recommendation_ab_ttl(env);
    }

    pub fn record_impression(
        env: &Env,
        recommendation_id: u64,
        viewer: Address,
    ) -> Result<(), ContractError> {
        viewer.require_auth();
        let recommendation = Self::get_recommendation(env, recommendation_id)
            .ok_or(ContractError::RecommendationNotFound)?;
        Self::check_audience(&recommendation, &viewer)?;

        let seen_key = RK::ImpressionSeen(recommendation_id, viewer.clone());
        let already_seen: bool = env.storage().persistent().get(&seen_key).unwrap_or(false);
        if !already_seen {
            env.storage().persistent().set(&seen_key, &true);
            Self::extend_user_key_ttl(env, &seen_key);
            Self::increment_counter(env, &RK::ImpressionCount(recommendation_id));
            Self::add_variant_funnel(env, recommendation.variant, 1, 0, 0);
        }
        publish_recommendation_impression_event(
            env,
            recommendation_id,
            recommendation.target_project_id,
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
        let recommendation = Self::get_recommendation(env, recommendation_id)
            .ok_or(ContractError::RecommendationNotFound)?;
        Self::check_audience(&recommendation, &viewer)?;
        let has_impression: bool = env
            .storage()
            .persistent()
            .get(&RK::ImpressionSeen(recommendation_id, viewer.clone()))
            .unwrap_or(false);
        if !has_impression {
            return Err(ContractError::RecommendationNoImpression);
        }

        let click_key = RK::ClickSeen(recommendation_id, viewer.clone());
        let already_clicked: bool = env.storage().persistent().get(&click_key).unwrap_or(false);
        if !already_clicked {
            env.storage().persistent().set(&click_key, &true);
            Self::extend_user_key_ttl(env, &click_key);
            Self::increment_counter(env, &RK::ClickCount(recommendation_id));
            Self::add_variant_funnel(env, recommendation.variant, 0, 1, 0);
        }
        publish_recommendation_clicked_event(
            env,
            recommendation_id,
            recommendation.target_project_id,
            viewer.clone(),
        );
        publish_recommendation_engagement_event(
            env,
            recommendation_id,
            recommendation.target_project_id,
            viewer,
            RecommendationEngagementKind::Click,
        );
        Ok(())
    }

    /// Record a unique downstream conversion (for example, a follow or review).
    /// A tracked impression is mandatory to preserve funnel denominator order.
    pub fn record_conversion(
        env: &Env,
        recommendation_id: u64,
        user: Address,
    ) -> Result<(), ContractError> {
        user.require_auth();
        let recommendation = Self::get_recommendation(env, recommendation_id)
            .ok_or(ContractError::RecommendationNotFound)?;
        Self::check_audience(&recommendation, &user)?;
        let has_impression: bool = env
            .storage()
            .persistent()
            .get(&RK::ImpressionSeen(recommendation_id, user.clone()))
            .unwrap_or(false);
        if !has_impression {
            return Err(ContractError::RecommendationNoImpression);
        }
        let converted_key = RK::Converted(recommendation_id, user.clone());
        let already_converted: bool = env
            .storage()
            .persistent()
            .get(&converted_key)
            .unwrap_or(false);
        if already_converted {
            return Err(ContractError::RecommendationAlreadyConverted);
        }
        env.storage().persistent().set(&converted_key, &true);
        Self::extend_user_key_ttl(env, &converted_key);
        Self::increment_counter(env, &RK::ConversionCount(recommendation_id));
        Self::add_variant_funnel(env, recommendation.variant, 0, 0, 1);
        publish_recommendation_conversion_event(
            env,
            recommendation_id,
            recommendation.target_project_id,
            user,
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
        let recommendation = Self::get_recommendation(env, recommendation_id)
            .ok_or(ContractError::RecommendationNotFound)?;
        Self::check_audience(&recommendation, &user)?;
        match kind {
            RecommendationEngagementKind::Impression => {
                Self::record_impression(env, recommendation_id, user.clone())?;
            }
            RecommendationEngagementKind::Click => {
                Self::record_click(env, recommendation_id, user.clone())?;
            }
            RecommendationEngagementKind::Follow => {
                Self::increment_counter(env, &RK::FollowCount(recommendation_id));
            }
            RecommendationEngagementKind::Bookmark => {
                Self::increment_counter(env, &RK::BookmarkCount(recommendation_id));
            }
            RecommendationEngagementKind::Endorse => {
                Self::increment_counter(env, &RK::EndorseCount(recommendation_id));
            }
            RecommendationEngagementKind::Review => {
                Self::increment_counter(env, &RK::ReviewCount(recommendation_id));
            }
        }
        publish_recommendation_engagement_event(
            env,
            recommendation_id,
            recommendation.target_project_id,
            user,
            kind,
        );
        Ok(())
    }

    pub fn give_feedback(
        env: &Env,
        recommendation_id: u64,
        user: Address,
        helpful: bool,
    ) -> Result<(), ContractError> {
        user.require_auth();
        let recommendation = Self::get_recommendation(env, recommendation_id)
            .ok_or(ContractError::RecommendationNotFound)?;
        Self::check_audience(&recommendation, &user)?;
        let feedback_key = RK::Feedback(recommendation_id, user.clone());
        if env
            .storage()
            .persistent()
            .get::<_, RecommendationFeedback>(&feedback_key)
            .is_some()
        {
            return Err(ContractError::RecommendationFeedbackAlreadyGiven);
        }
        let feedback = RecommendationFeedback {
            recommendation_id,
            user: user.clone(),
            helpful,
            created_at: env.ledger().timestamp(),
        };
        env.storage().persistent().set(&feedback_key, &feedback);
        Self::extend_user_key_ttl(env, &feedback_key);
        if helpful {
            Self::increment_counter(env, &RK::HelpfulCount(recommendation_id));
        } else {
            Self::increment_counter(env, &RK::NotHelpfulCount(recommendation_id));
        }
        publish_recommendation_feedback_event(
            env,
            recommendation_id,
            recommendation.target_project_id,
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

    fn ratio_ppm(numerator: u64, denominator: u64) -> u32 {
        if denominator == 0 {
            0
        } else {
            numerator
                .saturating_mul(RATIO_SCALE_PPM as u64)
                .checked_div(denominator)
                .unwrap_or(0) as u32
        }
    }

    pub fn compute_effectiveness_score_bps(
        impressions: u64,
        clicks: u64,
        conversions: u64,
        helpful: u64,
        not_helpful: u64,
        follows: u64,
        bookmarks: u64,
        endorses: u64,
        reviews: u64,
    ) -> u32 {
        let ctr = if impressions >= MIN_IMPRESSIONS_FOR_CTR_SIGNAL {
            Self::ratio_ppm(clicks, impressions) as u128
        } else {
            0
        };
        let feedback = helpful.saturating_add(not_helpful);
        let helpful_signal = if feedback >= MIN_FEEDBACK_FOR_HELPFUL_SIGNAL {
            Self::ratio_ppm(helpful, feedback) as u128
        } else {
            0
        };
        let engagement = follows
            .saturating_add(bookmarks)
            .saturating_add(endorses)
            .saturating_add(reviews)
            .saturating_add(conversions);
        let engagement_signal = if impressions >= MIN_IMPRESSIONS_FOR_CTR_SIGNAL {
            let capped = core::cmp::min(engagement, impressions);
            Self::ratio_ppm(capped, impressions) as u128
        } else {
            0
        };
        let scale = RATIO_SCALE_PPM as u128;
        let weighted = ctr * EFFECTIVENESS_WEIGHT_CTR_BPS as u128
            + helpful_signal * EFFECTIVENESS_WEIGHT_HELPFUL_BPS as u128
            + engagement_signal * EFFECTIVENESS_WEIGHT_ENGAGEMENT_BPS as u128;
        let score = weighted * SCORE_SCALE_BPS as u128 / (scale * scale);
        core::cmp::min(score, SCORE_SCALE_BPS as u128) as u32
    }

    pub fn get_analytics(env: &Env, recommendation_id: u64) -> Option<RecommendationAnalytics> {
        let recommendation = Self::get_recommendation(env, recommendation_id)?;
        let impressions = Self::read_counter(env, &RK::ImpressionCount(recommendation_id));
        let clicks = Self::read_counter(env, &RK::ClickCount(recommendation_id));
        let conversions = Self::read_counter(env, &RK::ConversionCount(recommendation_id));
        let helpful = Self::read_counter(env, &RK::HelpfulCount(recommendation_id));
        let not_helpful = Self::read_counter(env, &RK::NotHelpfulCount(recommendation_id));
        let follows = Self::read_counter(env, &RK::FollowCount(recommendation_id));
        let bookmarks = Self::read_counter(env, &RK::BookmarkCount(recommendation_id));
        let endorses = Self::read_counter(env, &RK::EndorseCount(recommendation_id));
        let reviews = Self::read_counter(env, &RK::ReviewCount(recommendation_id));
        let ctr = Self::ratio_ppm(clicks, impressions);
        let conversion_rate = Self::ratio_ppm(conversions, impressions);
        let helpful_ratio = Self::ratio_ppm(helpful, helpful.saturating_add(not_helpful));
        let effectiveness = Self::compute_effectiveness_score_bps(
            impressions,
            clicks,
            conversions,
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
            recommendation.target_project_id,
            impressions,
            clicks,
            conversions,
            ctr,
            conversion_rate,
            helpful,
            not_helpful,
            helpful_ratio,
            effectiveness,
        );
        Some(RecommendationAnalytics {
            recommendation_id,
            impressions,
            clicks,
            conversions,
            click_through_rate_ppm: ctr,
            conversion_rate_ppm: conversion_rate,
            helpful_count: helpful,
            not_helpful_count: not_helpful,
            helpful_ratio_ppm: helpful_ratio,
            follow_engagements: follows,
            bookmark_engagements: bookmarks,
            endorse_engagements: endorses,
            review_engagements: reviews,
            effectiveness_score_bps: effectiveness,
        })
    }

    pub fn get_ab_analytics(
        env: &Env,
        variant: RecommendationVariant,
    ) -> RecommendationABAnalytics {
        let totals: VariantTotals = env
            .storage()
            .persistent()
            .get(&Self::variant_totals_key(variant))
            .unwrap_or((0, 0, 0));
        StorageManager::extend_recommendation_ab_ttl(env);
        RecommendationABAnalytics {
            variant,
            impressions: totals.0,
            clicks: totals.1,
            conversions: totals.2,
            click_through_rate_ppm: Self::ratio_ppm(totals.1, totals.0),
            conversion_rate_ppm: Self::ratio_ppm(totals.2, totals.0),
        }
    }

    /// Return recommendations ordered from highest effectiveness to lowest.
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
        let mut pairs: alloc::vec::Vec<(u64, u32)> = alloc::vec::Vec::new();
        for id in ids.iter() {
            let impressions = Self::read_counter(env, &RK::ImpressionCount(id));
            let clicks = Self::read_counter(env, &RK::ClickCount(id));
            let conversions = Self::read_counter(env, &RK::ConversionCount(id));
            let helpful = Self::read_counter(env, &RK::HelpfulCount(id));
            let not_helpful = Self::read_counter(env, &RK::NotHelpfulCount(id));
            let follows = Self::read_counter(env, &RK::FollowCount(id));
            let bookmarks = Self::read_counter(env, &RK::BookmarkCount(id));
            let endorses = Self::read_counter(env, &RK::EndorseCount(id));
            let reviews = Self::read_counter(env, &RK::ReviewCount(id));
            pairs.push((
                id,
                Self::compute_effectiveness_score_bps(
                    impressions,
                    clicks,
                    conversions,
                    helpful,
                    not_helpful,
                    follows,
                    bookmarks,
                    endorses,
                    reviews,
                ),
            ));
        }
        pairs.sort_by(|left, right| {
            use core::cmp::Ordering;
            match right.1.cmp(&left.1) {
                Ordering::Equal => left.0.cmp(&right.0),
                ordering => ordering,
            }
        });
        if start_index as usize >= pairs.len() {
            return Vec::new(env);
        }
        let end = core::cmp::min(
            start_index.saturating_add(effective_limit),
            pairs.len() as u32,
        );
        let mut page = Vec::new(env);
        for (id, _) in pairs
            .iter()
            .skip(start_index as usize)
            .take((end - start_index) as usize)
        {
            if let Some(recommendation) = Self::get_recommendation(env, *id) {
                page.push_back(recommendation);
            }
        }
        page
    }
}
