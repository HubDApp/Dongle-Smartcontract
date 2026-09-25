//! Public contract types for dynamic project categories (issue #752).

use soroban_sdk::{contracttype, String};

/// Admin-managed category with an optional parent category.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectCategory {
    pub id: u64,
    pub name: String,
    pub description: String,
    pub parent_id: Option<u64>,
    pub active: bool,
    pub created_at: u64,
    pub updated_at: u64,
}

/// Persisted direct counts plus recursively aggregated hierarchy counts.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectCategoryStats {
    pub direct_project_count: u64,
    pub direct_active_project_count: u64,
    pub direct_child_count: u64,
    pub descendant_project_count: u64,
    pub hierarchy_active_project_count: u64,
}

/// Result of deleting a category while preserving projects and children.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectCategoryMigrationResult {
    pub deleted_category_id: u64,
    pub migrated_project_count: u64,
    pub reparented_child_count: u32,
    pub migration_target_id: Option<u64>,
}
