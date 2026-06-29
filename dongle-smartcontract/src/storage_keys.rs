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
    /// Project by slug (for URL lookups).
    ProjectBySlug(String),
    /// Project count.
    ProjectCount,
    /// Review by (project_id, reviewer address).
    Review(u64, Address),
    /// Verification record by project_id.
    Verification(u64),
    /// Next verification request id (counter).
    NextVerificationRequestId,
    /// Verification record by request_id.
    VerificationRecord(u64),
    /// Project verification history: list of verification request IDs.
    ProjectVerificationHistory(u64),
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
    ProjectLaunchTimestamp(u64),
    /// Project bounty URL by project ID.
    ProjectBountyUrl(u64),
    /// Project social links by project ID.
    ProjectSocialLinks(u64),
    /// Project maintainers by project ID.
    ProjectMaintainers(u64),
    /// Linked project IDs for a project.
    ProjectLinkedProjects(u64),
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
    /// Whether reviews are enabled for a project (true = enabled, absent = enabled by default).
    ReviewsEnabled(u64),
    /// Review report tracking: (project_id, reviewer_address, reporter_address) -> bool
    ReviewReport(u64, Address, Address),
    /// Verification renewal request by project_id
    VerificationRenewal(u64),
    /// Verification renewal history: (project_id, renewal_index) -> VerificationRenewalRecord
    VerificationRenewalHistory(u64, u32),
    /// Renewal count for a project (tracks number of renewals)
    VerificationRenewalCount(u64),
    /// List of featured project IDs.
    FeaturedProjects,
    /// Collection by id.
    Collection(u64),
    /// Collection name string by id (for uniqueness checks).
    CollectionNameById(u64),
    /// Next collection id (auto-increment counter).
    NextCollectionId,
    /// List of all collection IDs.
    CollectionList,
    /// Project IDs belonging to a collection.
    CollectionProjectIds(u64),
    /// Admin action log entry by sequential ID.
    AdminActionLog(u64),
    /// Next admin action log ID (auto-increment counter).
    AdminActionLogCount,
}

/// Additional storage keys for new features to stay under the 50-variant limit of StorageKey.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExtensionKey {
    ClaimRequest(u64),
    ClaimReqProjClaimant(u64, Address),
    ProjectClaimRequests(u64),
    NextClaimRequestId,
    ProjectDependency(u64, String),
    ProjectDependencyKeys(u64),
    DuplicateDispute(u64),
    ProjectDuplicateDisputes(u64),
    NextDuplicateDisputeId,
    VerificationDuration,
    ProjectFollowers(u64),
    UserSubscriptions(Address),
    FollowerCount(u64),
    /// Admin Timelock: scheduled action by ID.
    TimelockAction(u64),
    /// Admin Timelock: list of all scheduled action IDs.
    TimelockActionIds,
    /// Admin Timelock: next action ID counter.
    NextTimelockActionId,
    /// Admin Timelock: fee change params keyed by action ID.
    TimelockFeeParams(u64),
    /// Admin Timelock: admin add params keyed by action ID.
    TimelockAdminAddParams(u64),
    /// Admin Timelock: admin remove params keyed by action ID.
    TimelockAdminRemoveParams(u64),
    /// User bookmarks: list of project IDs bookmarked by a user.
    UserBookmarks(Address),
    /// Admin governance: approval threshold.
    AdminApprovalThreshold,
    /// Admin governance: next proposal ID counter.
    NextAdminProposalId,
    /// Admin governance: proposal by ID.
    AdminProposal(u64),
    /// Admin governance: list of all proposal IDs.
    AdminProposalIds,
    /// Project endorsements: list of addresses that endorsed a project.
    ProjectEndorsements(u64),
    /// Endorsement count for a project.
    EndorsementCount(u64),
    /// Fee refund record keyed by verification request_id.
    FeeRefundRecord(u64),
    /// Fee config history entry count.
    FeeConfigHistoryCount,
    /// Fee config history entry by index (oldest = 0).
    FeeConfigHistoryEntry(u32),
    /// Tombstone for a deleted review (project_id, reviewer). Allows indexers to distinguish deleted vs never-existed.
    ReviewTombstone(u64, Address),
    /// Timestamp of the last successful update for a review (project_id, reviewer). Used for cooldown enforcement.
    ReviewLastUpdated(u64, Address),
    /// Fee payment details for a project (payer, amount, token, timestamp).
    FeePaymentDetails(u64),
    /// Fee payment details for a registration (payer, amount, token, timestamp).
    RegistrationFeePaymentDetails(Address),
    /// List of reserved project names (admin-managed).
    ReservedNames,
    /// Optional region/market metadata for a project.
    ProjectRegion(u64),
    /// Integrity hash of key project metadata fields.
    ProjectIntegrityHash(u64),
}
