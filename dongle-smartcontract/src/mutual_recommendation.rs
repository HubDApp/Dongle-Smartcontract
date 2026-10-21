//! Mutual project recommendations (issue #948).
//!
//! Projects can submit recommendations for each other: `recommend_project`
//! records a `(recommender_id, target_id, reason_cid)` triple, guards against
//! self-recommendations and duplicates, exposes a per-project recommendation
//! counter, supports filtering recommendations by the *target* project's
//! category, and emits a `ProjectRecommended` event.

use crate::constants::*;
use crate::errors::ContractError;
use crate::events::publish_project_recommended_event;
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::ExtensionKey2 as EK2;
use crate::storage_manager::StorageManager;
use crate::types::ProjectRecommendation;
use soroban_sdk::{Env, String, Vec};

pub struct MutualRecommendationRegistry;

impl MutualRecommendationRegistry {
    fn next_id(env: &Env) -> u64 {
        let current: u64 = env
            .storage()
            .persistent()
            .get(&EK2::NextProjectRecId)
            .unwrap_or(0);
        let next = current.saturating_add(1);
        env.storage()
            .persistent()
            .set(&EK2::NextProjectRecId, &next);
        next
    }

    /// Record a mutual recommendation from one project to another.
    ///
    /// Guards:
    /// - both projects must exist,
    /// - a project cannot recommend itself,
    /// - a (recommender, target) pair can only be recorded once,
    /// - `reason_cid` must be non-empty and within `MAX_CID_LEN` bytes.
    pub fn recommend_project(
        env: &Env,
        recommender_id: u64,
        target_id: u64,
        reason_cid: String,
    ) -> Result<u64, ContractError> {
        if recommender_id == target_id {
            return Err(ContractError::SelfRecommendation);
        }
        if ProjectRegistry::get_project(env, recommender_id).is_none()
            || ProjectRegistry::get_project(env, target_id).is_none()
        {
            return Err(ContractError::ProjectNotFound);
        }
        if reason_cid.len() == 0 || reason_cid.len() as usize > MAX_CID_LEN {
            return Err(ContractError::InvalidInput);
        }
        let pair_key = EK2::ProjectRecPair(recommender_id, target_id);
        if env.storage().persistent().has(&pair_key) {
            return Err(ContractError::DuplicateRecommendation);
        }

        let id = Self::next_id(env);
        let rec = ProjectRecommendation {
            id,
            recommender_id,
            target_id,
            reason_cid: reason_cid.clone(),
            created_at: env.ledger().timestamp(),
        };
        env.storage().persistent().set(&EK2::ProjectRec(id), &rec);
        env.storage().persistent().set(&pair_key, &id);

        let mut global: Vec<u64> = env
            .storage()
            .persistent()
            .get(&EK2::ProjectRecList)
            .unwrap_or_else(|| Vec::new(env));
        global.push_back(id);
        env.storage().persistent().set(&EK2::ProjectRecList, &global);

        let mut for_target: Vec<u64> = env
            .storage()
            .persistent()
            .get(&EK2::ProjectRecs(target_id))
            .unwrap_or_else(|| Vec::new(env));
        for_target.push_back(id);
        env.storage()
            .persistent()
            .set(&EK2::ProjectRecs(target_id), &for_target);

        StorageManager::extend_project_ttl(env, recommender_id);
        StorageManager::extend_project_ttl(env, target_id);
        publish_project_recommended_event(env, id, recommender_id, target_id, reason_cid);
        Ok(id)
    }

    pub fn get_recommendation(env: &Env, recommendation_id: u64) -> Option<ProjectRecommendation> {
        env.storage().persistent().get(&EK2::ProjectRec(recommendation_id))
    }

    /// Recommendation counter for a project: number of mutual recommendations
    /// received by `target_id`. Surfaced on the project display.
    pub fn get_recommendation_count(env: &Env, target_id: u64) -> u32 {
        env.storage()
            .persistent()
            .get::<_, Vec<u64>>(&EK2::ProjectRecs(target_id))
            .map(|ids| ids.len())
            .unwrap_or(0)
    }

    /// List mutual recommendations whose *target* project belongs to
    /// `category`, oldest-first with pagination.
    pub fn list_by_category(
        env: &Env,
        category: String,
        start_index: u32,
        limit: u32,
    ) -> Vec<ProjectRecommendation> {
        let effective = if limit == 0 || limit > MAX_PAGE_LIMIT {
            MAX_PAGE_LIMIT
        } else {
            limit
        };
        let ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&EK2::ProjectRecList)
            .unwrap_or_else(|| Vec::new(env));
        let mut out = Vec::new(env);
        let mut skipped = 0u32;
        for id in ids.iter() {
            let rec: Option<ProjectRecommendation> =
                env.storage().persistent().get(&EK2::ProjectRec(id));
            if let Some(rec) = rec {
                let matches = ProjectRegistry::get_project(env, rec.target_id)
                    .map(|p| p.category == category)
                    .unwrap_or(false);
                if matches {
                    if skipped < start_index {
                        skipped += 1;
                        continue;
                    }
                    if out.len() >= effective {
                        break;
                    }
                    out.push_back(rec);
                }
            }
        }
        out
    }
}
