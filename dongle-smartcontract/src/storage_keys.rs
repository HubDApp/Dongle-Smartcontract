//! Storage key types for persistent storage. Modular to allow future extensions.
//!
//! ## Key namespace design (closes #665)
//!
//! Soroban `#[contracttype]` enums are XDR-serialised as a tagged union of the
//! form `(variant_ordinal, payload)`.  The Soroban SDK caps a single union at
//! **50 variants** — attempting to compile a 51st case panics the macro.
//!
//! ### StorageKey (first 50 variants)
//!
//! `StorageKey` holds the original, core set of storage keys.  It currently
//! has exactly 50 variants (ordinals 0–49).  Adding more variants to this enum
//! would push it over the cap and break the build.
//!
//! ### ExtensionKey (overflow — independent namespace)
//!
//! When `StorageKey` reached 50 variants, all new keys were added to
//! `ExtensionKey`.  Because `ExtensionKey` is a *different* XDR union type,
//! its ordinals are **entirely independent** of `StorageKey`'s ordinals.
//! There is **no cross-enum collision**: `StorageKey::Project(0)` and
//! `ExtensionKey::ClaimRequest(0)` serialise to different byte sequences and
//! never share a ledger entry.
//!
//! The only soundness requirement is that the two names used in the *same*
//! enum must be unique — the Rust compiler enforces this.
//!
//! ### Capacity and the 50-variant limit
//!
//! `ExtensionKey` follows the same 50-variant cap.  Its current variant count
//! is tracked by `tests::storage_key_uniqueness`.  When `ExtensionKey`
//! approaches 45 variants (the warning threshold) a third enum
//! (`ExtensionKey2`) must be introduced following the same pattern.
//!
//! The warning threshold test (`extension_key_variant_count_below_warn_threshold`)
//! will fail loudly before the limit is reached.
//!
//! ### Performance
//!
//! Key lookup is O(1) — the key is XDR-serialised once per call and handed
//! directly to the host storage map.  The two-enum split adds zero runtime
//! overhead.

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
    /// Project by canonical lowercase slug (for URL lookups and uniqueness).
    /// The key is normalized to lowercase so `Alpha` and `alpha` resolve to the
    /// same unique storage entry and duplicate detection remains consistent.
    ProjectBySlug(String),
    /// Project lifecycle status by project ID.
    ProjectLifecycleStatus(u64),
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
    /// Global pause flag (admin-controlled). Read by `get_config`.
    ContractPaused,
    /// Admin-configured duration (in seconds) a verification stays active.
    VerificationDuration,
    /// List of non-archived project IDs registered by owner.
    ActiveOwnerProjects(Address),
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
    /// Changelog: next changelog entry ID counter.
    NextChangelogEntryId,
    /// Changelog: entry by ID.
    ProjectChangelogEntry(u64),
    /// Changelog: list of changelog entry IDs for a project.
    ProjectChangelogEntries(u64),
    /// Project endorsements: list of addresses that endorsed a project.
    ProjectEndorsements(u64),
    /// Endorser at a zero-based project position.
    EndorsementAt(u64, u32),
    /// Zero-based position of an endorser in a project's index.
    EndorsementIndex(u64, Address),
    /// Endorsement count for a project.
    EndorsementCount(u64),
    /// Tombstone for a deleted review (project_id, reviewer). Allows indexers to distinguish deleted vs never-existed.
    ReviewTombstone(u64, Address),
    /// Timestamp of the last successful update for a review (project_id, reviewer). Used for cooldown enforcement.
    ReviewLastUpdated(u64, Address),
    /// Fee payment details for a project (payer, amount, token, timestamp).
    FeePaymentDetails(u64),
    /// Fee payment details for a registration (payer, amount, token, timestamp).
    RegistrationFeePaymentDetails(Address),
    /// Claimable refund owed after a rejected verification (issue #472).
    FeeRefund(u64),
    /// List of reserved project names (admin-managed).
    ReservedNames,
    /// Optional region/market metadata for a project.
    ProjectRegion(u64),
    /// Integrity hash of key project metadata fields.
    ProjectIntegrityHash(u64),
    /// Normalized project name index (lowercase, collapsed whitespace, no punctuation) -> project_id.
    /// Used for case/whitespace/punctuation-insensitive duplicate detection.
    ///
    /// Declared here rather than in `StorageKey`: Soroban caps a `#[contracttype]`
    /// union at 50 cases and `StorageKey` was at 51, which panics the macro. The
    /// key encoding is the variant name plus payload and is identical either way,
    /// so relocating it needs no storage migration. An unused duplicate of this
    /// variant already existed here.
    ProjectByNormalizedName(String),
    /// Inverted tag index: tag -> project ids carrying it (issue #483).
    ///
    /// Declared here rather than in `StorageKey` for the reason recorded on
    /// `ProjectByNormalizedName` above: Soroban caps a `#[contracttype]` union at
    /// 50 cases and `StorageKey` is already at exactly 50. The issue suggested
    /// `StorageKey::TagProjects`, which cannot compile.
    TagProjects(String),
    /// Watermark for the tag index: every project id `<= n` is represented in
    /// `TagProjects` (issue #483).
    ///
    /// Projects registered before the index existed are not in it, and an empty
    /// index entry is indistinguishable from "no project has this tag". The
    /// watermark makes the covered range explicit, so a lookup can serve indexed
    /// ids directly and scan only the uncovered tail. `reindex_tags` advances it.
    TagIndexWatermark,
    ContractClaim(u64, String),
    ProjectContracts(u64),
    ReviewEligibilityConfig,
    FirstInteraction(Address),
    ReviewRevisionCount(u64, Address),
    ReviewRevision(u64, Address, u32),
    /// Per-admin log index: list of action log IDs authored by a specific admin.
    AdminActionLogByAdmin(Address),
    /// Global index of pending verification request IDs, in creation order.
    PendingVerificationRequests,
    /// Fee configuration change history, appended oldest-first.
    ///
    /// Stored as a single `Vec<FeeConfigHistoryEntry>` rather than one key per
    /// entry because `ExtensionKey` is a `#[contracttype]` union and Soroban
    /// caps those at 50 cases; a per-entry key plus a separate count key would
    /// need two slots and push the enum over the limit.
    FeeConfigHistory,
    /// Verification suspension timeline for a project, oldest-first.
    ProjectVerificationSuspensions(u64),
}

/// Third overflow storage key enum, introduced because `ExtensionKey` has reached the
/// 50-variant Soroban `#[contracttype]` hard cap and cannot accept any further variants.
///
/// `ExtensionKey2` follows the exact same design rules as `ExtensionKey`:
///
/// - Ordinals are **entirely independent** of both `StorageKey` and `ExtensionKey`
///   because this is a different XDR union type.  There is no cross-enum collision.
/// - The only soundness requirement is that variant names within *this* enum are unique —
///   the Rust compiler enforces this.
/// - This enum is also subject to the 50-variant Soroban cap.  Its current variant count
///   is tracked by `tests::storage_key_uniqueness`.  When it approaches 45 variants
///   (the warning threshold), a fourth enum (`ExtensionKey3`) must be introduced.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExtensionKey2 {
    /// Evidence links for a review, keyed by (project_id, reviewer).
    ReviewEvidenceLinks(u64, Address),
    /// Archived review record, keyed by (project_id, reviewer).
    /// Stored at a shorter TTL than active reviews after `archive_old_reviews`
    /// moves eligible reviews out of primary storage.
    ArchivedReview(u64, Address),
    /// Index of reviewer addresses whose reviews have been archived for a project.
    /// Enables paginated enumeration of all archived reviews for a given project.
    ProjectArchivedReviews(u64),
    /// Append-only evidence CID versions for a verification request.
    VerificationEvidenceVersions(u64),
    /// Scheduled deprecation, sunset, alternatives, and redirect for a project.
    ProjectSunsetPlan(u64),
    /// Risk assessment captured for a verification request.
    VerificationRiskAssessment(u64),
    /// Verification request IDs currently flagged for additional review.
    HighRiskVerificationRequests,
    /// Current coefficients and threshold for the verification risk model.
    VerificationRiskModel,
    /// Full appeal history for a rejected verification request.
    VerificationAppeals(u64),
    /// Active rejection metadata for a project to enforce the per-rejection appeal cap.
    VerificationRejection(u64),
}

/// Storage keys for fee configuration history, split into a separate enum to stay under
/// Soroban's 50-variant limit per `#[contracttype]` enum.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FeeHistoryKey {
    /// Counter for fee configuration change history entries.
    FeeConfigHistoryCount,
    /// Fee configuration history entry by index.
    FeeConfigHistoryEntry(u32),
    /// Configurable maximum number of reviews allowed per project.
    MaxReviewsPerProject,
}

/// Storage keys for notification preferences and digest queues (#811).
///
/// `ExtensionKey` is at its 50-variant Soroban cap; new notification
/// keys use this independent enum following the same pattern as
/// `FeeHistoryKey`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NotificationKey {
    /// Global notification preferences for a user.
    UserNotificationPrefs(Address),
    /// Per-project notification override for a user.
    /// Keyed by `(user_address, project_id)`.
    UserProjectNotifOverride(Address, u64),
    /// Queue of project IDs with pending updates awaiting digest delivery.
    /// Cleared after a digest is emitted.
    UserDigestQueue(Address),
    /// Owner-facing verification expiry reminder state by project.
    VerificationExpiryNotification(u64),
}

/// Storage keys for review content integrity seals (#809).
///
/// `ExtensionKey` is at its 50-variant Soroban cap; review integrity
/// keys use this independent enum following the same pattern as
/// `FeeHistoryKey` and `NotificationKey`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReviewIntegrityKey {
    /// SHA-256 integrity hash stored for a review at write time.
    /// Keyed by `(project_id, reviewer)`.
    ReviewIntegrityHash(u64, Address),
}

/// Storage keys for bookmark folders and smart folders (#815).
///
/// `ExtensionKey` is at its 50-variant Soroban cap.  Bookmark-folder keys use
/// this independent enum following the same pattern as `FeeHistoryKey`,
/// `NotificationKey`, and `ReviewIntegrityKey`.
///
/// All keys are **per-user**: the `Address` payload is the folder owner.
/// Folder IDs are monotonically increasing counters scoped to each user.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BookmarkKey {
    /// Folder record for (owner, folder_id).
    BookmarkFolder(Address, u64),
    /// List of folder IDs owned by a user.
    UserFolderIds(Address),
    /// Next folder ID counter for a user (scoped per-user).
    NextFolderIdForUser(Address),
    /// Bookmarks inside a folder: list of project IDs in (owner, folder_id).
    FolderBookmarks(Address, u64),
    /// Smart folder record for (owner, smart_folder_id).
    SmartFolder(Address, u64),
    /// List of smart folder IDs owned by a user.
    UserSmartFolderIds(Address),
    /// Next smart folder ID counter for a user (scoped per-user).
    NextSmartFolderIdForUser(Address),
    /// Per-user index: project_id → folder_id.  Lets `move_bookmark` find
    /// the current folder of a project without scanning all folder lists.
    BookmarkFolderIndex(Address, u64),
}

/// Storage keys for featured algorithm configuration, A/B testing, and metrics (#816).
///
/// `ExtensionKey` is at its 50-variant Soroban cap. Featured algorithm keys use
/// this independent enum following the same pattern as `BookmarkKey`, `FeeHistoryKey`,
/// `NotificationKey`, and `ReviewIntegrityKey`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FeaturedAlgorithmKey {
    /// Active algorithm weights for quality-based featured selection.
    QualityWeights,
    /// Active algorithm weights for trending featured selection.
    TrendingWeights,
    /// Active algorithm weights for high-rating featured selection.
    HighRatingWeights,
    /// Active A/B test configuration.
    ABTestConfig,
    /// Metric tracking: impressions/query count for Variant A.
    MetricVariantACount,
    /// Metric tracking: impressions/query count for Variant B.
    MetricVariantBCount,
    /// Timestamp when featured projects list was last automatically generated.
    LastAutoGeneratedTimestamp,
}

/// Storage keys for recommendations, personalization, and A/B analytics (#749).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RecommendationKey {
    Recommendation(u64),
    NextRecommendationId,
    RecommendationList,
    RecommendationsForProject(u64),
    RecommendationsByAlgorithm(u32),
    ImpressionSeen(u64, Address),
    ClickSeen(u64, Address),
    Converted(u64, Address),
    ImpressionCount(u64),
    ClickCount(u64),
    ConversionCount(u64),
    FollowCount(u64),
    BookmarkCount(u64),
    EndorseCount(u64),
    ReviewCount(u64),
    HelpfulCount(u64),
    NotHelpfulCount(u64),
    Feedback(u64, Address),
    ViewHistory(Address),
    PersonalizedRecommendation(Address, u64, u64, u32),
    ABConfig,
    VariantAAnalytics,
    VariantBAnalytics,
}
