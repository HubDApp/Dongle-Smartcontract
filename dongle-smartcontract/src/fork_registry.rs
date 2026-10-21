//! Fork and migration detection for registered projects (issue #748).
//!
//! Detection is deterministic and read-only. Creating a relationship is
//! restricted to the child owner or an admin and requires at least one
//! positive signal: shared team, similar name, or the same repository URL.

use crate::admin_manager::AdminManager;
use crate::constants::MAX_PAGE_LIMIT;
use crate::errors::ContractError;
use crate::events::publish_fork_linked_event;
use crate::fork_types::{ForkDataMerge, ForkDetection, ForkRelationship, ForkSignal};
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::{ForkKey, StorageKey};
use crate::storage_manager::StorageManager;
use crate::types::{Project, ProjectStats, VerificationRecord};
use crate::utils::Utils;
use soroban_sdk::{Address, Env, String, Vec};

const MIN_SIMILAR_NAME_LENGTH: usize = 3;
const SIMILAR_NAME_THRESHOLD_BPS: u32 = 6_000;
const MAX_FORK_ANCESTRY_DEPTH: usize = 16;
const MAX_FORK_CHILDREN: u32 = 1_000;

pub struct ForkRegistry;

impl ForkRegistry {
    fn shared_maintainers(env: &Env, child: &Project, parent: &Project) -> Vec<Address> {
        let mut shared = Vec::new(env);
        if let Some(child_maintainers) = &child.maintainers {
            if let Some(parent_maintainers) = &parent.maintainers {
                for child_maintainer in child_maintainers.iter() {
                    if parent_maintainers.contains(&child_maintainer) {
                        shared.push_back(child_maintainer);
                    }
                }
            }
        }
        shared
    }

    /// Return normalized Levenshtein similarity in basis points.
    ///
    /// Project names are bounded by `MAX_NAME_LEN`, so a two-row dynamic
    /// programming implementation is bounded and avoids floating point.
    pub fn name_similarity_bps(env: &Env, left: &String, right: &String) -> u32 {
        let left = Utils::normalize_project_name(env, left);
        let right = Utils::normalize_project_name(env, right);
        let left_len = left.len() as usize;
        let right_len = right.len() as usize;

        if left_len == 0 || right_len == 0 {
            return 0;
        }
        if left == right {
            return 10_000;
        }

        let mut left_bytes = [0u8; 64];
        let mut right_bytes = [0u8; 64];
        if left_len > left_bytes.len() || right_len > right_bytes.len() {
            return 0;
        }
        left.copy_into_slice(&mut left_bytes[..left_len]);
        right.copy_into_slice(&mut right_bytes[..right_len]);

        let mut previous = [0u32; 65];
        let mut current = [0u32; 65];
        for j in 0..=right_len {
            previous[j] = j as u32;
        }

        for i in 1..=left_len {
            current[0] = i as u32;
            for j in 1..=right_len {
                let substitution =
                    previous[j - 1] + u32::from(left_bytes[i - 1] != right_bytes[j - 1]);
                let insertion = current[j - 1].saturating_add(1);
                let deletion = previous[j].saturating_add(1);
                current[j] = core::cmp::min(substitution, core::cmp::min(insertion, deletion));
            }
            let mut swap = [0u32; 65];
            swap.copy_from_slice(&current);
            previous.copy_from_slice(&swap);
        }

        let max_len = core::cmp::max(left_len, right_len) as u32;
        let matching = max_len.saturating_sub(previous[right_len]);
        matching.saturating_mul(10_000) / max_len
    }

    fn same_repository(env: &Env, child: &Project, parent: &Project) -> bool {
        match (&child.repository_url, &parent.repository_url) {
            (Some(child_repo), Some(parent_repo)) => {
                child_repo == parent_repo
                    || Utils::to_lowercase(env, child_repo) == Utils::to_lowercase(env, parent_repo)
            }
            _ => false,
        }
    }

    /// Compare a proposed child with a proposed parent and return all matching
    /// migration signals. No storage is written by this function.
    pub fn detect(
        env: &Env,
        child_project_id: u64,
        parent_project_id: u64,
    ) -> Result<ForkDetection, ContractError> {
        if child_project_id == parent_project_id {
            return Err(ContractError::CannotLinkToSelf);
        }

        let child = ProjectRegistry::get_project(env, child_project_id)
            .ok_or(ContractError::ProjectNotFound)?;
        let parent = ProjectRegistry::get_project(env, parent_project_id)
            .ok_or(ContractError::ProjectNotFound)?;

        let mut signals = Vec::new(env);
        let mut shared_team_members = Vec::new(env);
        let mut confidence_bps = 0u32;

        if child.owner == parent.owner {
            signals.push_back(ForkSignal::SameOwner);
            shared_team_members.push_back(child.owner.clone());
            confidence_bps = confidence_bps.saturating_add(3_000);
        }

        let shared_maintainers = Self::shared_maintainers(env, &child, &parent);
        if !shared_maintainers.is_empty() {
            signals.push_back(ForkSignal::SharedMaintainer);
            for maintainer in shared_maintainers.iter() {
                shared_team_members.push_back(maintainer);
            }
            confidence_bps = confidence_bps.saturating_add(2_500);
        }

        let normalized_child_name = Utils::normalize_project_name(env, &child.name);
        let normalized_parent_name = Utils::normalize_project_name(env, &parent.name);
        let min_name_len = core::cmp::min(
            normalized_child_name.len() as usize,
            normalized_parent_name.len() as usize,
        );
        if min_name_len >= MIN_SIMILAR_NAME_LENGTH {
            let similarity = Self::name_similarity_bps(env, &child.name, &parent.name);
            if similarity >= SIMILAR_NAME_THRESHOLD_BPS {
                signals.push_back(ForkSignal::SimilarName);
                confidence_bps = confidence_bps.saturating_add(similarity);
            }
        }

        if Self::same_repository(env, &child, &parent) {
            signals.push_back(ForkSignal::SameRepository);
            confidence_bps = confidence_bps.saturating_add(4_500);
        }

        Ok(ForkDetection {
            child_project_id,
            parent_project_id,
            signals,
            shared_team_members,
            confidence_bps: core::cmp::min(confidence_bps, 10_000),
        })
    }

    fn ensure_no_ancestry_cycle(
        env: &Env,
        child_project_id: u64,
        parent_project_id: u64,
    ) -> Result<(), ContractError> {
        let mut cursor = parent_project_id;
        for _ in 0..MAX_FORK_ANCESTRY_DEPTH {
            if cursor == child_project_id {
                return Err(ContractError::CircularDependency);
            }
            let next = env
                .storage()
                .persistent()
                .get(&ForkKey::ParentForChild(cursor))
                .unwrap_or(0);
            if next == 0 {
                return Ok(());
            }
            cursor = next;
        }
        Err(ContractError::InvalidInput)
    }

    fn parent_data(env: &Env, parent_project_id: u64) -> Result<ForkDataMerge, ContractError> {
        let review_stats: ProjectStats = env
            .storage()
            .persistent()
            .get(&StorageKey::ProjectStats(parent_project_id))
            .unwrap_or(ProjectStats {
                rating_sum: 0,
                review_count: 0,
                average_rating: 0,
            });

        let parent = ProjectRegistry::get_project(env, parent_project_id)
            .ok_or(ContractError::ProjectNotFound)?;
        let verification_record = parent.current_verification_id.and_then(|request_id| {
            env.storage()
                .persistent()
                .get::<StorageKey, VerificationRecord>(&StorageKey::VerificationRecord(request_id))
        });

        Ok(ForkDataMerge {
            review_stats,
            verification_record,
            merged_at: env.ledger().timestamp(),
        })
    }

    /// Create an immutable parent-child relationship for a detected fork or
    /// migration. The child owner or an admin must authenticate.
    ///
    /// When `merge_parent_data` is true, parent review aggregates and the
    /// latest verification record are copied into `merged_data` with explicit
    /// parent provenance. The child's authorization and individual review
    /// records are never impersonated by the parent.
    pub fn link(
        env: &Env,
        child_project_id: u64,
        parent_project_id: u64,
        caller: Address,
        merge_parent_data: bool,
    ) -> Result<ForkRelationship, ContractError> {
        caller.require_auth();

        let child = ProjectRegistry::get_project(env, child_project_id)
            .ok_or(ContractError::ProjectNotFound)?;
        if child.owner != caller && !AdminManager::is_admin(env, &caller) {
            return Err(ContractError::Unauthorized);
        }
        if env
            .storage()
            .persistent()
            .has(&ForkKey::ParentForChild(child_project_id))
        {
            return Err(ContractError::AlreadyLinked);
        }

        let detection = Self::detect(env, child_project_id, parent_project_id)?;
        if detection.signals.is_empty() {
            return Err(ContractError::InvalidInput);
        }
        Self::ensure_no_ancestry_cycle(env, child_project_id, parent_project_id)?;

        let mut children: Vec<u64> = env
            .storage()
            .persistent()
            .get(&ForkKey::ChildProjects(parent_project_id))
            .unwrap_or_else(|| Vec::new(env));
        if children.len() >= MAX_FORK_CHILDREN {
            return Err(ContractError::MaxProjectsExceeded);
        }

        let relationship = ForkRelationship {
            child_project_id,
            parent_project_id,
            signals: detection.signals,
            shared_team_members: detection.shared_team_members,
            confidence_bps: detection.confidence_bps,
            linked_by: caller.clone(),
            created_at: env.ledger().timestamp(),
            merged_data: if merge_parent_data {
                Some(Self::parent_data(env, parent_project_id)?)
            } else {
                None
            },
        };

        children.push_back(child_project_id);
        env.storage()
            .persistent()
            .set(&ForkKey::Relationship(child_project_id), &relationship);
        env.storage().persistent().set(
            &ForkKey::ParentForChild(child_project_id),
            &parent_project_id,
        );
        env.storage()
            .persistent()
            .set(&ForkKey::ChildProjects(parent_project_id), &children);

        StorageManager::extend_fork_relationship_ttl(env, child_project_id, parent_project_id);
        publish_fork_linked_event(env, &relationship);
        crate::notification_registry::NotificationRegistry::emit_project_notification(
            env,
            child_project_id,
            crate::types::NotificationKind::ProjectUpdate,
        );

        Ok(relationship)
    }

    pub fn get_relationship(env: &Env, child_project_id: u64) -> Option<ForkRelationship> {
        let relationship = env
            .storage()
            .persistent()
            .get(&ForkKey::Relationship(child_project_id));
        if let Some(parent_project_id) = env
            .storage()
            .persistent()
            .get(&ForkKey::ParentForChild(child_project_id))
        {
            StorageManager::extend_fork_relationship_ttl(env, child_project_id, parent_project_id);
        }
        relationship
    }

    pub fn get_parent(env: &Env, child_project_id: u64) -> Option<u64> {
        let parent_project_id: Option<u64> = env
            .storage()
            .persistent()
            .get(&ForkKey::ParentForChild(child_project_id));
        if let Some(parent_project_id) = &parent_project_id {
            StorageManager::extend_fork_relationship_ttl(env, child_project_id, *parent_project_id);
        }
        parent_project_id
    }

    pub fn list_children(
        env: &Env,
        parent_project_id: u64,
        start_index: u32,
        limit: u32,
    ) -> Vec<u64> {
        if ProjectRegistry::get_project(env, parent_project_id).is_none() {
            return Vec::new(env);
        }
        let effective_limit = if limit == 0 || limit > MAX_PAGE_LIMIT {
            MAX_PAGE_LIMIT
        } else {
            limit
        };
        let children: Vec<u64> = env
            .storage()
            .persistent()
            .get(&ForkKey::ChildProjects(parent_project_id))
            .unwrap_or_else(|| Vec::new(env));
        if start_index >= children.len() {
            return Vec::new(env);
        }
        let end = core::cmp::min(start_index.saturating_add(effective_limit), children.len());
        let mut page = Vec::new(env);
        for index in start_index..end {
            if let Some(child_id) = children.get(index) {
                page.push_back(child_id);
            }
        }
        StorageManager::extend_fork_children_ttl(env, parent_project_id);
        page
    }
}
