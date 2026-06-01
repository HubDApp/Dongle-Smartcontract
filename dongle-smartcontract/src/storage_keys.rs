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
    /// Whether registration fee has been paid for address.
    RegistrationFeePaidForAddress(Address),
    /// Admin address mapping (for role-based access control).
    Admin(soroban_sdk::Address),
    /// List of all admin addresses.
    AdminList,
    /// Minimum project age configuration for verification.
    MinProjectAge,
    /// Project tags by project ID.
    ProjectTags(u64),
    /// Project social links by project ID.
    ProjectSocialLinks(u64),
    /// Project reports by project ID.
    ProjectReports(u64),
    /// Report count for a project.
    ProjectReportCount(u64),
    /// User report tracking (project_id, reporter).
    UserReport(u64, Address),
    /// List of project IDs reviewed by a user.
    UserReviews(Address),
    /// Treasury address.
    Treasury,
    /// List of reviewer addresses for a project (by project_id).
    ProjectReviews(u64),
    /// Pending ownership transfer recipient for a project.
    PendingTransfer(u64),
    /// List of project IDs by category.
    CategoryProjects(String),
}
