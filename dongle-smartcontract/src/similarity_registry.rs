//! Similarity and near-duplicate project detection registry (#913).
//!
//! Implements similarity scoring based on name, description, tags, and dependencies,
//! with fuzzy string matching, sorted similarity reporting, and duplicate dispute integration.

use crate::dependency_registry::DependencyRegistry;
use crate::dispute_registry::DisputeRegistry;
use crate::errors::ContractError;
use crate::events::publish_similar_projects_queried_event;
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::StorageKey;
use crate::types::{Project, SimilarProject};
use alloc::vec;
use soroban_sdk::{Address, Env, String, Vec};

pub struct SimilarityRegistry;

impl SimilarityRegistry {
    /// Calculate similarity score between two projects in basis points (0..10_000).
    ///
    /// Weights:
    /// - Name similarity (fuzzy matching): 40% (4_000 bps)
    /// - Description similarity (fuzzy matching): 25% (2_500 bps)
    /// - Tags similarity (Jaccard index): 20% (2_000 bps)
    /// - Dependencies similarity (Jaccard index): 15% (1_500 bps)
    pub fn calculate_similarity_score(env: &Env, p1: &Project, p2: &Project) -> u32 {
        if p1.id == p2.id {
            return 10_000;
        }

        // 1. Name fuzzy matching (40%)
        let name_sim = Self::fuzzy_string_similarity(&p1.name, &p2.name);

        // 2. Description fuzzy matching (25%)
        let desc_sim = Self::fuzzy_string_similarity(&p1.description, &p2.description);

        // 3. Tags Jaccard similarity (20%)
        let tags_sim = Self::calculate_tags_similarity(env, p1, p2);

        // 4. Dependencies Jaccard similarity (15%)
        let deps_sim = Self::calculate_dependencies_similarity(env, p1.id, p2.id);

        let total_score = (name_sim as u64 * 4_000
            + desc_sim as u64 * 2_500
            + tags_sim as u64 * 2_000
            + deps_sim as u64 * 1_500)
            / 10_000;

        total_score as u32
    }

    /// Fuzzy string similarity using Dice's bigram coefficient.
    /// Returns a score between 0 and 10_000 basis points.
    pub fn fuzzy_string_similarity(s1: &String, s2: &String) -> u32 {
        let len1 = s1.len() as usize;
        let len2 = s2.len() as usize;

        if len1 == 0 && len2 == 0 {
            return 10_000;
        }
        if len1 == 0 || len2 == 0 {
            return 0;
        }

        let mut b1 = vec![0u8; len1];
        let mut b2 = vec![0u8; len2];
        s1.copy_into_slice(&mut b1);
        s2.copy_into_slice(&mut b2);

        // Normalize to lowercase ASCII
        for b in b1.iter_mut() {
            if *b >= b'A' && *b <= b'Z' {
                *b += 32;
            }
        }
        for b in b2.iter_mut() {
            if *b >= b'A' && *b <= b'Z' {
                *b += 32;
            }
        }

        // Exact match short-circuit
        if b1 == b2 {
            return 10_000;
        }

        if len1 < 2 || len2 < 2 {
            return 0;
        }

        let bigrams1_count = len1 - 1;
        let bigrams2_count = len2 - 1;

        let mut matched_in_2 = vec![false; bigrams2_count];
        let mut matches = 0usize;

        for i in 0..bigrams1_count {
            let bg1 = (b1[i], b1[i + 1]);
            for j in 0..bigrams2_count {
                if !matched_in_2[j] && bg1 == (b2[j], b2[j + 1]) {
                    matched_in_2[j] = true;
                    matches += 1;
                    break;
                }
            }
        }

        let total_bigrams = bigrams1_count + bigrams2_count;
        if total_bigrams == 0 {
            return 0;
        }

        let score = (2 * matches as u64 * 10_000) / total_bigrams as u64;
        score as u32
    }

    /// Jaccard similarity for project tags.
    fn calculate_tags_similarity(_env: &Env, p1: &Project, p2: &Project) -> u32 {
        let tags1 = match &p1.tags {
            Some(t) if !t.is_empty() => t,
            _ => return 0,
        };
        let tags2 = match &p2.tags {
            Some(t) if !t.is_empty() => t,
            _ => return 0,
        };

        let len1 = tags1.len();
        let len2 = tags2.len();
        let mut intersection = 0u32;

        for i in 0..len1 {
            if let Some(t1) = tags1.get(i) {
                for j in 0..len2 {
                    if let Some(t2) = tags2.get(j) {
                        if t1 == t2 {
                            intersection += 1;
                            break;
                        }
                    }
                }
            }
        }

        let union = len1 + len2 - intersection;
        if union == 0 {
            return 0;
        }

        ((intersection as u64 * 10_000) / union as u64) as u32
    }

    /// Jaccard similarity for project dependencies.
    fn calculate_dependencies_similarity(env: &Env, id1: u64, id2: u64) -> u32 {
        let deps1 = DependencyRegistry::dependency_project_ids(env, id1);
        let deps2 = DependencyRegistry::dependency_project_ids(env, id2);

        let len1 = deps1.len();
        let len2 = deps2.len();
        if len1 == 0 && len2 == 0 {
            return 0;
        }
        if len1 == 0 || len2 == 0 {
            return 0;
        }

        let mut intersection = 0u32;
        for i in 0..len1 {
            if let Some(d1) = deps1.get(i) {
                if deps2.contains(&d1) {
                    intersection += 1;
                }
            }
        }

        let union = len1 + len2 - intersection;
        if union == 0 {
            return 0;
        }

        ((intersection as u64 * 10_000) / union as u64) as u32
    }

    /// Find projects similar to `project_id` exceeding `threshold`.
    ///
    /// Threshold supports:
    /// - Basis points (0..10_000, where 5000 = 50.00% or 0.50)
    /// - Percentage shorthand (1..100, where 50 = 50% = 5000 bps)
    pub fn find_similar_projects(
        env: &Env,
        project_id: u64,
        threshold: u32,
    ) -> Vec<SimilarProject> {
        let Some(target) = ProjectRegistry::get_project(env, project_id) else {
            return Vec::new(env);
        };

        // Normalize threshold: 1..=100 treated as percentage (100 -> 10000 bps)
        let effective_threshold = if threshold > 0 && threshold <= 100 {
            threshold.saturating_mul(100)
        } else {
            threshold
        };

        let count: u64 = env
            .storage()
            .persistent()
            .get(&StorageKey::ProjectCount)
            .unwrap_or(0);

        let mut candidates = Vec::new(env);

        for id in 1..=count {
            if id == project_id {
                continue;
            }
            let Some(candidate) = ProjectRegistry::get_project(env, id) else {
                continue;
            };
            if candidate.archived {
                continue;
            }

            let score = Self::calculate_similarity_score(env, &target, &candidate);
            if score >= effective_threshold {
                candidates.push_back(SimilarProject {
                    project_id: candidate.id,
                    name: candidate.name,
                    similarity_score: score,
                });
            }
        }

        // Sort descending by similarity_score, then ascending by project_id
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

                if item_i.similarity_score > item_best.similarity_score
                    || (item_i.similarity_score == item_best.similarity_score
                        && item_i.project_id < item_best.project_id)
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

        publish_similar_projects_queried_event(env, project_id, sorted.len());
        sorted
    }

    /// Open a duplicate dispute integrated with similarity verification.
    /// Checks that the candidate duplicate meets `min_threshold` before opening.
    pub fn open_duplicate_dispute_with_similarity(
        env: &Env,
        project_id: u64,
        original_project_id: u64,
        creator: Address,
        evidence_cid: String,
        min_threshold: u32,
    ) -> Result<u64, ContractError> {
        let p1 =
            ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;
        let p2 = ProjectRegistry::get_project(env, original_project_id)
            .ok_or(ContractError::ProjectNotFound)?;

        let effective_threshold = if min_threshold > 0 && min_threshold <= 100 {
            min_threshold.saturating_mul(100)
        } else {
            min_threshold
        };

        if effective_threshold > 0 {
            let score = Self::calculate_similarity_score(env, &p1, &p2);
            if score < effective_threshold {
                return Err(ContractError::SimilarityThresholdNotMet);
            }
        }

        DisputeRegistry::open_duplicate_dispute(
            env,
            project_id,
            original_project_id,
            creator,
            evidence_cid,
        )
    }
}
