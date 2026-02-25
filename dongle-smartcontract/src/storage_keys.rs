//! Storage key types for persistent storage. Modular to allow future extensions.

use soroban_sdk::{contracttype, Address, String};

/// Keys for contract storage. Using an enum keeps keys namespaced and avoids collisions.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StorageKey {
    /// Project by id.
    Project(u64),
    /// Next project id (counter).
    NextProjectId,
    /// Number of projects registered by owner (Address).
    OwnerProjectCount(Address),
    /// Project stats (ratings, etc).
    ProjectStats(u64),
    /// List of project IDs registered by owner.
    OwnerProjects(Address),
    /// Project by name (for duplicate detection).
    ProjectByName(String),
    /// Project count.
    ProjectCount,
    /// Review by (project_id, reviewer address).
    Review(u64, Address),
    /// Verification record by project_id.
    Verification(u64),
    /// Fee configuration (single global).
    FeeConfig,
    /// Whether verification fee has been paid for project_id.
    FeePaidForProject(u64),
    /// Admin address (for fee set and verifier checks).
    Admin,
    /// List of project IDs reviewed by a user.
    UserReviews(Address),
    /// Treasury address.
    Treasury,
    /// List of reviewer addresses for a project (by project_id).
    ProjectReviews(u64),
}
