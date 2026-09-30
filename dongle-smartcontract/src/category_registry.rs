//! Admin-managed dynamic project categories with bounded hierarchy, stats,
//! rename propagation, and safe deletion migrations (issue #752).

use crate::auth::require_admin_auth;
use crate::category_types::{
    ProjectCategory, ProjectCategoryMigrationResult, ProjectCategoryStats,
};
use crate::constants::{
    LEDGER_BUMP_PROJECT, LEDGER_THRESHOLD_PROJECT, MAX_CATEGORY_CHILDREN,
    MAX_CATEGORY_DESCRIPTION_LEN, MAX_CATEGORY_HIERARCHY_DEPTH, MAX_PAGE_LIMIT,
    MAX_PROJECT_CATEGORIES,
};
use crate::errors::ContractError;
use crate::events::{
    publish_project_category_created_event, publish_project_category_deleted_event,
    publish_project_category_updated_event,
};
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::{ProjectCategoryKey as CK, StorageKey};
use crate::storage_manager::StorageManager;
use crate::utils::Utils;
use soroban_sdk::{Address, Env, String, Vec};

pub struct CategoryRegistry;

impl CategoryRegistry {
    fn empty_stats() -> ProjectCategoryStats {
        ProjectCategoryStats {
            direct_project_count: 0,
            direct_active_project_count: 0,
            direct_child_count: 0,
            descendant_project_count: 0,
            hierarchy_active_project_count: 0,
        }
    }

    fn validate_name(env: &Env, name: &String) -> Result<String, ContractError> {
        Utils::validate_category_field(name)?;
        let normalized = Utils::normalize_project_name(env, name);
        if normalized.is_empty() {
            return Err(ContractError::InvalidInput);
        }
        Ok(normalized)
    }

    fn validate_description(description: &String) -> Result<(), ContractError> {
        if description.len() as usize > MAX_CATEGORY_DESCRIPTION_LEN {
            return Err(ContractError::InvalidInput);
        }
        Ok(())
    }

    pub fn get_category(env: &Env, category_id: u64) -> Option<ProjectCategory> {
        let category = env.storage().persistent().get(&CK::Category(category_id));
        if category.is_some() {
            StorageManager::extend_project_category_ttl(env, category_id);
        }
        category
    }

    fn extend_name_key_ttl(env: &Env, normalized: &String) {
        let key = CK::IdByNormalizedName(normalized.clone());
        if env.storage().persistent().has(&key) {
            env.storage().persistent().extend_ttl(
                &key,
                LEDGER_THRESHOLD_PROJECT,
                LEDGER_BUMP_PROJECT,
            );
        }
    }

    pub fn get_category_by_name(env: &Env, name: &String) -> Option<ProjectCategory> {
        let normalized = Utils::normalize_project_name(env, name);
        let name_key = CK::IdByNormalizedName(normalized.clone());
        let category_id: Option<u64> = env.storage().persistent().get(&name_key);
        if let Some(category_id) = category_id {
            Self::extend_name_key_ttl(env, &normalized);
            return Self::get_category(env, category_id);
        }
        None
    }

    /// Managed categories must be active. Unmanaged labels remain accepted for
    /// backward compatibility with projects registered before category rollout.
    pub fn validate_project_category(env: &Env, category: &String) -> Result<(), ContractError> {
        if let Some(record) = Self::get_category_by_name(env, category) {
            if !record.active {
                return Err(ContractError::ProjectCategoryInactive);
            }
        }
        Ok(())
    }

    fn next_id(env: &Env) -> u64 {
        let current: u64 = env
            .storage()
            .persistent()
            .get(&CK::NextCategoryId)
            .unwrap_or(0);
        let next = current.saturating_add(1);
        env.storage().persistent().set(&CK::NextCategoryId, &next);
        StorageManager::extend_project_category_global_ttl(env);
        next
    }

    fn child_ids(env: &Env, category_id: u64) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&CK::Children(category_id))
            .unwrap_or_else(|| Vec::new(env))
    }

    fn parent_id(env: &Env, category_id: u64) -> Option<u64> {
        env.storage().persistent().get(&CK::Parent(category_id))
    }

    fn depth(env: &Env, category_id: u64) -> Option<u32> {
        let mut cursor = category_id;
        let mut depth = 0u32;
        for _ in 0..=MAX_CATEGORY_HIERARCHY_DEPTH {
            if depth >= MAX_CATEGORY_HIERARCHY_DEPTH {
                return None;
            }
            depth = depth.saturating_add(1);
            match Self::parent_id(env, cursor) {
                Some(parent) => cursor = parent,
                None => return Some(depth),
            }
        }
        None
    }

    fn is_descendant(env: &Env, candidate_id: u64, ancestor_id: u64) -> bool {
        let mut cursor = candidate_id;
        for _ in 0..MAX_CATEGORY_HIERARCHY_DEPTH {
            if cursor == ancestor_id {
                return true;
            }
            match Self::parent_id(env, cursor) {
                Some(parent) => cursor = parent,
                None => return false,
            }
        }
        false
    }

    fn subtree_height(env: &Env, category_id: u64, current_depth: u32) -> u32 {
        if current_depth >= MAX_CATEGORY_HIERARCHY_DEPTH {
            return current_depth;
        }
        let mut height = current_depth;
        for child_id in Self::child_ids(env, category_id).iter() {
            height = core::cmp::max(
                height,
                Self::subtree_height(env, child_id, current_depth.saturating_add(1)),
            );
        }
        height
    }

    fn validate_parent(
        env: &Env,
        category_id: Option<u64>,
        parent_id: Option<u64>,
    ) -> Result<(), ContractError> {
        let parent_id = match parent_id {
            Some(parent_id) => parent_id,
            None => return Ok(()),
        };
        if category_id == Some(parent_id) {
            return Err(ContractError::ProjectCategoryHierarchyInvalid);
        }
        let parent =
            Self::get_category(env, parent_id).ok_or(ContractError::ProjectCategoryNotFound)?;
        if !parent.active {
            return Err(ContractError::ProjectCategoryInactive);
        }
        if let Some(category_id) = category_id {
            if Self::is_descendant(env, parent_id, category_id) {
                return Err(ContractError::ProjectCategoryHierarchyInvalid);
            }
        }
        let parent_depth =
            Self::depth(env, parent_id).ok_or(ContractError::ProjectCategoryHierarchyInvalid)?;
        let subtree_height = match category_id {
            Some(category_id) => Self::subtree_height(env, category_id, 1),
            None => 1,
        };
        if parent_depth.saturating_add(subtree_height) > MAX_CATEGORY_HIERARCHY_DEPTH {
            return Err(ContractError::ProjectCategoryHierarchyInvalid);
        }
        Ok(())
    }

    fn ensure_child_capacity(
        env: &Env,
        parent_id: u64,
        additional: u32,
    ) -> Result<(), ContractError> {
        if Self::child_ids(env, parent_id)
            .len()
            .saturating_add(additional)
            > MAX_CATEGORY_CHILDREN
        {
            return Err(ContractError::ProjectCategoryHierarchyInvalid);
        }
        Ok(())
    }

    fn append_child(env: &Env, parent_id: u64, child_id: u64) -> Result<(), ContractError> {
        let mut children = Self::child_ids(env, parent_id);
        if !children.contains(&child_id) {
            Self::ensure_child_capacity(env, parent_id, 1)?;
            children.push_back(child_id);
        }
        env.storage()
            .persistent()
            .set(&CK::Children(parent_id), &children);
        StorageManager::extend_project_category_ttl(env, parent_id);
        Ok(())
    }

    fn remove_child(env: &Env, parent_id: u64, child_id: u64) {
        let children = Self::child_ids(env, parent_id);
        let mut remaining = Vec::new(env);
        for index in 0..children.len() {
            if let Some(id) = children.get(index) {
                if id != child_id {
                    remaining.push_back(id);
                }
            }
        }
        if remaining.is_empty() {
            env.storage().persistent().remove(&CK::Children(parent_id));
        } else {
            env.storage()
                .persistent()
                .set(&CK::Children(parent_id), &remaining);
        }
    }

    pub fn create_category(
        env: &Env,
        admin: Address,
        name: String,
        description: String,
        parent_id: Option<u64>,
    ) -> Result<ProjectCategory, ContractError> {
        require_admin_auth(env, &admin)?;
        let normalized = Self::validate_name(env, &name)?;
        Self::validate_description(&description)?;
        if env
            .storage()
            .persistent()
            .has(&CK::IdByNormalizedName(normalized.clone()))
        {
            return Err(ContractError::ProjectCategoryAlreadyExists);
        }
        let mut categories: Vec<u64> = env
            .storage()
            .persistent()
            .get(&CK::CategoryList)
            .unwrap_or_else(|| Vec::new(env));
        if categories.len() >= MAX_PROJECT_CATEGORIES {
            return Err(ContractError::InvalidInput);
        }
        Self::validate_parent(env, None, parent_id)?;
        if let Some(parent_id) = parent_id {
            Self::ensure_child_capacity(env, parent_id, 1)?;
        }

        let id = Self::next_id(env);
        if let Some(parent_id) = parent_id {
            Self::append_child(env, parent_id, id)?;
        }

        let now = env.ledger().timestamp();
        let category = ProjectCategory {
            id,
            name: name.clone(),
            description,
            parent_id,
            active: true,
            created_at: now,
            updated_at: now,
        };
        categories.push_back(id);
        env.storage().persistent().set(&CK::Category(id), &category);
        env.storage()
            .persistent()
            .set(&CK::IdByNormalizedName(normalized.clone()), &id);
        Self::extend_name_key_ttl(env, &normalized);
        env.storage()
            .persistent()
            .set(&CK::CategoryList, &categories);
        if parent_id.is_some() {
            env.storage().persistent().set(&CK::Parent(id), &parent_id);
        }
        // Backfill projects that used this free-form label before the category
        // was introduced so switching to managed categories loses no stats.
        let mut stats = Self::empty_stats();
        let existing_projects: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::CategoryProjects(name.clone()))
            .unwrap_or_else(|| Vec::new(env));
        for project_id in existing_projects.iter() {
            if let Some(project) = ProjectRegistry::get_project(env, project_id) {
                stats.direct_project_count = stats.direct_project_count.saturating_add(1);
                if !project.archived {
                    stats.direct_active_project_count =
                        stats.direct_active_project_count.saturating_add(1);
                }
            }
        }
        env.storage().persistent().set(&CK::Stats(id), &stats);
        StorageManager::extend_project_category_ttl(env, id);
        StorageManager::extend_project_category_global_ttl(env);
        publish_project_category_created_event(env, &category, admin);
        Ok(category)
    }

    pub fn update_category(
        env: &Env,
        admin: Address,
        category_id: u64,
        name: String,
        description: String,
        parent_id: Option<u64>,
        active: bool,
    ) -> Result<ProjectCategory, ContractError> {
        require_admin_auth(env, &admin)?;
        let mut category =
            Self::get_category(env, category_id).ok_or(ContractError::ProjectCategoryNotFound)?;
        let normalized = Self::validate_name(env, &name)?;
        Self::validate_description(&description)?;
        let old_name = category.name.clone();
        let old_normalized = Utils::normalize_project_name(env, &old_name);
        let old_parent = category.parent_id;
        Self::validate_parent(env, Some(category_id), parent_id)?;

        let duplicate_id: Option<u64> = env
            .storage()
            .persistent()
            .get(&CK::IdByNormalizedName(normalized.clone()));
        if duplicate_id.is_some() && duplicate_id != Some(category_id) {
            return Err(ContractError::ProjectCategoryAlreadyExists);
        }

        let stats: ProjectCategoryStats = env
            .storage()
            .persistent()
            .get(&CK::Stats(category_id))
            .unwrap_or_else(Self::empty_stats);
        if !active
            && (stats.direct_project_count > 0 || !Self::child_ids(env, category_id).is_empty())
        {
            return Err(ContractError::ProjectCategoryHasProjects);
        }
        if active && parent_id.is_some() {
            let parent = Self::get_category(env, parent_id.unwrap())
                .ok_or(ContractError::ProjectCategoryNotFound)?;
            if !parent.active {
                return Err(ContractError::ProjectCategoryInactive);
            }
        }

        if name != old_name {
            let project_ids: Vec<u64> = env
                .storage()
                .persistent()
                .get(&StorageKey::CategoryProjects(old_name.clone()))
                .unwrap_or_else(|| Vec::new(env));
            // Install the new lookup before project hooks resolve the destination.
            // If only casing changed, both names intentionally share one alias.
            env.storage()
                .persistent()
                .set(&CK::IdByNormalizedName(normalized.clone()), &category_id);
            Self::extend_name_key_ttl(env, &normalized);
            for project_id in project_ids.iter() {
                ProjectRegistry::admin_migrate_category(
                    env,
                    project_id,
                    name.clone(),
                    admin.clone(),
                )?;
            }
            if normalized != old_normalized {
                env.storage()
                    .persistent()
                    .remove(&CK::IdByNormalizedName(old_normalized));
            }
        }

        if old_parent != parent_id {
            if let Some(old_parent) = old_parent {
                Self::remove_child(env, old_parent, category_id);
            }
            if let Some(parent_id) = parent_id {
                Self::append_child(env, parent_id, category_id)?;
            }
            if parent_id.is_some() {
                env.storage()
                    .persistent()
                    .set(&CK::Parent(category_id), &parent_id);
            } else {
                env.storage().persistent().remove(&CK::Parent(category_id));
            }
        }

        category.name = name;
        category.description = description;
        category.parent_id = parent_id;
        category.active = active;
        category.updated_at = env.ledger().timestamp();
        env.storage()
            .persistent()
            .set(&CK::Category(category_id), &category);
        StorageManager::extend_project_category_ttl(env, category_id);
        publish_project_category_updated_event(env, &category, admin);
        Ok(category)
    }

    pub fn delete_category(
        env: &Env,
        admin: Address,
        category_id: u64,
        migrate_to: Option<u64>,
    ) -> Result<ProjectCategoryMigrationResult, ContractError> {
        require_admin_auth(env, &admin)?;
        let category =
            Self::get_category(env, category_id).ok_or(ContractError::ProjectCategoryNotFound)?;
        let children = Self::child_ids(env, category_id);
        if let Some(target_id) = migrate_to {
            let target =
                Self::get_category(env, target_id).ok_or(ContractError::ProjectCategoryNotFound)?;
            if !target.active {
                return Err(ContractError::ProjectCategoryInactive);
            }
            if target_id == category_id || Self::is_descendant(env, target_id, category_id) {
                return Err(ContractError::ProjectCategoryHierarchyInvalid);
            }
            let target_depth = Self::depth(env, target_id)
                .ok_or(ContractError::ProjectCategoryHierarchyInvalid)?;
            for child_id in children.iter() {
                if target_depth.saturating_add(Self::subtree_height(env, child_id, 1))
                    > MAX_CATEGORY_HIERARCHY_DEPTH
                {
                    return Err(ContractError::ProjectCategoryHierarchyInvalid);
                }
            }
            Self::ensure_child_capacity(env, target_id, children.len() as u32)?;
        }

        let project_ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::CategoryProjects(category.name.clone()))
            .unwrap_or_else(|| Vec::new(env));
        if !project_ids.is_empty() && migrate_to.is_none() {
            return Err(ContractError::ProjectCategoryHasProjects);
        }
        let target_name = match migrate_to {
            Some(target_id) => Some(
                Self::get_category(env, target_id)
                    .ok_or(ContractError::ProjectCategoryNotFound)?
                    .name,
            ),
            None => None,
        };
        let mut migrated_project_count = 0u64;
        if let Some(target_name) = target_name {
            for project_id in project_ids.iter() {
                ProjectRegistry::admin_migrate_category(
                    env,
                    project_id,
                    target_name.clone(),
                    admin.clone(),
                )?;
                migrated_project_count = migrated_project_count.saturating_add(1);
            }
        }

        let mut reparented_child_count = 0u32;
        for child_id in children.iter() {
            if let Some(mut child) = Self::get_category(env, child_id) {
                child.parent_id = migrate_to;
                child.updated_at = env.ledger().timestamp();
                env.storage()
                    .persistent()
                    .set(&CK::Category(child_id), &child);
                if migrate_to.is_some() {
                    env.storage()
                        .persistent()
                        .set(&CK::Parent(child_id), &migrate_to);
                } else {
                    env.storage().persistent().remove(&CK::Parent(child_id));
                }
                if let Some(target_id) = migrate_to {
                    Self::append_child(env, target_id, child_id)?;
                }
                StorageManager::extend_project_category_ttl(env, child_id);
                reparented_child_count = reparented_child_count.saturating_add(1);
            }
        }
        if let Some(parent_id) = category.parent_id {
            Self::remove_child(env, parent_id, category_id);
        }

        let mut categories: Vec<u64> = env
            .storage()
            .persistent()
            .get(&CK::CategoryList)
            .unwrap_or_else(|| Vec::new(env));
        let mut remaining = Vec::new(env);
        for index in 0..categories.len() {
            if let Some(id) = categories.get(index) {
                if id != category_id {
                    remaining.push_back(id);
                }
            }
        }
        env.storage()
            .persistent()
            .set(&CK::CategoryList, &remaining);
        let normalized = Utils::normalize_project_name(env, &category.name);
        env.storage()
            .persistent()
            .remove(&CK::IdByNormalizedName(normalized));
        env.storage()
            .persistent()
            .remove(&CK::Category(category_id));
        env.storage().persistent().remove(&CK::Stats(category_id));
        env.storage()
            .persistent()
            .remove(&CK::Children(category_id));
        env.storage().persistent().remove(&CK::Parent(category_id));
        StorageManager::extend_project_category_global_ttl(env);

        let result = ProjectCategoryMigrationResult {
            deleted_category_id: category_id,
            migrated_project_count,
            reparented_child_count,
            migration_target_id: migrate_to,
        };
        publish_project_category_deleted_event(env, &result, admin);
        Ok(result)
    }

    pub fn list_categories(env: &Env, start_index: u32, limit: u32) -> Vec<ProjectCategory> {
        let effective_limit = if limit == 0 || limit > MAX_PAGE_LIMIT {
            MAX_PAGE_LIMIT
        } else {
            limit
        };
        let ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&CK::CategoryList)
            .unwrap_or_else(|| Vec::new(env));
        if start_index >= ids.len() {
            return Vec::new(env);
        }
        let end = core::cmp::min(start_index.saturating_add(effective_limit), ids.len());
        let mut page = Vec::new(env);
        for index in start_index..end {
            if let Some(id) = ids.get(index) {
                if let Some(category) = Self::get_category(env, id) {
                    page.push_back(category);
                }
            }
        }
        page
    }

    pub fn list_children(env: &Env, category_id: u64) -> Vec<ProjectCategory> {
        if Self::get_category(env, category_id).is_none() {
            return Vec::new(env);
        }
        let children = Self::child_ids(env, category_id);
        let mut page = Vec::new(env);
        for child_id in children.iter() {
            if let Some(child) = Self::get_category(env, child_id) {
                page.push_back(child);
            }
        }
        page
    }

    fn hierarchy_totals(env: &Env, category_id: u64, depth: u32) -> (u64, u64) {
        if depth >= MAX_CATEGORY_HIERARCHY_DEPTH {
            return (0, 0);
        }
        let stats: ProjectCategoryStats = env
            .storage()
            .persistent()
            .get(&CK::Stats(category_id))
            .unwrap_or_else(Self::empty_stats);
        let mut projects = stats.direct_project_count;
        let mut active = stats.direct_active_project_count;
        for child_id in Self::child_ids(env, category_id).iter() {
            let (child_projects, child_active) = Self::hierarchy_totals(env, child_id, depth + 1);
            projects = projects.saturating_add(child_projects);
            active = active.saturating_add(child_active);
        }
        (projects, active)
    }

    pub fn get_stats(env: &Env, category_id: u64) -> Option<ProjectCategoryStats> {
        if Self::get_category(env, category_id).is_none() {
            return None;
        }
        let direct: ProjectCategoryStats = env
            .storage()
            .persistent()
            .get(&CK::Stats(category_id))
            .unwrap_or_else(Self::empty_stats);
        let (hierarchy_projects, hierarchy_active) = Self::hierarchy_totals(env, category_id, 0);
        Some(ProjectCategoryStats {
            direct_project_count: direct.direct_project_count,
            direct_active_project_count: direct.direct_active_project_count,
            direct_child_count: Self::child_ids(env, category_id).len() as u64,
            descendant_project_count: hierarchy_projects
                .saturating_sub(direct.direct_project_count),
            hierarchy_active_project_count: hierarchy_active,
        })
    }

    pub fn on_project_assigned(env: &Env, category: &String, archived: bool) {
        if let Some(record) = Self::get_category_by_name(env, category) {
            let mut stats: ProjectCategoryStats = env
                .storage()
                .persistent()
                .get(&CK::Stats(record.id))
                .unwrap_or_else(Self::empty_stats);
            stats.direct_project_count = stats.direct_project_count.saturating_add(1);
            if !archived {
                stats.direct_active_project_count =
                    stats.direct_active_project_count.saturating_add(1);
            }
            env.storage()
                .persistent()
                .set(&CK::Stats(record.id), &stats);
            StorageManager::extend_project_category_ttl(env, record.id);
        }
    }

    pub fn on_project_unassigned(env: &Env, category: &String, was_archived: bool) {
        if let Some(record) = Self::get_category_by_name(env, category) {
            let mut stats: ProjectCategoryStats = env
                .storage()
                .persistent()
                .get(&CK::Stats(record.id))
                .unwrap_or_else(Self::empty_stats);
            stats.direct_project_count = stats.direct_project_count.saturating_sub(1);
            if !was_archived {
                stats.direct_active_project_count =
                    stats.direct_active_project_count.saturating_sub(1);
            }
            env.storage()
                .persistent()
                .set(&CK::Stats(record.id), &stats);
            StorageManager::extend_project_category_ttl(env, record.id);
        }
    }

    pub fn on_project_active_changed(env: &Env, category: &String, active: bool) {
        if let Some(record) = Self::get_category_by_name(env, category) {
            let mut stats: ProjectCategoryStats = env
                .storage()
                .persistent()
                .get(&CK::Stats(record.id))
                .unwrap_or_else(Self::empty_stats);
            if active {
                stats.direct_active_project_count =
                    stats.direct_active_project_count.saturating_add(1);
            } else {
                stats.direct_active_project_count =
                    stats.direct_active_project_count.saturating_sub(1);
            }
            env.storage()
                .persistent()
                .set(&CK::Stats(record.id), &stats);
            StorageManager::extend_project_category_ttl(env, record.id);
        }
    }
}
